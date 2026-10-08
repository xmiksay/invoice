//! Fixtures for received / imported documents, categories, custom fields and
//! multipart uploads. Synthetic data only.

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use serde_json::{Value, json};

use super::{TEST_TOKEN, call, send};

/// A received invoice body for `contact`; `extra` overrides fields.
pub fn received_body(contact: &str, extra: Value) -> Value {
    let mut b = json!({
        "direction": "received", "docType": "invoice", "supplierNumber": "FV-2026-777",
        "contactId": contact, "issueDate": "2026-10-01", "taxPointDate": "2026-10-01",
        "dueDate": "2026-10-15", "vatRecap": [{ "rate": "21", "base": "1000", "vat": "210" }],
        "payable": "1210"
    });
    if let (Some(o), Some(e)) = (b.as_object_mut(), extra.as_object()) {
        o.extend(e.clone());
    }
    b
}

pub async fn create_received(app: &Router, contact: &str, extra: Value) -> Value {
    let (status, doc) = call(
        app,
        Method::POST,
        "/api/documents",
        Some(received_body(contact, extra)),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{doc}");
    doc
}

pub async fn create_category(app: &Router, name: &str, kind: &str) -> String {
    let (status, c) = call(
        app,
        Method::POST,
        "/api/settings/categories",
        Some(json!({ "name": name, "kind": kind })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{c}");
    c["id"].as_str().expect("category id").to_string()
}

pub async fn create_field(app: &Router, body: Value) -> Value {
    let (status, f) = call(app, Method::POST, "/api/settings/custom-fields", Some(body)).await;
    assert_eq!(status, StatusCode::CREATED, "{f}");
    f
}

pub const BOUNDARY: &str = "XtestBoundary7MA4YWxkTrZu0gW";

/// `multipart/form-data` body with one part named `name`.
pub fn multipart(name: &str, bytes: &[u8]) -> Vec<u8> {
    let mut body = format!(
        "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{name}\"; \
         filename=\"doc.pdf\"\r\nContent-Type: application/pdf\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
    body
}

/// `PUT /api/documents/{id}/original` with a raw multipart body.
pub async fn upload_raw(app: &Router, doc_id: &str, body: Vec<u8>) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(Method::PUT)
        .uri(format!("/api/documents/{doc_id}/original"))
        .header("authorization", format!("Bearer {TEST_TOKEN}"))
        .header(
            "content-type",
            format!("multipart/form-data; boundary={BOUNDARY}"),
        )
        .body(Body::from(body))
        .expect("build request");
    let resp = send(app.clone(), req).await;
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20)
        .await
        .expect("read body");
    let json = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).expect("JSON body")
    };
    (status, json)
}

pub async fn upload(app: &Router, doc_id: &str, bytes: &[u8]) -> (StatusCode, Value) {
    upload_raw(app, doc_id, multipart("file", bytes)).await
}
