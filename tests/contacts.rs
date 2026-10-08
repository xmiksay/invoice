//! Contacts CRUD, search and IČO uniqueness.

mod common;

use axum::Router;
use axum::http::{Method, StatusCode};
use common::{TestDb, call, router};
use serde_json::{Value, json};

async fn create(app: &Router, body: Value) -> Value {
    let (status, c) = call(app, Method::POST, "/api/contacts", Some(body)).await;
    assert_eq!(status, StatusCode::CREATED, "{c}");
    c
}

fn names(list: &Value) -> Vec<&str> {
    list["items"]
        .as_array()
        .map(|items| items.iter().filter_map(|c| c["name"].as_str()).collect())
        .unwrap_or_default()
}

#[tokio::test]
async fn contact_crud() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());

    let created = create(
        &app,
        json!({
            "name": " ACME s.r.o. ",
            "ico": "27074358",
            "dic": "cz27074358",
            "street": "Dlouhá 1",
            "city": "Praha",
            "zip": "11000",
            "email": "info@acme.example",
            "defaultDueDays": 30,
            "defaultLocale": "en",
            "defaultCurrency": "eur"
        }),
    )
    .await;
    assert_eq!(created["name"], "ACME s.r.o.");
    assert_eq!(created["country"], "CZ");
    assert_eq!(created["dic"], "CZ27074358");
    assert_eq!(created["defaultCurrency"], "EUR");
    assert_eq!(created["note"], Value::Null);
    assert!(created["createdAt"].is_string());
    let id = created["id"].as_str().expect("id").to_string();
    let uri = format!("/api/contacts/{id}");

    let (status, got) = call(&app, Method::GET, &uri, None).await;
    assert_eq!((status, &got), (StatusCode::OK, &created));

    let (status, updated) = call(
        &app,
        Method::PUT,
        &uri,
        Some(json!({ "name": "ACME a.s.", "country": "sk", "note": "VIP" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(updated["name"], "ACME a.s.");
    assert_eq!(updated["country"], "SK");
    assert_eq!(updated["ico"], Value::Null, "PUT replaces every field");
    assert_eq!(updated["note"], "VIP");
    assert_eq!(updated["createdAt"], created["createdAt"]);

    let (status, body) = call(&app, Method::DELETE, &uri, None).await;
    assert_eq!((status, body), (StatusCode::NO_CONTENT, Value::Null));
    let (status, err) = call(&app, Method::GET, &uri, None).await;
    assert_eq!(
        (status, err),
        (StatusCode::NOT_FOUND, json!({ "code": "not_found" }))
    );
    let (status, _) = call(&app, Method::DELETE, &uri, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = call(&app, Method::GET, "/api/contacts/not-a-uuid", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = call(&app, Method::PUT, &uri, Some(json!({ "name": "X" }))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn contact_validation_is_422() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let (status, err) = call(
        &app,
        Method::POST,
        "/api/contacts",
        Some(json!({
            "ico": "1234",
            "dic": "1234",
            "country": "CZE",
            "defaultDueDays": -1,
            "defaultLocale": "de",
            "email": "x"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        err,
        json!({
            "code": "validation",
            "fields": {
                "name": "required",
                "ico": "invalid_ico",
                "dic": "invalid",
                "country": "invalid",
                "defaultDueDays": "invalid",
                "defaultLocale": "invalid",
                "email": "invalid"
            }
        })
    );
}

#[tokio::test]
async fn duplicate_ico_is_422_and_null_ico_is_not_unique() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());

    let a = create(&app, json!({ "name": "A", "ico": "27074358" })).await;
    create(&app, json!({ "name": "B" })).await;
    let c = create(&app, json!({ "name": "C", "ico": "" })).await;
    assert_eq!(c["ico"], Value::Null);

    let dup = json!({ "code": "validation", "fields": { "ico": "duplicate" } });
    let (status, err) = call(
        &app,
        Method::POST,
        "/api/contacts",
        Some(json!({ "name": "A2", "ico": "270 74 358" })),
    )
    .await;
    assert_eq!(
        (status, err),
        (StatusCode::UNPROCESSABLE_ENTITY, dup.clone())
    );

    let c_uri = format!("/api/contacts/{}", c["id"].as_str().unwrap_or_default());
    let (status, err) = call(
        &app,
        Method::PUT,
        &c_uri,
        Some(json!({ "name": "C", "ico": "27074358" })),
    )
    .await;
    assert_eq!((status, err), (StatusCode::UNPROCESSABLE_ENTITY, dup));

    // Re-saving a contact with its own IČO is fine.
    let a_uri = format!("/api/contacts/{}", a["id"].as_str().unwrap_or_default());
    let (status, _) = call(
        &app,
        Method::PUT,
        &a_uri,
        Some(json!({ "name": "A", "ico": "27074358" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn search_and_pagination() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());

    create(
        &app,
        json!({ "name": "Žluťoučký kůň s.r.o.", "city": "Brno" }),
    )
    .await;
    create(
        &app,
        json!({ "name": "Alfa", "ico": "12345679", "city": "Praha" }),
    )
    .await;
    create(
        &app,
        json!({ "name": "beta", "dic": "CZ99887766", "city": "Ostrava" }),
    )
    .await;
    create(&app, json!({ "name": "Gama 100%", "city": "praha" })).await;

    let (status, all) = call(&app, Method::GET, "/api/contacts", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(all["total"], 4);
    assert_eq!(names(&all).len(), 4);

    for (q, expected) in [
        ("PRAHA", vec!["Alfa", "Gama 100%"]),
        ("alf", vec!["Alfa"]),
        ("2345", vec!["Alfa"]),
        ("cz9988", vec!["beta"]),
        ("ŽLUŤ", vec!["Žluťoučký kůň s.r.o."]),
        ("100%", vec!["Gama 100%"]),
        ("%", vec!["Gama 100%"]),
        ("_", Vec::new()),
        ("nothing", Vec::new()),
    ] {
        let uri = format!("/api/contacts?q={}", urlencode(q));
        let (status, list) = call(&app, Method::GET, &uri, None).await;
        assert_eq!(status, StatusCode::OK, "{q}");
        assert_eq!(names(&list), expected, "{q}");
        assert_eq!(list["total"], expected.len(), "{q}");
    }

    let (_, page) = call(&app, Method::GET, "/api/contacts?limit=2&offset=1", None).await;
    assert_eq!(page["total"], 4);
    assert_eq!(page["items"].as_array().map(Vec::len), Some(2));

    let (_, page) = call(&app, Method::GET, "/api/contacts?q=praha&limit=1", None).await;
    assert_eq!(page["total"], 2);
    assert_eq!(names(&page).len(), 1);

    let (status, page) = call(
        &app,
        Method::GET,
        &format!("/api/contacts?offset={}", u64::MAX),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "huge offset is clamped, not a 500");
    assert_eq!(page, json!({ "items": [], "total": 4 }));
    let (status, _) = call(&app, Method::GET, "/api/contacts?offset=-1", None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, err) = call(&app, Method::GET, "/api/contacts?limit=abc", None).await;
    assert_eq!(
        (status, err),
        (StatusCode::BAD_REQUEST, json!({ "code": "bad_request" }))
    );
}

fn urlencode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}
