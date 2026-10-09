//! CSV / XLSX import: file-level errors and limits, XLSX, Windows-1250,
//! separators, the sample file (and that it imports back).

mod common;

use axum::http::StatusCode;
use common::csvio::{HEADER, by_key, confirm, doc_id, file, form, post, preview, preview_raw, row};
use common::documents::{get_doc, set_company};
use common::{TestDb, get, router, send};
use rust_xlsxwriter::{ExcelDateTime, Format, Workbook};
use serde_json::{Value, json};

fn received(number: &str) -> String {
    row(&[
        ("direction", "received"),
        ("doc_type", "invoice"),
        ("supplier_number", number),
        ("issue_date", "1.3.2026"),
        ("counterparty_name", "Vzorový Dodavatel a.s."),
        ("counterparty_ico", "87654326"),
        ("base_0", "100"),
        ("total", "100"),
    ])
}

async fn file_error(app: &axum::Router, bytes: &[u8]) -> Value {
    let (status, body) = preview_raw(app, bytes).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    body
}

#[tokio::test]
async fn file_errors_name_the_column() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    set_company(&app, true).await;
    // Received rows need `supplier_number`; issued ones would not.
    let header = HEADER.replace("supplier_number;", "");
    let r = received("FV-1").replacen(";FV-1", "", 1);
    assert_eq!(
        file_error(&app, &file(&header, &[&r])).await,
        json!({ "code": "validation", "fields": { "file": "missing_column" }, "detail": "supplier_number" })
    );
    // A missing column no row needs is fine.
    let rows = preview(
        &app,
        &file(
            "direction;doc_type;number;issue_date;counterparty_name;base_0;total",
            &["issued;invoice;1;1.3.2026;X;10;10"],
        ),
    )
    .await;
    assert_eq!(rows[0]["status"], "ok", "{rows:?}");
    assert_eq!(
        file_error(
            &app,
            &file(&format!("{HEADER};vat_15"), &[&received("FV-1")])
        )
        .await,
        json!({ "code": "validation", "fields": { "file": "invalid_column" }, "detail": "vat_15" })
    );
    assert_eq!(
        file_error(
            &app,
            &file(&format!("{HEADER};BASE_x"), &[&received("FV-1")])
        )
        .await["detail"],
        "base_x"
    );
    let empty = file_error(&app, &file(HEADER, &[";;;", ""])).await;
    assert_eq!(
        empty,
        json!({ "code": "validation", "fields": { "file": "empty" } })
    );
    assert_eq!(file_error(&app, b"").await["fields"]["file"], "empty");
    assert_eq!(
        file_error(&app, b"PK\x03\x04junk").await["fields"]["file"],
        "invalid"
    );

    let many: Vec<String> = (0..501).map(|i| received(&format!("FV-{i}"))).collect();
    let refs: Vec<&str> = many.iter().map(String::as_str).collect();
    let body = file_error(&app, &file(HEADER, &refs)).await;
    assert_eq!(body["fields"]["file"], "too_many");
    assert_eq!(preview(&app, &file(HEADER, &refs[..500])).await.len(), 500);
}

#[tokio::test]
async fn upload_errors() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    set_company(&app, true).await;
    let csv = file(HEADER, &[&received("FV-1")]);
    let (status, body) = post(&app, "/api/import/csv/preview", form(&[], Some("{}"))).await;
    assert_eq!(
        (status, &body["fields"]["file"]),
        (StatusCode::UNPROCESSABLE_ENTITY, &json!("required"))
    );
    let two = form(&[("a.csv", &csv), ("b.csv", &csv)], None);
    let (status, body) = post(&app, "/api/import/csv/preview", two).await;
    assert_eq!(
        (status, &body["fields"]["file"]),
        (StatusCode::UNPROCESSABLE_ENTITY, &json!("invalid"))
    );
    let (status, body) = post(
        &app,
        "/api/import/csv/confirm",
        form(&[("a.csv", &csv)], Some("nope")),
    )
    .await;
    assert_eq!(
        (status, &body["fields"]["options"]),
        (StatusCode::UNPROCESSABLE_ENTITY, &json!("invalid"))
    );
    let big = vec![b'x'; 50 * 1024 * 1024 + 1];
    let (status, body) = post(
        &app,
        "/api/import/csv/preview",
        form(&[("big.csv", &big)], None),
    )
    .await;
    assert_eq!(
        (status, &body["code"]),
        (StatusCode::PAYLOAD_TOO_LARGE, &json!("too_large"))
    );
}

#[tokio::test]
async fn windows_1250_with_commas_and_dots() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    set_company(&app, true).await;
    let text = "Direction,Doc_Type,Supplier_Number,Issue_Date,Counterparty_Name,Base_21,Vat_21,Total\r\n\
                received,invoice,FV-9,2026-03-01,Účetní Dodavatel s.r.o.,\"1,000.50\",210.11,\"1 210,61\"\r\n";
    let (bytes, _, unmappable) = encoding_rs::WINDOWS_1250.encode(text);
    assert!(!unmappable);
    assert!(std::str::from_utf8(&bytes).is_err(), "really not UTF-8");
    let rows = preview(&app, &bytes).await;
    assert_eq!(rows[0]["status"], "ok", "{rows:?}");
    assert_eq!(rows[0]["counterparty"]["name"], "Účetní Dodavatel s.r.o.");
    assert_eq!(rows[0]["total"], "1210.61");
}

fn workbook() -> Vec<u8> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    let date = Format::new().set_num_format("d.m.yyyy");
    let header = [
        "direction",
        "doc_type",
        "number",
        "supplier_number",
        "issue_date",
        "counterparty_name",
        "counterparty_ico",
        "currency",
        "exchange_rate",
        "base_21",
        "vat_21",
        "base_0",
        "total",
    ];
    for (c, h) in (0u16..).zip(header) {
        ws.write_string(0, c, h).expect("header");
    }
    let day = |d: u8| ExcelDateTime::from_ymd(2026, 3, d).expect("date");
    for (r, (dir, n, ico)) in (1u32..).zip([
        ("issued", "2026000100", 12_345_679.0),
        ("received", "XF-7", 87_654_326.0),
    ]) {
        ws.write_string(r, 0, dir).expect("c");
        ws.write_string(r, 1, "invoice").expect("c");
        ws.write_string(r, if dir == "issued" { 2 } else { 3 }, n)
            .expect("c");
        ws.write_datetime_with_format(r, 4, day(2), &date)
            .expect("c");
        ws.write_string(r, 5, "Fiktivní Partner s.r.o.").expect("c");
        ws.write_number(r, 6, ico).expect("c");
    }
    // Issued CZK: amounts as numbers with binary noise.
    for (c, v) in [(9u16, 1234.5), (10, 259.25), (11, 0.1 + 0.2), (12, 1494.05)] {
        ws.write_number(1, c, v).expect("n");
    }
    // Received EUR at a rate, the recap in CZK.
    ws.write_string(2, 7, "EUR").expect("c");
    ws.write_number(2, 8, 24.335).expect("c");
    for (c, v) in [(9u16, 2433.5), (10, 511.04), (12, 121.0)] {
        ws.write_number(2, c, v).expect("n");
    }
    wb.save_to_buffer().expect("xlsx")
}

#[tokio::test]
async fn xlsx_numbers_and_dates_are_exact() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    set_company(&app, true).await;
    let bytes = workbook();
    let rows = preview(&app, &bytes).await;
    for r in &rows {
        assert_eq!(r["status"], "ok", "{r}");
    }
    assert_eq!(rows[0]["issueDate"], "2026-03-02");
    assert_eq!(rows[0]["counterparty"]["ico"], "12345679");
    let results = confirm(&app, &bytes, &["row:2", "row:3"]).await;
    let issued = get_doc(&app, &doc_id(&results, "row:2")).await;
    assert_eq!(issued["totals"]["recap"][0]["base"], "1234.50");
    assert_eq!(
        issued["totals"]["recap"][1],
        json!({ "vatRate": "0", "base": "0.30", "vat": "0.00", "baseCzk": null, "vatCzk": null })
    );
    assert_eq!(issued["totals"]["payable"], "1494.05");
    let rec = get_doc(&app, &doc_id(&results, "row:3")).await;
    assert_eq!(
        (&rec["exchangeRate"], &rec["totals"]["payable"]),
        (&json!("24.335"), &json!("121.00"))
    );
}

#[tokio::test]
async fn sample_downloads_and_imports_back() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    set_company(&app, true).await;
    let resp = send(
        app.clone(),
        get("/api/import/csv/sample", Some(common::TEST_TOKEN)),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let h = resp.headers();
    assert_eq!(h["content-type"], "text/csv; charset=utf-8");
    assert_eq!(
        h["content-disposition"],
        "attachment; filename=\"import-sample.csv\""
    );
    let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20)
        .await
        .expect("body")
        .to_vec();
    assert!(bytes.starts_with(b"\xEF\xBB\xBF"));
    let text = String::from_utf8(bytes.clone()).expect("utf-8");
    let header = text.lines().next().expect("header");
    for col in [
        "direction",
        "base_21",
        "vat_21",
        "base_12",
        "vat_12",
        "base_0",
        "total_czk",
        "note",
    ] {
        assert!(
            header
                .split(';')
                .any(|c| c.trim_start_matches('\u{feff}') == col),
            "{col} in {header}"
        );
    }
    assert!(!header.contains("vat_0"));
    assert_eq!(text.matches("\r\n").count(), 4);

    let rows = preview(&app, &bytes).await;
    assert_eq!(rows.len(), 3);
    for r in &rows {
        assert_eq!(r["status"], "ok", "{r}");
    }
    let results = confirm(&app, &bytes, &["row:2", "row:3", "row:4"]).await;
    let credit = get_doc(&app, &doc_id(&results, "row:4")).await;
    assert_eq!(credit["docType"], "credit_note");
    assert_eq!(
        credit["relatedDocumentId"],
        json!(doc_id(&results, "row:2"))
    );
    assert_eq!(credit["totals"]["payable"], "1210.00");
    assert_eq!(by_key(&results, "row:3")["status"], "imported");
    let resp = send(app.clone(), get("/api/import/csv/sample", None)).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn crafted_xlsx_and_too_many_rate_columns() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    set_company(&app, true).await;
    // A1 + XFD1048576: the declared used range is never allocated.
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    ws.write_string(0, 0, "direction").expect("a1");
    ws.write_string(1_048_575, 16_383, "x").expect("xfd");
    let bytes = wb.save_to_buffer().expect("xlsx");
    assert_eq!(
        file_error(&app, &bytes).await,
        json!({ "code": "validation", "fields": { "file": "invalid" } })
    );
    // More rate columns than a document holds recap rows (50).
    let rates: Vec<String> = (30..=77).map(|r| format!("base_{r}")).collect();
    let header = format!("{HEADER};{}", rates.join(";"));
    assert_eq!(
        file_error(&app, &file(&header, &[&received("FV-1")])).await["detail"],
        "base_77",
        "the 51st rate (base_21 / base_12 / base_0 come first)"
    );
}

#[tokio::test]
async fn non_payer_company_imports_issued_rows_as_non_payer() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    set_company(&app, false).await;
    let issued = |vat: &str, total: &str| {
        row(&[
            ("direction", "issued"),
            ("doc_type", "invoice"),
            ("number", "NP-1"),
            ("issue_date", "1.3.2026"),
            ("counterparty_name", "Fiktivní Odběratel s.r.o."),
            ("base_21", "1000"),
            ("vat_21", vat),
            ("total", total),
        ])
    };
    let bad = preview(&app, &file(HEADER, &[&issued("210", "1210")])).await;
    assert_eq!(
        (&bad[0]["error"], &bad[0]["field"]),
        (&json!("not_allowed"), &json!("vat_21"))
    );
    let bytes = file(HEADER, &[&issued("", "1000")]);
    let results = confirm(&app, &bytes, &["row:2"]).await;
    let doc = get_doc(&app, &doc_id(&results, "row:2")).await;
    assert_eq!(
        (&doc["vatMode"], &doc["supplier"]["vatPayer"]),
        (&json!("non_payer"), &json!(false))
    );
}
