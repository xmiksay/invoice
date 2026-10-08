//! Received documents in the list: filters, search, summary fields.

mod common;

use axum::http::{Method, StatusCode};
use common::documents::{create_contact, id, set_company};
use common::received::{create_category, create_received};
use common::{TestDb, call, router};
use serde_json::json;

#[tokio::test]
async fn list_filters_and_search() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    set_company(&app, true).await;
    let s = create_contact(
        &app,
        json!({ "name": "Dodavatel Test s.r.o.", "ico": "27074358" }),
    )
    .await;
    let cat = create_category(&app, "Software", "expense").await;
    let a = create_received(
        &app,
        &s,
        json!({ "supplierNumber": "ABC-42", "categoryId": cat }),
    )
    .await;
    create_received(&app, &s, json!({ "supplierNumber": "XYZ-1" })).await;
    let get = |q: &'static str| {
        let app = app.clone();
        async move {
            let (status, body) = call(&app, Method::GET, &format!("/api/documents{q}"), None).await;
            assert_eq!(status, StatusCode::OK, "{body}");
            body
        }
    };
    let l = get("?direction=received&q=abc-4").await;
    assert_eq!(l["total"], 1);
    let item = &l["items"][0];
    assert_eq!(item["id"], json!(id(&a)));
    assert_eq!(
        (
            &item["supplierNumber"],
            &item["categoryId"],
            &item["hasPdf"],
            &item["imported"]
        ),
        (&json!("ABC-42"), &json!(cat), &json!(false), &json!(false))
    );
    assert_eq!(item["customerName"], "Dodavatel Test s.r.o.");
    assert_eq!(get("?q=Dodavatel%20Test").await["total"], 2);
    let by_cat = format!("?categoryId={cat}");
    let (_, l) = call(&app, Method::GET, &format!("/api/documents{by_cat}"), None).await;
    assert_eq!(l["total"], 1);
    assert_eq!(get("?imported=true").await["total"], 0);
    assert_eq!(get("?imported=false&direction=received").await["total"], 2);
    assert_eq!(get("?direction=issued").await["total"], 0);
}
