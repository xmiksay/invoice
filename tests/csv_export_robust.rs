//! CSV export robustness: one snapshot per export, bad documents skipped,
//! formula cells guarded (and unguarded again by the import, CSV and XLSX).

mod common;

use axum::http::{Method, StatusCode};
use common::csv_export::{export, export_bytes, fixture, import_all, parse};
use common::csvio::preview;
use common::documents::{create_bank, create_contact, create_issued, get_doc, item};
use common::documents::{post_action, set_company};
use common::received::create_category;
use common::{TEST_TOKEN, TestDb, call, get, router, send};
use rust_xlsxwriter::Workbook;
use sea_orm::ConnectionTrait;
use serde_json::json;

const ISSUED: &str = "/api/export/csv?direction=issued";

async fn setup(app: &axum::Router) {
    set_company(app, true).await;
    create_bank(app, "CZK").await;
    create_category(app, "Software", "expense").await;
}

#[tokio::test]
async fn the_export_reads_one_snapshot() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    setup(&app).await;
    import_all(&app, &fixture()).await;

    // The handler has answered (ids taken, header ready); the rows are
    // loaded only while the body is read.
    let resp = send(app.clone(), get(ISSUED, Some(TEST_TOKEN))).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let (status, c) = post_action(&app, &db_id(&db, "DB-1").await, "cancel").await;
    assert_eq!(status, StatusCode::OK, "{c}");
    db.conn
        .execute_unprepared(
            "UPDATE document_vat_recap SET vat_rate = 10 WHERE document_id IN \
             (SELECT id FROM documents WHERE number = 'FA-3')",
        )
        .await
        .expect("change a rate");
    let bytes = axum::body::to_bytes(resp.into_body(), 1 << 24)
        .await
        .expect("read body");
    let csv = parse(&bytes);
    assert_eq!(csv.column("number"), ["FA-3", "ZF-1", "FA-1", "DB-1"]);
    let fa3 = csv.find("number", "FA-3");
    assert_eq!(csv.cell(fa3, "base_21"), "2500,00", "the snapshot's rate");
    assert!(!csv.header.iter().any(|h| h == "base_10"));

    // A new export sees the changes.
    let now = export(&app, ISSUED).await;
    assert_eq!(now.column("number"), ["FA-3", "ZF-1", "FA-1"]);
    assert_eq!(now.cell(0, "base_10"), "2500,00");
}

async fn db_id(db: &TestDb, number: &str) -> String {
    use sea_orm::{FromQueryResult, JsonValue, Statement};
    let row = JsonValue::find_by_statement(Statement::from_sql_and_values(
        sea_orm::DbBackend::Postgres,
        "SELECT id::text AS id FROM documents WHERE number = $1",
        [number.into()],
    ))
    .one(&db.conn)
    .await
    .expect("query")
    .expect("document");
    row["id"].as_str().expect("id").to_string()
}

#[tokio::test]
async fn a_bad_document_is_skipped_not_the_file() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    setup(&app).await;
    import_all(&app, &fixture()).await;
    db.conn
        .execute_unprepared(
            "UPDATE documents SET customer_snapshot = '\"broken\"' WHERE number = 'ZF-1'; \
             UPDATE documents SET exchange_rate = NULL WHERE number = 'FA-3'; \
             UPDATE document_vat_recap SET base_czk = NULL WHERE document_id IN \
             (SELECT id FROM documents WHERE number = 'FA-3')",
        )
        .await
        .expect("corrupt documents");
    let csv = export(&app, ISSUED).await;
    assert_eq!(csv.column("number"), ["FA-1", "DB-1"]);
    let csv = export(&app, "/api/export/accountant?from=2026-01-01&to=2026-12-31").await;
    assert_eq!(csv.column("number")[..2], ["FA-1", "DB-1"]);
}

#[tokio::test]
async fn formula_cells_round_trip() {
    let first = TestDb::new().await;
    let a = router(first.conn.clone());
    setup(&a).await;
    let customer = create_contact(
        &a,
        json!({ "name": "=Fiktivní Odběratel", "ico": "12345679",
                                               "city": "@Brno" }),
    )
    .await;
    create_issued(
        &a,
        json!({ "contactId": customer, "issueDate": "2026-10-01", "headerNote": "-sleva",
                "lines": [item("1", "100", "21")] }),
    )
    .await;
    let bytes = export_bytes(&a, ISSUED).await;
    let csv = parse(&bytes);
    assert_eq!(csv.cell(0, "counterparty_name"), "'=Fiktivní Odběratel");
    assert_eq!(csv.cell(0, "counterparty_city"), "'@Brno");
    assert_eq!(csv.cell(0, "note"), "'-sleva");
    assert_eq!(csv.cell(0, "total"), "121,00");

    let second = TestDb::new().await;
    let b = router(second.conn.clone());
    setup(&b).await;
    let results = import_all(&b, &bytes).await;
    let doc = get_doc(&b, results[0]["documentId"].as_str().expect("id")).await;
    assert_eq!(doc["customer"]["name"], "=Fiktivní Odběratel", "{doc}");
    assert_eq!(doc["headerNote"], "-sleva");
    assert_eq!(export_bytes(&b, ISSUED).await, bytes);
    let (status, c) = call(&b, Method::GET, "/api/contacts?q=Fikt", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(c["items"][0]["name"], "=Fiktivní Odběratel", "{c}");
}

#[tokio::test]
async fn xlsx_text_cells_are_unguarded() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    setup(&app).await;
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    let cells = [
        ("direction", "issued"),
        ("doc_type", "invoice"),
        ("number", "'-7"),
        ("issue_date", "1.3.2026"),
        ("counterparty_name", "'+420 Firma"),
        ("base_0", "100"),
        ("total", "100"),
    ];
    for (col, (h, v)) in (0u16..).zip(cells) {
        ws.write_string(0, col, h).expect("header");
        ws.write_string(1, col, v).expect("cell");
    }
    let bytes = wb.save_to_buffer().expect("xlsx");
    let rows = preview(&app, &bytes).await;
    assert_eq!(rows[0]["status"], "ok", "{}", rows[0]);
    assert_eq!(rows[0]["counterparty"]["name"], "+420 Firma");
    assert_eq!(rows[0]["number"], "-7");
}
