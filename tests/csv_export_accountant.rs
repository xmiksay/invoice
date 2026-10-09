//! Accountant export (`GET /api/export/accountant`): tax-date period,
//! direction, proformas / drafts / cancelled excluded, period validation.

mod common;

use axum::http::StatusCode;
use common::csv_export::{export, export_error, fixture, get_raw, import_all};
use common::documents::{create_bank, create_contact, create_doc, create_issued, id, item};
use common::documents::{post_action, set_company};
use common::received::{create_category, create_received};
use common::{TestDb, router};
use sea_orm::ConnectionTrait;
use serde_json::json;

const JANUARY: &str = "/api/export/accountant?from=2026-01-01&to=2026-01-31";

#[tokio::test]
async fn period_direction_and_exclusions() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    set_company(&app, true).await;
    create_bank(&app, "CZK").await;
    create_category(&app, "Software", "expense").await;
    let contact = create_contact(&app, json!({ "name": "Nativní", "ico": "12345679" })).await;
    let supplier = create_contact(&app, json!({ "name": "Dodavatel", "ico": "87654326" })).await;
    import_all(&app, &fixture()).await;
    // A January draft and a cancelled January invoice never show up.
    let body = json!({ "contactId": contact, "issueDate": "2026-01-20",
                       "taxPointDate": "2026-01-20", "lines": [item("1", "5", "21")] });
    create_doc(&app, body.clone()).await;
    let cancelled = id(&create_issued(&app, body).await);
    let (status, c) = post_action(&app, &cancelled, "cancel").await;
    assert_eq!(status, StatusCode::OK, "{c}");

    let (status, headers, _) = get_raw(&app, JANUARY).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers["content-disposition"],
        "attachment; filename=\"ucetni-2026-01-01-2026-01-31.csv\""
    );
    let csv = export(&app, JANUARY).await;
    assert_eq!(csv.column("direction"), ["issued", "issued", "received"]);
    assert_eq!(csv.column("number")[..2], ["FA-3", "FA-1"], "no proforma");
    assert_eq!(csv.cell(2, "supplier_number"), "FV-42");

    let received = export(&app, &format!("{JANUARY}&direction=received")).await;
    assert_eq!(received.column("supplier_number"), ["FV-42"]);
    let issued = export(&app, &format!("{JANUARY}&direction=issued")).await;
    assert_eq!(issued.column("number"), ["FA-3", "FA-1"]);
    let both = export(&app, &format!("{JANUARY}&direction=both")).await;
    assert_eq!(both.rows.len(), 3);

    // February: the credit note by its tax date; the ends are inclusive.
    let feb = export(&app, "/api/export/accountant?from=2026-02-02&to=2026-02-02").await;
    assert_eq!(feb.column("number"), ["DB-1"]);
    assert_eq!(feb.cell(0, "total"), "-121,00");

    // A received document without a tax date counts by its received date.
    create_received(
        &app,
        &supplier,
        json!({ "supplierNumber": "FV-77", "issueDate": "2026-01-30",
                "taxPointDate": "2026-01-30", "receivedDate": "2026-02-03" }),
    )
    .await;
    db.conn
        .execute_unprepared(
            "UPDATE documents SET tax_point_date = NULL WHERE supplier_number = 'FV-77'",
        )
        .await
        .expect("drop the tax date");
    let feb = export(&app, "/api/export/accountant?from=2026-02-01&to=2026-02-28").await;
    assert_eq!(feb.column("supplier_number"), ["", "FV-77"]);
    assert_eq!(feb.cell(1, "tax_date"), "");
    assert_eq!(export(&app, JANUARY).await.rows.len(), 3);

    let empty = export(&app, "/api/export/accountant?from=2025-01-01&to=2025-12-31").await;
    assert!(empty.rows.is_empty(), "header-only file");
}

#[tokio::test]
async fn invalid_period() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    for (query, fields) in [
        ("", &["from", "to"][..]),
        ("from=2026-01-01", &["to"]),
        ("from=01.01.2026&to=2026-01-31", &["from"]),
        ("from=2026-02-01&to=2026-01-31", &["from", "to"]),
        ("from=2026-01-01&to=2027-01-03", &["from", "to"]),
        (
            "from=2026-01-01&to=2026-01-31&direction=all",
            &["direction"],
        ),
    ] {
        let (status, body) = export_error(&app, &format!("/api/export/accountant?{query}")).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{query}");
        let got = body["fields"].as_object().expect("fields");
        assert_eq!(got.len(), fields.len(), "{query}: {body}");
        for f in fields {
            assert_eq!(got[*f], "invalid", "{query}: {body}");
        }
    }
    // 366 days apart is still fine (a leap-year span).
    let ok = export(&app, "/api/export/accountant?from=2027-03-01&to=2028-03-01").await;
    assert!(ok.rows.is_empty());
}
