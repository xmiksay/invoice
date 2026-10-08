//! VAT rates: seed, CRUD, single-default invariant, duplicate rates.

mod common;

use axum::http::{Method, StatusCode};
use common::{TestDb, call, router};
use serde_json::{Value, json};

async fn list_rates(app: &axum::Router) -> Vec<Value> {
    let (status, list) = call(app, Method::GET, "/api/settings/vat-rates", None).await;
    assert_eq!(status, StatusCode::OK);
    list.as_array().expect("array").clone()
}

fn default_rates(list: &[Value]) -> Vec<Value> {
    list.iter()
        .filter(|r| r["isDefault"] == true)
        .map(|r| r["rate"].clone())
        .collect()
}

#[tokio::test]
async fn vat_rates_are_seeded() {
    let db = TestDb::new().await;
    let list = list_rates(&router(db.conn.clone())).await;
    let summary: Vec<_> = list
        .iter()
        .map(|r| {
            (
                r["rate"].clone(),
                r["label"].clone(),
                r["isDefault"].clone(),
                r["position"].clone(),
            )
        })
        .collect();
    assert_eq!(
        summary,
        vec![
            (json!("21"), json!("Základní"), json!(true), json!(1)),
            (json!("12"), json!("Snížená"), json!(false), json!(2)),
            (json!("0"), json!("Nulová"), json!(false), json!(3)),
        ]
    );
    assert!(list.iter().all(|r| r["active"] == true));
}

#[tokio::test]
async fn vat_rate_crud_keeps_exactly_one_default() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());

    let (status, created) = call(
        &app,
        Method::POST,
        "/api/settings/vat-rates",
        Some(json!({ "rate": "12.5", "label": "Test", "isDefault": true })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["rate"], "12.5");
    assert_eq!(created["position"], 4, "appended after the seeded rates");
    assert_eq!(created["active"], true);
    assert_eq!(default_rates(&list_rates(&app).await), vec![json!("12.5")]);

    // Un-flagging the default promotes the first other rate by position.
    let id = created["id"].as_str().unwrap_or_default().to_string();
    let uri = format!("/api/settings/vat-rates/{id}");
    let (status, updated) = call(
        &app,
        Method::PUT,
        &uri,
        Some(json!({ "rate": "15", "label": "Changed", "isDefault": false, "active": false, "position": 0 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(
        (&updated["rate"], &updated["active"], &updated["position"]),
        (&json!("15"), &json!(false), &json!(0))
    );
    let list = list_rates(&app).await;
    assert_eq!(list[0]["id"], json!(id), "ordered by position");
    assert_eq!(default_rates(&list), vec![json!("21")]);

    // Deleting the default promotes another one.
    let base_id = list
        .iter()
        .find(|r| r["rate"] == "21")
        .and_then(|r| r["id"].as_str())
        .unwrap_or_default()
        .to_string();
    let (status, _) = call(
        &app,
        Method::DELETE,
        &format!("/api/settings/vat-rates/{base_id}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let list = list_rates(&app).await;
    assert_eq!(list.len(), 3);
    assert_eq!(default_rates(&list).len(), 1);
}

#[tokio::test]
async fn vat_rate_validation_and_duplicates() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());

    for (rate, reason) in [
        ("", "required"),
        ("abc", "invalid"),
        ("101", "invalid"),
        ("1.234", "invalid"),
    ] {
        let (status, err) = call(
            &app,
            Method::POST,
            "/api/settings/vat-rates",
            Some(json!({ "rate": rate, "label": "X" })),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{rate}");
        assert_eq!(err["fields"], json!({ "rate": reason }), "{rate}");
    }

    let (status, err) = call(
        &app,
        Method::POST,
        "/api/settings/vat-rates",
        Some(json!({ "rate": "21.00", "label": "Dup" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        err,
        json!({ "code": "validation", "fields": { "rate": "duplicate" } })
    );

    let twelve = list_rates(&app)
        .await
        .into_iter()
        .find(|r| r["rate"] == "12")
        .and_then(|r| r["id"].as_str().map(str::to_string))
        .unwrap_or_default();
    let (status, err) = call(
        &app,
        Method::PUT,
        &format!("/api/settings/vat-rates/{twelve}"),
        Some(json!({ "rate": "0", "label": "Clash" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(err["fields"], json!({ "rate": "duplicate" }));
}

fn id_of(list: &[Value], rate: &str) -> String {
    list.iter()
        .find(|r| r["rate"] == rate)
        .and_then(|r| r["id"].as_str())
        .unwrap_or_default()
        .to_string()
}

#[tokio::test]
async fn default_rate_must_be_active() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let invalid_default = json!({ "code": "validation", "fields": { "isDefault": "invalid" } });

    let (status, err) = call(
        &app,
        Method::POST,
        "/api/settings/vat-rates",
        Some(json!({ "rate": "5", "label": "X", "isDefault": true, "active": false })),
    )
    .await;
    assert_eq!(
        (status, err),
        (StatusCode::UNPROCESSABLE_ENTITY, invalid_default.clone())
    );

    // Deactivating the default promotes the first other ACTIVE rate.
    let list = list_rates(&app).await;
    let (id21, id12, id0) = (id_of(&list, "21"), id_of(&list, "12"), id_of(&list, "0"));
    let put = |id: &str, body: Value| {
        let app = app.clone();
        let uri = format!("/api/settings/vat-rates/{id}");
        async move { call(&app, Method::PUT, &uri, Some(body)).await }
    };
    let (status, _) = put(
        &id12,
        json!({ "rate": "12", "label": "Snížená", "active": false }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = put(
        &id21,
        json!({ "rate": "21", "label": "Základní", "active": false }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["isDefault"], false);
    assert_eq!(default_rates(&list_rates(&app).await), vec![json!("0")]);

    // The default is now the only active rate: deactivating it is rejected.
    let (status, err) = put(
        &id0,
        json!({ "rate": "0", "label": "Nulová", "active": false }),
    )
    .await;
    assert_eq!(
        (status, err),
        (StatusCode::UNPROCESSABLE_ENTITY, invalid_default)
    );
    let list = list_rates(&app).await;
    assert_eq!(default_rates(&list), vec![json!("0")], "rolled back");
    assert!(list.iter().any(|r| r["rate"] == "0" && r["active"] == true));
}
