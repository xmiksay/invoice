//! ARES lookup against a local mock server — the real ARES is never called.

mod common;

use axum::Router;
use axum::extract::Path;
use axum::http::{Method, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use common::{TestDb, call, router_with_ares};
use serde_json::json;

const FULL: &str = "44444443";
const VILLAGE: &str = "12345679";
const MISSING: &str = "11111119";
const SERVER_ERROR: &str = "22222227";
const GARBAGE: &str = "33333335";

fn fixture(name: &str) -> String {
    let path = format!("{}/tests/fixtures/ares/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"))
}

fn json_response(status: StatusCode, body: String) -> Response {
    (status, [(header::CONTENT_TYPE, "application/json")], body).into_response()
}

async fn mock_subject(Path(ico): Path<String>) -> Response {
    match ico.as_str() {
        FULL | VILLAGE => json_response(StatusCode::OK, fixture(&format!("{ico}.json"))),
        SERVER_ERROR => (StatusCode::INTERNAL_SERVER_ERROR, "boom").into_response(),
        GARBAGE => json_response(StatusCode::OK, "<html>not json</html>".into()),
        _ => json_response(StatusCode::NOT_FOUND, fixture("not_found.json")),
    }
}

/// Serves `/rest/ekonomicke-subjekty/{ico}` on an ephemeral port; returns the base URL.
async fn mock_ares() -> String {
    let app = Router::new().route("/rest/ekonomicke-subjekty/{ico}", get(mock_subject));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock ARES");
    let addr = listener.local_addr().expect("mock address");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("mock ARES server");
    });
    format!("http://{addr}/rest/")
}

async fn app() -> (TestDb, Router) {
    let db = TestDb::new().await;
    let app = router_with_ares(db.conn.clone(), &mock_ares().await);
    (db, app)
}

#[tokio::test]
async fn maps_full_record_with_orientation_number() {
    let (_db, app) = app().await;
    let (status, body) = call(&app, Method::GET, &format!("/api/ares/{FULL}"), None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body,
        json!({
            "name": "Ukázková firma a.s.",
            "ico": FULL,
            "dic": "CZ44444443",
            "street": "Ukázková 778/3a",
            "city": "Praha",
            "zip": "14000",
            "country": "CZ"
        })
    );
}

#[tokio::test]
async fn maps_record_without_street() {
    let (_db, app) = app().await;
    let (status, body) = call(&app, Method::GET, &format!("/api/ares/{VILLAGE}"), None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["street"], "Dolní Lhota 12");
    assert_eq!(body["city"], "Lhota");
    assert_eq!(body["zip"], "01234", "PSČ keeps its leading zero");
    assert_eq!(body["dic"], serde_json::Value::Null);
}

#[tokio::test]
async fn ares_404_is_ares_not_found() {
    let (_db, app) = app().await;
    let (status, body) = call(&app, Method::GET, &format!("/api/ares/{MISSING}"), None).await;
    assert_eq!(
        (status, body),
        (StatusCode::NOT_FOUND, json!({ "code": "ares_not_found" }))
    );
}

#[tokio::test]
async fn ares_500_and_garbage_are_unavailable() {
    let (_db, app) = app().await;
    for ico in [SERVER_ERROR, GARBAGE] {
        let (status, body) = call(&app, Method::GET, &format!("/api/ares/{ico}"), None).await;
        assert_eq!(
            (status, body),
            (
                StatusCode::BAD_GATEWAY,
                json!({ "code": "ares_unavailable" })
            ),
            "{ico}"
        );
    }
}

#[tokio::test]
async fn unreachable_ares_is_unavailable() {
    let db = TestDb::new().await;
    // Bind then drop: the port is (almost certainly) closed afterwards.
    let addr = std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|l| l.local_addr())
        .expect("free port");
    let app = router_with_ares(db.conn.clone(), &format!("http://{addr}"));
    let (status, body) = call(&app, Method::GET, &format!("/api/ares/{FULL}"), None).await;
    assert_eq!(
        (status, body),
        (
            StatusCode::BAD_GATEWAY,
            json!({ "code": "ares_unavailable" })
        )
    );
}

#[tokio::test]
async fn invalid_ico_is_422_without_calling_ares() {
    let (_db, app) = app().await;
    for ico in ["12345678", "1234", "abcdefgh"] {
        let (status, body) = call(&app, Method::GET, &format!("/api/ares/{ico}"), None).await;
        assert_eq!(
            (status, body),
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                json!({ "code": "validation", "fields": { "ico": "invalid_ico" } })
            ),
            "{ico}"
        );
    }
}

#[tokio::test]
async fn requires_token() {
    let (_db, app) = app().await;
    let resp = common::send(app, common::get(&format!("/api/ares/{FULL}"), None)).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
