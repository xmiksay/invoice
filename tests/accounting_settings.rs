//! Settings → Accounting (`GET` / `PUT /api/settings/accounting`).

mod common;

use axum::http::{Method, StatusCode};
use common::{TestDb, call, get, router, send};
use serde_json::{Value, json};

const URI: &str = "/api/settings/accounting";

fn row(s: &Value, direction: &str, doc_type: &str) -> Value {
    s["pohoda"]["codes"]
        .as_array()
        .expect("codes")
        .iter()
        .find(|r| r["direction"] == direction && r["docType"] == doc_type)
        .cloned()
        .unwrap_or_else(|| panic!("no row {direction} {doc_type} in {s}"))
}

#[tokio::test]
async fn get_put_and_validation() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());

    let resp = send(app.clone(), get(URI, None)).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let (status, s) = call(&app, Method::GET, URI, None).await;
    assert_eq!(status, StatusCode::OK, "{s}");
    assert_eq!(s["pohoda"]["ico"], Value::Null);
    let codes = s["pohoda"]["codes"].as_array().expect("codes");
    assert_eq!(codes.len(), 12, "every direction × exported type");
    assert_eq!(
        codes[0],
        json!({ "direction": "issued", "docType": "invoice", "accounting": null,
                "classificationVat": null, "classificationVatNonDeductible": null, "numberSeries": null })
    );
    assert!(s.get("money").is_none(), "Money S3 comes in 3c");

    let (status, saved) = call(
        &app,
        Method::PUT,
        URI,
        Some(json!({ "pohoda": { "ico": "87654326", "codes": [
            { "direction": "received", "docType": "simplified", "accounting": " 1Ppok ",
              "classificationVat": "", "classificationVatNonDeductible": "PN",
              "numberSeries": "PZ" },
        ]}})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    let (_, s) = call(&app, Method::GET, URI, None).await;
    assert_eq!(s, saved);
    assert_eq!(s["pohoda"]["ico"], "87654326");
    assert_eq!(s["pohoda"]["codes"].as_array().expect("codes").len(), 12);
    assert_eq!(
        row(&s, "received", "simplified"),
        json!({ "direction": "received", "docType": "simplified", "accounting": "1Ppok",
                "classificationVat": null, "classificationVatNonDeductible": "PN", "numberSeries": "PZ" })
    );
    assert_eq!(row(&s, "issued", "simplified")["accounting"], Value::Null);

    let (status, err) = call(
        &app,
        Method::PUT,
        URI,
        Some(json!({ "pohoda": { "ico": "12345678", "codes": [
            { "direction": "sideways", "docType": "invoice" },
            { "direction": "issued", "docType": "proforma" },
            { "direction": "issued", "docType": "invoice", "numberSeries": "N".repeat(20) },
            { "direction": "issued", "docType": "invoice" },
            { "direction": "issued", "docType": "credit_note", "classificationVatNonDeductible": "PN" },
        ]}})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        err,
        json!({ "code": "validation", "fields": {
            "pohoda.ico": "invalid_ico",
            "pohoda.codes.0.direction": "invalid",
            "pohoda.codes.1.docType": "invalid",
            "pohoda.codes.2.numberSeries": "too_long",
            "pohoda.codes.3.docType": "duplicate",
            "pohoda.codes.4.classificationVatNonDeductible": "invalid",
        }})
    );
    let (_, after) = call(&app, Method::GET, URI, None).await;
    assert_eq!(after, saved, "a rejected PUT changes nothing");

    // An empty object clears everything.
    let (status, cleared) = call(&app, Method::PUT, URI, Some(json!({}))).await;
    assert_eq!(status, StatusCode::OK, "{cleared}");
    assert_eq!(cleared["pohoda"]["ico"], Value::Null);
    assert_eq!(
        row(&cleared, "received", "simplified")["numberSeries"],
        Value::Null
    );
}
