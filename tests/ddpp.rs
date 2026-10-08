//! DDPP issued automatically for proforma payments of a VAT payer.

mod common;

use axum::http::{Method, StatusCode};
use common::documents::{
    create_bank, create_contact, create_issued, dead_url, get_doc, id, issuable, item, mock_cnb,
    pay, post_action, set_company,
};
use common::{TestDb, call, router, router_with_cnb};
use serde_json::{Value, json};

/// A payer proforma with a 21 % (1000) and a 12 % (500) line → payable 1770.
async fn two_rate_proforma(app: &axum::Router) -> Value {
    let mut body = issuable(app).await;
    body["docType"] = json!("proforma");
    body["lines"] = json!([item("1", "1000", "21"), item("1", "500", "12")]);
    create_issued(app, body).await
}

fn ddpp_id(payment: &Value) -> String {
    payment["advanceDocumentId"]
        .as_str()
        .expect("DDPP issued")
        .to_string()
}

#[tokio::test]
async fn payment_issues_ddpp_split_by_rate() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let p = two_rate_proforma(&app).await;
    assert_eq!(p["totals"]["payable"], "1770.00");

    let (status, payment) = pay(&app, &id(&p), json!({ "amount": "1000" })).await;
    assert_eq!(status, StatusCode::CREATED, "{payment}");
    let d = get_doc(&app, &ddpp_id(&payment)).await;
    assert_eq!(d["docType"], "advance_tax_doc");
    assert_eq!(d["number"], "DP20260001");
    assert_eq!(d["status"], "issued");
    for f in ["issueDate", "taxPointDate", "dueDate"] {
        assert_eq!(d[f], "2026-10-05", "{f}");
    }
    assert_eq!(d["relatedDocumentId"], json!(id(&p)));
    assert_eq!(d["parent"]["id"], json!(id(&p)));
    assert_eq!(
        (&d["parent"]["docType"], &d["parent"]["number"]),
        (&json!("proforma"), &json!("Z20260001"))
    );
    assert_eq!(d["paymentId"], payment["id"]);
    assert_eq!(d["contactId"], p["contactId"]);
    assert_eq!(d["customer"], p["customer"]);
    assert_eq!(d["supplier"], p["supplier"]);
    // Gross 1210 / 560: 12 % gets round2(1000 × 560 / 1770) = 316.38, 21 % the rest (683.62).
    // VAT from above: 683.62 × 21/121 = 118.64, 316.38 × 12/112 = 33.90.
    assert_eq!(
        d["totals"]["recap"],
        json!([
            { "vatRate": "21", "base": "564.98", "vat": "118.64", "baseCzk": null, "vatCzk": null },
            { "vatRate": "12", "base": "282.48", "vat": "33.90", "baseCzk": null, "vatCzk": null },
        ])
    );
    assert_eq!(d["totals"]["total"], "1000.00");
    assert_eq!(d["totals"]["payable"], "1000.00");
    let lines = d["lines"].as_array().expect("lines");
    assert_eq!(lines.len(), 2);
    assert_eq!(
        lines[0]["description"],
        "Přijatá záloha k zálohové faktuře Z20260001"
    );
    assert_eq!(
        (&lines[0]["unitPrice"], &lines[0]["quantity"]),
        (&json!("564.98"), &json!("1"))
    );

    let (_, second) = pay(&app, &id(&p), json!({ "amount": "770" })).await;
    assert_eq!(
        get_doc(&app, &ddpp_id(&second)).await["number"],
        "DP20260002"
    );

    let proforma = get_doc(&app, &id(&p)).await;
    assert_eq!(proforma["paymentState"], "paid");
    let related = proforma["relatedDocuments"].as_array().expect("related");
    assert_eq!(related.len(), 2);
    assert_eq!(related[0]["docType"], "advance_tax_doc");
    assert_eq!(related[0]["payable"], "1000.00");

    let (_, payments) = call(
        &app,
        Method::GET,
        &format!("/api/documents/{}/payments", id(&p)),
        None,
    )
    .await;
    assert_eq!(
        payments[0]["advanceDocumentId"],
        payment["advanceDocumentId"]
    );
}

#[tokio::test]
async fn non_payer_and_invoices_issue_no_ddpp() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let mut body = issuable(&app).await;
    set_company(&app, false).await;
    body["lines"] = json!([item("1", "1000", "0")]);
    let invoice = create_issued(&app, body.clone()).await;
    body["docType"] = json!("proforma");
    let p = create_issued(&app, body).await;
    assert_eq!(p["vatMode"], "non_payer");
    for doc in [&p, &invoice] {
        let (status, payment) = pay(&app, &id(doc), json!({ "amount": "100" })).await;
        assert_eq!(status, StatusCode::CREATED, "{payment}");
        assert_eq!(payment["advanceDocumentId"], Value::Null);
    }
    let (_, list) = call(
        &app,
        Method::GET,
        "/api/documents?docType=advance_tax_doc",
        None,
    )
    .await;
    assert_eq!(list["total"], 0);
}

#[tokio::test]
async fn foreign_currency_rate_from_cnb_or_manual() {
    let db = TestDb::new().await;
    let (url, _) = mock_cnb().await;
    let app = router_with_cnb(db.conn.clone(), &url);
    set_company(&app, true).await;
    let contact = create_contact(&app, json!({})).await;
    create_bank(&app, "EUR").await;
    let body = json!({ "docType": "proforma", "contactId": contact, "currency": "EUR",
        "issueDate": "2026-10-01", "lines": [item("1", "100", "21")] });
    let p = create_issued(&app, body.clone()).await;

    let (_, payment) = pay(&app, &id(&p), json!({ "amount": "121" })).await;
    let d = get_doc(&app, &ddpp_id(&payment)).await;
    assert_eq!(d["exchangeRate"], "25.125");
    assert_eq!(d["exchangeRateSource"], "cnb");
    assert_eq!(d["exchangeRateDate"], "2026-10-05");
    assert_eq!(d["totals"]["recap"][0]["baseCzk"], "2512.50");
    assert_eq!(d["totals"]["recap"][0]["vatCzk"], "527.63");
    assert_eq!(d["totals"]["totalCzk"], "3040.13");

    let (_, payment) = pay(
        &app,
        &id(&p),
        json!({ "amount": "121", "exchangeRate": "24.5" }),
    )
    .await;
    let d = get_doc(&app, &ddpp_id(&payment)).await;
    assert_eq!(
        (&d["exchangeRate"], &d["exchangeRateSource"]),
        (&json!("24.5"), &json!("manual"))
    );
    assert_eq!(d["exchangeRateDate"], Value::Null);
    let (status, err) = pay(&app, &id(&p), json!({ "amount": "1", "exchangeRate": "0" })).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(err["fields"], json!({ "exchangeRate": "invalid" }));

    // ČNB down and no manual rate: 422 and the payment is not stored.
    let down = router_with_cnb(db.conn.clone(), &dead_url());
    let mut manual = body;
    manual["exchangeRate"] = json!("25");
    let p2 = create_issued(&down, manual).await;
    let (status, err) = pay(
        &down,
        &id(&p2),
        json!({ "amount": "10", "date": "2026-09-01" }),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(err["fields"], json!({ "exchangeRate": "required" }));
    let p2 = get_doc(&down, &id(&p2)).await;
    assert_eq!(
        (&p2["paid"], &p2["relatedDocuments"]),
        (&json!("0.00"), &json!([]))
    );
}

#[tokio::test]
async fn deleting_the_payment_cancels_its_ddpp() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let p = two_rate_proforma(&app).await;
    let (_, payment) = pay(&app, &id(&p), json!({ "amount": "500" })).await;
    let ddpp = ddpp_id(&payment);
    // A native DDPP is cancelled only through its payment.
    let (status, e) = call(
        &app,
        Method::POST,
        &format!("/api/documents/{ddpp}/cancel"),
        None,
    )
    .await;
    assert_eq!(
        (status, &e["code"]),
        (StatusCode::CONFLICT, &json!("invalid_state"))
    );
    let uri = format!(
        "/api/documents/{}/payments/{}",
        id(&p),
        payment["id"].as_str().expect("payment id")
    );
    let (status, _) = call(&app, Method::DELETE, &uri, None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let d = get_doc(&app, &ddpp).await;
    assert_eq!(d["status"], "cancelled");
    assert_eq!(d["cancelReason"], "Platba smazána");
    assert!(d["cancelledAt"].is_string());
    assert_eq!(get_doc(&app, &id(&p)).await["paid"], "0.00");
}

#[tokio::test]
async fn payment_of_a_settled_ddpp_cannot_be_deleted() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let p = two_rate_proforma(&app).await;
    let (_, payment) = pay(&app, &id(&p), json!({ "amount": "1770" })).await;
    let (status, invoice) = post_action(&app, &id(&p), "settle").await;
    assert_eq!(status, StatusCode::CREATED, "{invoice}");
    let uri = format!(
        "/api/documents/{}/payments/{}",
        id(&p),
        payment["id"].as_str().expect("payment id")
    );
    // A draft deduction must be removed first …
    let (status, err) = call(&app, Method::DELETE, &uri, None).await;
    assert_eq!(
        (status, err),
        (StatusCode::CONFLICT, json!({ "code": "advance_in_use" }))
    );
    // … and once an issued invoice deducts the DDPP, its payment is frozen.
    let (status, issued) = post_action(&app, &id(&invoice), "issue").await;
    assert_eq!(status, StatusCode::OK, "{issued}");
    let (status, err) = call(&app, Method::DELETE, &uri, None).await;
    assert_eq!(
        (status, err),
        (StatusCode::CONFLICT, json!({ "code": "advance_settled" }))
    );
    assert_eq!(get_doc(&app, &ddpp_id(&payment)).await["status"], "issued");
    assert_eq!(get_doc(&app, &id(&p)).await["paid"], "1770.00");
}
