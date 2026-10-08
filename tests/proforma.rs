//! Proformas: create / issue rules, cancel guard, DDPP immutability.

mod common;

use axum::http::{Method, StatusCode};
use common::documents::{
    create_doc, create_issued, get_doc, id, issuable, issue, pay, post_action,
};
use common::{TestDb, call, router};
use serde_json::{Value, json};

fn proforma(mut body: Value) -> Value {
    body["docType"] = json!("proforma");
    body
}

#[tokio::test]
async fn proforma_is_issued_without_tax_point_date() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let invoice_body = issuable(&app).await;
    let body = proforma(invoice_body.clone());

    let mut with_tax_point = body.clone();
    with_tax_point["taxPointDate"] = json!("2026-10-01");
    let (status, err) = call(&app, Method::POST, "/api/documents", Some(with_tax_point)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(err["fields"], json!({ "taxPointDate": "invalid" }));

    let draft = create_doc(&app, body).await;
    assert_eq!(draft["docType"], "proforma");
    assert_eq!(draft["taxPointDate"], Value::Null);
    assert_eq!(draft["settled"], false);
    assert_eq!(draft["sign"], 1);

    // The type of a draft cannot change.
    let mut put = draft.clone();
    put["docType"] = json!("invoice");
    let uri = format!("/api/documents/{}", id(&draft));
    let (status, err) = call(&app, Method::PUT, &uri, Some(put)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(err["fields"], json!({ "docType": "invalid" }));
    let mut put = draft.clone();
    put["docType"] = Value::Null;
    let (status, saved) = call(&app, Method::PUT, &uri, Some(put)).await;
    assert_eq!(
        (status, &saved["docType"]),
        (StatusCode::OK, &json!("proforma"))
    );

    let (status, issued) = issue(&app, &id(&draft)).await;
    assert_eq!(status, StatusCode::OK, "{issued}");
    assert_eq!(issued["number"], "Z20260001");
    assert_eq!(issued["taxPointDate"], Value::Null);
    assert_eq!(issued["paymentState"], "unpaid");
    assert_eq!(issued["totals"]["payable"], "1210.00");

    // An invoice keeps its own series.
    let invoice = create_issued(&app, invoice_body).await;
    assert_eq!(invoice["number"], "20260001");
}

#[tokio::test]
async fn cancel_is_refused_while_paid() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let body = proforma(issuable(&app).await);
    let p = create_issued(&app, body.clone()).await;
    let (status, payment) = pay(&app, &id(&p), json!({ "amount": "100" })).await;
    assert_eq!(status, StatusCode::CREATED, "{payment}");
    let (status, err) = post_action(&app, &id(&p), "cancel").await;
    assert_eq!(
        (status, err),
        (StatusCode::CONFLICT, json!({ "code": "invalid_state" }))
    );

    let unpaid = create_issued(&app, body).await;
    let (status, cancelled) = post_action(&app, &id(&unpaid), "cancel").await;
    assert_eq!(
        (status, &cancelled["status"]),
        (StatusCode::OK, &json!("cancelled"))
    );
}

#[tokio::test]
async fn ddpp_is_immutable() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let p = create_issued(&app, proforma(issuable(&app).await)).await;
    let (_, payment) = pay(&app, &id(&p), json!({ "amount": "121" })).await;
    let ddpp_id = payment["advanceDocumentId"]
        .as_str()
        .expect("DDPP issued")
        .to_string();
    let ddpp = get_doc(&app, &ddpp_id).await;

    let conflict = |code: &str| json!({ "code": code });
    let uri = format!("/api/documents/{ddpp_id}");
    let (status, err) = call(&app, Method::PUT, &uri, Some(ddpp.clone())).await;
    assert_eq!(
        (status, err),
        (StatusCode::CONFLICT, conflict("document_locked"))
    );
    let (status, err) = call(&app, Method::DELETE, &uri, None).await;
    assert_eq!(
        (status, err),
        (StatusCode::CONFLICT, conflict("document_locked"))
    );
    for action in ["cancel", "issue"] {
        let (status, err) = post_action(&app, &ddpp_id, action).await;
        assert_eq!(
            (status, err),
            (StatusCode::CONFLICT, conflict("invalid_state")),
            "{action}"
        );
    }
    let (status, err) = pay(&app, &ddpp_id, json!({ "amount": "1" })).await;
    assert_eq!(
        (status, err),
        (StatusCode::CONFLICT, conflict("invalid_state"))
    );

    // No payment state, never overdue, and invisible to payment filters.
    assert_eq!(ddpp["paymentState"], Value::Null);
    assert_eq!(ddpp["overdue"], false);
    let (_, list) = call(
        &app,
        Method::GET,
        "/api/documents?paymentState=unpaid&docType=advance_tax_doc",
        None,
    )
    .await;
    assert_eq!(list["total"], 0);
    let (_, list) = call(
        &app,
        Method::GET,
        "/api/documents?docType=advance_tax_doc",
        None,
    )
    .await;
    assert_eq!(list["total"], 1);
    assert_eq!(list["items"][0]["paymentState"], Value::Null);
    assert_eq!(list["items"][0]["relatedDocumentId"], json!(id(&p)));
}
