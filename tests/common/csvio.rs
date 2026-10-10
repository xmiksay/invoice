//! CSV / XLSX import requests and fixtures (fictitious parties only: our
//! company 44444443, counterparties 12345679 / 87654326).

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use serde_json::Value;

use super::{TEST_TOKEN, send};

const BOUNDARY: &str = "XcsvBoundary9pQ2mZ";

/// The header every fixture row is written under.
pub const HEADER: &str = "direction;doc_type;number;supplier_number;related_number;issue_date;\
tax_date;due_date;received_date;counterparty_name;counterparty_ico;counterparty_dic;currency;\
exchange_rate;vat_mode;base_21;vat_21;base_12;vat_12;base_0;rounding;total;paid_date;\
variable_symbol;vat_deductible;category;note";

/// A CSV file (UTF-8, CRLF) of the header and `rows`.
pub fn file(header: &str, rows: &[&str]) -> Vec<u8> {
    let mut s = header.to_string();
    for r in rows {
        s.push_str("\r\n");
        s.push_str(r);
    }
    s.push_str("\r\n");
    s.into_bytes()
}

/// `multipart/form-data` with one `file` part per entry of `files` and an
/// optional `options` text part.
pub fn form(files: &[(&str, &[u8])], options: Option<&str>) -> Vec<u8> {
    let mut body = Vec::new();
    for (name, bytes) in files {
        body.extend_from_slice(
            format!(
                "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; \
                 filename=\"{name}\"\r\nContent-Type: application/octet-stream\r\n\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(bytes);
        body.extend_from_slice(b"\r\n");
    }
    if let Some(o) = options {
        body.extend_from_slice(
            format!(
                "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"options\"\r\n\r\n{o}\r\n"
            )
            .as_bytes(),
        );
    }
    body.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());
    body
}

pub async fn post(app: &Router, uri: &str, body: Vec<u8>) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header("authorization", format!("Bearer {TEST_TOKEN}"))
        .header(
            "content-type",
            format!("multipart/form-data; boundary={BOUNDARY}"),
        )
        .body(Body::from(body))
        .expect("build request");
    let resp = send(app.clone(), req).await;
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1 << 24)
        .await
        .expect("read body");
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

pub async fn preview_raw(app: &Router, bytes: &[u8]) -> (StatusCode, Value) {
    post(
        app,
        "/api/import/csv/preview",
        form(&[("import.csv", bytes)], None),
    )
    .await
}

/// Preview; asserts 200 and returns `entries`.
pub async fn preview(app: &Router, bytes: &[u8]) -> Vec<Value> {
    let (status, body) = preview_raw(app, bytes).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["entries"].as_array().expect("entries").clone()
}

/// Confirm `selected`; asserts 200 and returns `results`.
pub async fn confirm(app: &Router, bytes: &[u8], selected: &[&str]) -> Vec<Value> {
    let o = serde_json::json!({ "selected": selected }).to_string();
    let (status, body) = post(
        app,
        "/api/import/csv/confirm",
        form(&[("import.csv", bytes)], Some(&o)),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["results"].as_array().expect("results").clone()
}

pub fn by_key<'a>(rows: &'a [Value], key: &str) -> &'a Value {
    rows.iter()
        .find(|r| r["key"] == key)
        .unwrap_or_else(|| panic!("no entry {key} in {rows:?}"))
}

pub fn doc_id(results: &[Value], key: &str) -> String {
    let r = by_key(results, key);
    assert_eq!(r["status"], "imported", "{r}");
    r["documentId"].as_str().expect("id").to_string()
}

/// One row under [`HEADER`] from `column → value` pairs (others empty).
pub fn row(pairs: &[(&str, &str)]) -> String {
    row_in(HEADER, pairs)
}

/// One row under `header` from `column → value` pairs (others empty).
pub fn row_in(header: &str, pairs: &[(&str, &str)]) -> String {
    header
        .split(';')
        .map(|h| pairs.iter().find(|(k, _)| *k == h).map_or("", |(_, v)| *v))
        .collect::<Vec<_>>()
        .join(";")
}
