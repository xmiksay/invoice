//! Payments and the derived payment state.

mod common;

use axum::http::{Method, StatusCode};
use common::documents::{create_doc, id, issuable, issue};
use common::{TestDb, call, router};
use serde_json::{Value, json};

async fn pay(app: &axum::Router, doc: &str, amount: &str) -> (StatusCode, Value) {
    call(
        app,
        Method::POST,
        &format!("/api/documents/{doc}/payments"),
        Some(json!({ "date": "2026-10-05", "amount": amount, "note": " bank " })),
    )
    .await
}

async fn state_of(app: &axum::Router, doc: &str) -> (Value, Value) {
    let (_, d) = call(app, Method::GET, &format!("/api/documents/{doc}"), None).await;
    (d["paymentState"].clone(), d["paid"].clone())
}

#[tokio::test]
async fn payments_drive_the_payment_state() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let body = issuable(&app).await; // payable 1210.00
    let doc = id(&create_doc(&app, body).await);
    issue(&app, &doc).await;
    assert_eq!(state_of(&app, &doc).await, (json!("unpaid"), json!("0.00")));

    let (status, p1) = pay(&app, &doc, "200.5").await;
    assert_eq!(status, StatusCode::CREATED, "{p1}");
    assert_eq!(p1["amount"], "200.50");
    assert_eq!(p1["note"], "bank");
    assert_eq!(p1["date"], "2026-10-05");
    assert_eq!(
        state_of(&app, &doc).await,
        (json!("partial"), json!("200.50"))
    );

    let (_, p2) = pay(&app, &doc, "1009.50").await;
    assert_eq!(
        state_of(&app, &doc).await,
        (json!("paid"), json!("1210.00"))
    );
    pay(&app, &doc, "1").await;
    assert_eq!(
        state_of(&app, &doc).await,
        (json!("overpaid"), json!("1211.00"))
    );

    let (status, list) = call(
        &app,
        Method::GET,
        &format!("/api/documents/{doc}/payments"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().map(Vec::len), Some(3));
    assert_eq!(list[0]["id"], p1["id"]);

    let p2_uri = format!(
        "/api/documents/{doc}/payments/{}",
        p2["id"].as_str().expect("id")
    );
    let (status, _) = call(&app, Method::DELETE, &p2_uri, None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(
        state_of(&app, &doc).await,
        (json!("partial"), json!("201.50"))
    );
    let (status, _) = call(&app, Method::DELETE, &p2_uri, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Cancelled: payments stay but are frozen.
    call(
        &app,
        Method::POST,
        &format!("/api/documents/{doc}/cancel"),
        None,
    )
    .await;
    let (status, err) = pay(&app, &doc, "1").await;
    assert_eq!(
        (status, err),
        (StatusCode::CONFLICT, json!({ "code": "invalid_state" }))
    );
    let p1_uri = format!(
        "/api/documents/{doc}/payments/{}",
        p1["id"].as_str().expect("id")
    );
    let (status, _) = call(&app, Method::DELETE, &p1_uri, None).await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (_, list) = call(
        &app,
        Method::GET,
        &format!("/api/documents/{doc}/payments"),
        None,
    )
    .await;
    assert_eq!(list.as_array().map(Vec::len), Some(2));
    assert_eq!(state_of(&app, &doc).await, (Value::Null, json!("201.50")));
}

#[tokio::test]
async fn payments_are_rejected_on_drafts_and_validated() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let body = issuable(&app).await;
    let draft = id(&create_doc(&app, body.clone()).await);
    let (status, err) = pay(&app, &draft, "10").await;
    assert_eq!(
        (status, err),
        (StatusCode::CONFLICT, json!({ "code": "invalid_state" }))
    );

    let doc = id(&create_doc(&app, body).await);
    issue(&app, &doc).await;
    for (amount, reason) in [
        ("0", "invalid"),
        ("-5", "invalid"),
        ("1.005", "invalid"),
        ("", "required"),
    ] {
        let (status, err) = pay(&app, &doc, amount).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{amount}");
        assert_eq!(err["fields"], json!({ "amount": reason }), "{amount}");
    }
    // Each amount fits, but the sum would overflow documents.paid.
    let max = "9999999999999999.99";
    assert_eq!(pay(&app, &doc, max).await.0, StatusCode::CREATED);
    let (status, err) = pay(&app, &doc, "0.01").await;
    assert_eq!(
        (status, &err["fields"]),
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            &json!({ "amount": "invalid" })
        )
    );
    assert_eq!(state_of(&app, &doc).await, (json!("overpaid"), json!(max)));

    let (status, err) = call(
        &app,
        Method::POST,
        &format!("/api/documents/{doc}/payments"),
        Some(json!({ "amount": "5" })),
    )
    .await;
    assert_eq!(
        (status, &err["fields"]),
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            &json!({ "date": "required" })
        )
    );
    let (status, _) = call(
        &app,
        Method::GET,
        "/api/documents/00000000-0000-0000-0000-000000000001/payments",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
