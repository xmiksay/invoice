//! Catalog items and groups.

mod common;

use axum::http::{Method, StatusCode};
use common::{TestDb, call, router};
use serde_json::{Value, json};

async fn create_item(app: &axum::Router, name: &str, price: &str, rate: &str) -> Value {
    let (status, item) = call(
        app,
        Method::POST,
        "/api/catalog/items",
        Some(json!({ "name": name, "unit": "h", "unitPrice": price, "vatRate": rate })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{item}");
    item
}

fn id(v: &Value) -> String {
    v["id"].as_str().expect("id").to_string()
}

#[tokio::test]
async fn item_crud_and_filters() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let a = create_item(&app, "Programování", "1500.50", "21").await;
    assert_eq!(a["unitPrice"], "1500.5");
    assert_eq!(
        (&a["currency"], &a["active"]),
        (&json!("CZK"), &json!(true))
    );
    create_item(&app, "Analýza", "1000", "21").await;

    let uri = format!("/api/catalog/items/{}", id(&a));
    let (status, updated) = call(
        &app,
        Method::PUT,
        &uri,
        Some(
            json!({ "name": "Programování", "unitPrice": "60", "currency": "eur",
                     "vatRate": "21", "active": false, "note": "senior" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(
        (&updated["currency"], &updated["active"]),
        (&json!("EUR"), &json!(false))
    );

    let (_, all) = call(&app, Method::GET, "/api/catalog/items", None).await;
    let names: Vec<&str> = all
        .as_array()
        .expect("list")
        .iter()
        .filter_map(|i| i["name"].as_str())
        .collect();
    assert_eq!(names, vec!["Analýza", "Programování"]);
    let (_, active) = call(&app, Method::GET, "/api/catalog/items?active=true", None).await;
    assert_eq!(active.as_array().map(Vec::len), Some(1));
    let (_, found) = call(&app, Method::GET, "/api/catalog/items?q=PROGRAM", None).await;
    assert_eq!(found[0]["id"], a["id"]);

    let (status, err) = call(
        &app,
        Method::POST,
        "/api/catalog/items",
        Some(json!({ "unitPrice": "x" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        err["fields"],
        json!({ "name": "required", "unitPrice": "invalid", "vatRate": "required" })
    );

    let (status, _) = call(&app, Method::DELETE, &uri, None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(&app, Method::GET, &uri, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = call(&app, Method::DELETE, &uri, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn groups_members_mixed_vat_and_cascade() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let a = create_item(&app, "Práce", "1000", "21").await;
    let b = create_item(&app, "Materiál", "200", "21").await;
    let reduced = create_item(&app, "Kniha", "300", "12").await;

    let (status, g) = call(
        &app,
        Method::POST,
        "/api/catalog/groups",
        Some(json!({ "name": "Instalace", "members": [
            { "itemId": id(&b), "quantity": "2" }, { "itemId": id(&a), "quantity": "1.5" } ] })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{g}");
    assert_eq!(g["collapse"], true);
    let members = g["members"].as_array().expect("members");
    assert_eq!(members.len(), 2);
    assert_eq!(members[0]["itemId"], b["id"]);
    assert_eq!(
        (&members[0]["position"], &members[0]["quantity"]),
        (&json!(1), &json!("2"))
    );
    assert_eq!(members[1]["item"]["name"], "Práce");

    let mixed = json!({ "name": "Mix", "members": [
        { "itemId": id(&a), "quantity": "1" }, { "itemId": id(&reduced), "quantity": "1" } ] });
    let (status, err) = call(&app, Method::POST, "/api/catalog/groups", Some(mixed)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(err["fields"], json!({ "members": "mixed_vat" }));
    let bad = json!({ "name": "Bad", "members": [
        { "itemId": id(&a), "quantity": "1" }, { "itemId": id(&a), "quantity": "1" } ] });
    let (_, err) = call(&app, Method::POST, "/api/catalog/groups", Some(bad)).await;
    assert_eq!(err["fields"], json!({ "members.1.itemId": "duplicate" }));
    let missing = json!({ "name": "Missing", "members": [
        { "itemId": "00000000-0000-0000-0000-000000000001", "quantity": "1" } ] });
    let (_, err) = call(&app, Method::POST, "/api/catalog/groups", Some(missing)).await;
    assert_eq!(err["fields"], json!({ "members.0.itemId": "invalid" }));
    let (_, err) = call(
        &app,
        Method::POST,
        "/api/catalog/groups",
        Some(json!({ "name": "Empty" })),
    )
    .await;
    assert_eq!(err["fields"], json!({ "members": "required" }));

    let uri = format!("/api/catalog/groups/{}", id(&g));
    let (status, updated) = call(
        &app,
        Method::PUT,
        &uri,
        Some(
            json!({ "name": "Instalace+", "collapse": false, "members": [
            { "itemId": id(&a), "quantity": "3" }, { "itemId": id(&b), "quantity": "1" } ] }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(updated["collapse"], false);
    assert_eq!(updated["members"][0]["itemId"], a["id"]);

    // A rate change may not make the group mixed.
    let (status, err) = call(
        &app,
        Method::PUT,
        &format!("/api/catalog/items/{}", id(&b)),
        Some(json!({ "name": "Materiál", "unitPrice": "200", "vatRate": "12" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(err["fields"], json!({ "vatRate": "mixed_vat" }));

    // Deleting an item removes it from its groups.
    let (status, _) = call(
        &app,
        Method::DELETE,
        &format!("/api/catalog/items/{}", id(&a)),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, after) = call(&app, Method::GET, &uri, None).await;
    let members = after["members"].as_array().expect("members");
    assert_eq!(members.len(), 1);
    assert_eq!(
        (&members[0]["itemId"], &members[0]["position"]),
        (&b["id"], &json!(1))
    );

    // The last member cannot be deleted; a lone item may change its rate.
    let b_uri = format!("/api/catalog/items/{}", id(&b));
    let (status, err) = call(&app, Method::DELETE, &b_uri, None).await;
    assert_eq!(
        (status, err),
        (
            StatusCode::CONFLICT,
            json!({ "code": "catalog_item_in_use" })
        )
    );
    let (status, _) = call(
        &app,
        Method::PUT,
        &b_uri,
        Some(json!({ "name": "Materiál", "unitPrice": "200", "vatRate": "12" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, list) = call(&app, Method::GET, "/api/catalog/groups?q=instal", None).await;
    assert_eq!(list.as_array().map(Vec::len), Some(1));
    let (status, _) = call(&app, Method::DELETE, &uri, None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(&app, Method::GET, &uri, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = call(&app, Method::DELETE, &b_uri, None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(
        &app,
        Method::PUT,
        &uri,
        Some(json!({ "name": "x", "members": [
        { "itemId": id(&b), "quantity": "1" } ] })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
