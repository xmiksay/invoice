mod common;

use std::time::Duration;

use axum::http::{StatusCode, header};
use common::{TEST_TOKEN, TestDb, get, json, router, send};
use sea_orm::{ConnectOptions, Database};
use serde_json::json;

#[tokio::test]
async fn health_ok() {
    let db = TestDb::new().await;
    let resp = send(router(db.conn.clone()), get("/api/health", None)).await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        json(resp).await,
        json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") })
    );
}

#[tokio::test]
async fn health_degraded_when_db_unreachable() {
    // Lazy pool against a closed port: the app builds, the ping fails.
    let mut opts = ConnectOptions::new("postgres://invoice:invoice@127.0.0.1:1/invoice");
    opts.connect_lazy(true)
        .acquire_timeout(Duration::from_millis(500))
        .connect_timeout(Duration::from_millis(500));
    let conn = Database::connect(opts).await.expect("lazy connect");

    let resp = send(router(conn), get("/api/health", None)).await;
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(json(resp).await, json!({ "status": "degraded" }));
}

#[tokio::test]
async fn openapi_is_public() {
    let db = TestDb::new().await;
    let resp = send(router(db.conn.clone()), get("/api/openapi.json", None)).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let doc = json(resp).await;
    assert!(doc["paths"]["/api/health"].is_object());
    assert!(doc["paths"]["/api/auth/me"].is_object());
}

#[tokio::test]
async fn me_without_token_is_401() {
    let db = TestDb::new().await;
    let resp = send(router(db.conn.clone()), get("/api/auth/me", None)).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        resp.headers()
            .get(header::WWW_AUTHENTICATE)
            .and_then(|v| v.to_str().ok()),
        Some("Bearer")
    );
    assert_eq!(json(resp).await, json!({ "code": "unauthorized" }));
}

#[tokio::test]
async fn me_with_wrong_token_is_401() {
    let db = TestDb::new().await;
    let resp = send(
        router(db.conn.clone()),
        get("/api/auth/me", Some("wrong-token")),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(json(resp).await, json!({ "code": "unauthorized" }));
}

#[tokio::test]
async fn me_with_right_token_is_200() {
    let db = TestDb::new().await;
    let resp = send(
        router(db.conn.clone()),
        get("/api/auth/me", Some(TEST_TOKEN)),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let me = json(resp).await;
    assert_eq!(me["user"]["email"], "owner@example.com");
    assert_eq!(
        me["space"],
        json!({ "slug": "acme", "name": "Space acme", "role": "owner" })
    );
}

#[tokio::test]
async fn unknown_api_path_is_404_json() {
    let db = TestDb::new().await;
    let resp = send(
        router(db.conn.clone()),
        get("/api/does-not-exist", Some(TEST_TOKEN)),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    assert_eq!(json(resp).await, json!({ "code": "not_found" }));
}

#[tokio::test]
async fn unknown_api_path_is_404_without_token_too() {
    let db = TestDb::new().await;
    let resp = send(router(db.conn.clone()), get("/api/does-not-exist", None)).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn business_route_without_token_is_401() {
    let db = TestDb::new().await;
    let resp = send(router(db.conn.clone()), get("/api/contacts", None)).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn spa_fallback_is_not_behind_auth() {
    let db = TestDb::new().await;
    let resp = send(router(db.conn.clone()), get("/invoices/42", None)).await;
    let status = resp.status();
    assert_ne!(status, StatusCode::UNAUTHORIZED);
    // 200 with index.html once the frontend is built, 503 before that.
    assert!(
        status == StatusCode::OK || status == StatusCode::SERVICE_UNAVAILABLE,
        "unexpected SPA status {status}"
    );
    if status == StatusCode::OK {
        assert!(
            resp.headers()
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .is_some_and(|ct| ct.starts_with("text/html"))
        );
    }
}
