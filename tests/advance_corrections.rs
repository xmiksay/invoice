//! DDPP corrections (`advance_credit_note`): creation, cap, exact VAT,
//! settlement net of corrections and the guards around deductions.

mod common;

use axum::http::{Method, StatusCode};
use common::documents::{
    create_bank, create_contact, create_issued, get_doc, id, issuable, issue, item, pay,
    post_action, set_company,
};
use common::{TestDb, call, router};
use serde_json::{Value, json};

/// A payer proforma 1000 @ 21 % + 500 @ 12 % paid 1000 → its DDPP:
/// 564.98 / 118.64 @ 21 %, 282.48 / 33.90 @ 12 % (VAT from above).
async fn paid_proforma(app: &axum::Router) -> (Value, Value, String) {
    let mut body = issuable(app).await;
    body["docType"] = json!("proforma");
    body["lines"] = json!([item("1", "1000", "21"), item("1", "500", "12")]);
    let p = create_issued(app, body).await;
    let (status, payment) = pay(app, &id(&p), json!({ "amount": "1000" })).await;
    assert_eq!(status, StatusCode::CREATED, "{payment}");
    let ddpp = payment["advanceDocumentId"]
        .as_str()
        .expect("DDPP")
        .to_string();
    (p, payment, ddpp)
}

async fn correct(app: &axum::Router, ddpp: &str) -> (StatusCode, Value) {
    call(
        app,
        Method::POST,
        &format!("/api/documents/{ddpp}/credit-note"),
        Some(json!({ "correctionReason": "Vrácení zálohy" })),
    )
    .await
}

async fn put_lines(app: &axum::Router, doc: &Value, lines: Value) -> Value {
    let mut put = doc.clone();
    put["lines"] = lines;
    let uri = format!("/api/documents/{}", id(doc));
    let (status, saved) = call(app, Method::PUT, &uri, Some(put)).await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    saved
}

fn rate_row(doc: &Value, rate: &str) -> Value {
    doc["totals"]["recap"]
        .as_array()
        .expect("recap")
        .iter()
        .find(|r| r["vatRate"] == rate)
        .cloned()
        .unwrap_or(Value::Null)
}

#[tokio::test]
async fn full_correction_nets_the_ddpp_to_zero() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let (p, _, ddpp) = paid_proforma(&app).await;

    let (status, oc) = correct(&app, &ddpp).await;
    assert_eq!(status, StatusCode::CREATED, "{oc}");
    assert_eq!(
        (&oc["docType"], &oc["status"], &oc["sign"]),
        (&json!("advance_credit_note"), &json!("draft"), &json!(-1))
    );
    assert_eq!(oc["relatedDocumentId"], json!(ddpp));
    assert_eq!(oc["parent"]["docType"], "advance_tax_doc");
    assert_eq!(oc["lines"].as_array().map(Vec::len), Some(2));
    // Exact VAT: the DDPP's 118.64, not round2(564.98 × 21 %) = 118.65.
    assert_eq!(rate_row(&oc, "21")["vat"], "118.64");
    assert_eq!(rate_row(&oc, "12")["vat"], "33.90");
    assert_eq!(oc["totals"]["total"], "1000.00");
    let (status, c) = call(
        &app,
        Method::POST,
        "/api/documents/compute",
        Some(
            json!({ "documentId": id(&oc), "docType": "advance_credit_note",
                     "lines": oc["lines"].clone() }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{c}");
    assert_eq!(c["totals"]["vat"], "152.54");

    let (status, issued) = issue(&app, &id(&oc)).await;
    assert_eq!(status, StatusCode::OK, "{issued}");
    let number = issued["number"].as_str().expect("number");
    assert!(
        number.starts_with("OP") && number.ends_with("0001"),
        "{number}"
    );
    assert_eq!(issued["totals"]["total"], "1000.00");
    assert_eq!(rate_row(&issued, "21")["vat"], "118.64");
    assert_eq!(issued["paymentState"], "unpaid", "a refund is due");

    // Nothing left: settle skips the DDPP, an advance line on it is invalid.
    let (status, inv) = post_action(&app, &id(&p), "settle").await;
    assert_eq!(status, StatusCode::CREATED, "{inv}");
    assert!(
        inv["lines"]
            .as_array()
            .expect("lines")
            .iter()
            .all(|l| l["kind"] != "advance")
    );
    let mut lines = inv["lines"].clone();
    lines
        .as_array_mut()
        .expect("lines")
        .push(json!({ "kind": "advance", "advanceDocumentId": ddpp }));
    let mut put = inv.clone();
    put["lines"] = lines;
    let uri = format!("/api/documents/{}", id(&inv));
    let (status, err) = call(&app, Method::PUT, &uri, Some(put)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        err["fields"],
        json!({ "lines.2.advanceDocumentId": "invalid" })
    );

    // Nothing left to correct: the DDPP says why, the endpoint refuses.
    assert_eq!(
        get_doc(&app, &ddpp).await["correctionBlock"],
        "fully_corrected"
    );
    // Every document carries the field, `null` when it does not apply.
    let proforma = get_doc(&app, &id(&p)).await;
    assert_eq!(proforma.get("correctionBlock"), Some(&Value::Null));
    let (status, err) = correct(&app, &ddpp).await;
    assert_eq!(
        (status, err),
        (StatusCode::CONFLICT, json!({ "code": "invalid_state" }))
    );
}

#[tokio::test]
async fn partial_correction_reduces_the_deduction() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let (p, _, ddpp) = paid_proforma(&app).await;

    let (_, oc) = correct(&app, &ddpp).await;
    let oc = put_lines(&app, &oc, json!([item("1", "100", "21")])).await;
    assert_eq!(
        rate_row(&oc, "21")["vat"],
        "21.00",
        "partial: computed normally"
    );
    let (status, issued) = issue(&app, &id(&oc)).await;
    assert_eq!(status, StatusCode::OK, "{issued}");

    // A rate the DDPP does not have, or more than is left, is refused.
    let (_, more) = correct(&app, &ddpp).await;
    let more = put_lines(&app, &more, json!([item("1", "1", "0")])).await;
    assert_eq!(
        issue(&app, &id(&more)).await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    put_lines(&app, &more, json!([item("1", "464.99", "21")])).await;
    assert_eq!(
        issue(&app, &id(&more)).await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );

    // The rest of 21 % exactly: VAT = 118.64 − 21.00.
    let rest = put_lines(&app, &more, json!([item("1", "464.98", "21")])).await;
    assert_eq!(rate_row(&rest, "21")["vat"], "97.64");
    // Delete the draft instead; settle deducts the net amounts.
    let uri = format!("/api/documents/{}", id(&rest));
    assert_eq!(
        call(&app, Method::DELETE, &uri, None).await.0,
        StatusCode::NO_CONTENT
    );
    let (status, inv) = post_action(&app, &id(&p), "settle").await;
    assert_eq!(status, StatusCode::CREATED, "{inv}");
    let adv = inv["lines"]
        .as_array()
        .expect("lines")
        .iter()
        .find(|l| l["kind"] == "advance")
        .cloned()
        .expect("advance line");
    assert_eq!(
        adv["recap"],
        json!([
            { "vatRate": "21", "base": "-464.98", "vat": "-97.64" },
            { "vatRate": "12", "base": "-282.48", "vat": "-33.90" },
        ])
    );
}

#[tokio::test]
async fn deductions_payments_and_cancel_guards() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let (p, payment, ddpp) = paid_proforma(&app).await;
    let payment_uri = format!(
        "/api/documents/{}/payments/{}",
        id(&p),
        payment["id"].as_str().expect("payment id")
    );

    assert_eq!(get_doc(&app, &ddpp).await["correctionBlock"], Value::Null);
    // A draft correction blocks deleting the payment.
    let (_, oc) = correct(&app, &ddpp).await;
    let (status, err) = call(&app, Method::DELETE, &payment_uri, None).await;
    assert_eq!(
        (status, err),
        (StatusCode::CONFLICT, json!({ "code": "advance_in_use" }))
    );

    // A draft settlement deducts the DDPP: no new correction, no issue.
    let (_, inv) = post_action(&app, &id(&p), "settle").await;
    assert_eq!(
        get_doc(&app, &ddpp).await["correctionBlock"],
        "advance_in_use"
    );
    let (status, err) = correct(&app, &ddpp).await;
    assert_eq!(
        (status, err),
        (StatusCode::CONFLICT, json!({ "code": "advance_in_use" }))
    );
    let (status, err) = issue(&app, &id(&oc)).await;
    assert_eq!(
        (status, err),
        (StatusCode::CONFLICT, json!({ "code": "advance_in_use" }))
    );
    // Issued settlement → advance_settled.
    let (status, _) = issue(&app, &id(&inv)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        get_doc(&app, &ddpp).await["correctionBlock"],
        "advance_settled"
    );
    let (status, err) = correct(&app, &ddpp).await;
    assert_eq!(
        (status, err),
        (StatusCode::CONFLICT, json!({ "code": "advance_settled" }))
    );
    // Free the DDPP again; the correction issues, then cannot be cancelled
    // while a new settlement deducts what is left.
    post_action(&app, &id(&inv), "cancel").await;
    let oc = put_lines(&app, &oc, json!([item("1", "100", "21")])).await;
    let (status, issued) = issue(&app, &id(&oc)).await;
    assert_eq!(status, StatusCode::OK, "{issued}");
    let (_, inv2) = post_action(&app, &id(&p), "settle").await;
    issue(&app, &id(&inv2)).await;
    let (status, err) = post_action(&app, &id(&oc), "cancel").await;
    assert_eq!(
        (status, err),
        (StatusCode::CONFLICT, json!({ "code": "advance_settled" }))
    );
    post_action(&app, &id(&inv2), "cancel").await;
    let (status, c) = post_action(&app, &id(&oc), "cancel").await;
    assert_eq!(
        (status, &c["status"]),
        (StatusCode::OK, &json!("cancelled"))
    );
    // Only cancelled corrections left: the payment can go, its DDPP is cancelled.
    let (status, _) = call(&app, Method::DELETE, &payment_uri, None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(get_doc(&app, &ddpp).await["status"], "cancelled");
    let (status, _) = correct(&app, &ddpp).await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn foreign_currency_keeps_the_ddpp_rate_and_czk_amounts() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    set_company(&app, true).await;
    let contact = create_contact(&app, json!({})).await;
    create_bank(&app, "EUR").await;
    let p = create_issued(
        &app,
        json!({ "docType": "proforma", "contactId": contact, "currency": "EUR",
                "exchangeRate": "25", "lines": [item("1", "1000", "21"), item("1", "500", "12")] }),
    )
    .await;
    let (status, payment) = pay(
        &app,
        &id(&p),
        json!({ "amount": "1000", "exchangeRate": "25.1" }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{payment}");
    let ddpp = get_doc(&app, payment["advanceDocumentId"].as_str().expect("DDPP")).await;

    let (_, oc) = correct(&app, &id(&ddpp)).await;
    assert_eq!(
        (&oc["exchangeRate"], &oc["exchangeRateSource"]),
        (&json!("25.1"), &json!("original"))
    );
    let (status, issued) = issue(&app, &id(&oc)).await;
    assert_eq!(status, StatusCode::OK, "{issued}");
    assert_eq!(issued["totals"]["recap"], ddpp["totals"]["recap"]);
    assert_eq!(issued["totals"]["total"], ddpp["totals"]["total"]);
}

#[tokio::test]
async fn imported_corrections_of_a_deducted_ddpp_are_guarded() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let (p, _, ddpp) = paid_proforma(&app).await;
    let base = json!({ "contactId": p["contactId"], "taxPointDate": "2026-10-05" });
    let imported = |number: &str| {
        let mut b = base.clone();
        b["imported"] = json!(true);
        b["docType"] = json!("advance_credit_note");
        b["number"] = json!(number);
        b["relatedDocumentId"] = json!(ddpp);
        b["lines"] = json!([item("1", "10", "21")]);
        b
    };
    // Issued while the DDPP is free; then a settlement deducts the DDPP.
    let (_, first) = call(
        &app,
        Method::POST,
        "/api/documents",
        Some(imported("OP-OLD-1")),
    )
    .await;
    let (status, first) = issue(&app, &id(&first)).await;
    assert_eq!(status, StatusCode::OK, "{first}");
    let (_, second) = call(
        &app,
        Method::POST,
        "/api/documents",
        Some(imported("OP-OLD-2")),
    )
    .await;
    let (_, inv) = post_action(&app, &id(&p), "settle").await;
    let (status, err) = issue(&app, &id(&second)).await;
    assert_eq!(
        (status, err),
        (StatusCode::CONFLICT, json!({ "code": "advance_in_use" }))
    );
    issue(&app, &id(&inv)).await;
    let (status, err) = post_action(&app, &id(&first), "cancel").await;
    assert_eq!(
        (status, err),
        (StatusCode::CONFLICT, json!({ "code": "advance_settled" }))
    );
}
