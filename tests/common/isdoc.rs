//! ISDOC fixtures, multipart import requests, zips and XSD validation.

use std::io::{Cursor, Read, Write as _};

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use serde_json::Value;

use super::{TEST_TOKEN, send};

const BOUNDARY: &str = "XisdocBoundary4kQ9wz";

pub fn fixture(name: &str) -> String {
    let path = format!("{}/tests/fixtures/isdoc/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"))
}

/// `multipart/form-data` with one `files` part per file and an optional
/// `options` text part.
pub fn form(files: &[(&str, &[u8])], options: Option<&str>) -> Vec<u8> {
    let mut body = Vec::new();
    for (name, bytes) in files {
        body.extend_from_slice(
            format!(
                "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"files\"; \
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

pub async fn post_form(app: &Router, uri: &str, body: Vec<u8>) -> (StatusCode, Value) {
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

/// Preview; asserts 200 and returns `entries`.
pub async fn preview(app: &Router, files: &[(&str, &[u8])]) -> Vec<Value> {
    let (status, body) = post_form(app, "/api/import/isdoc/preview", form(files, None)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["entries"].as_array().expect("entries").clone()
}

/// Confirm; asserts 200 and returns `results`.
pub async fn confirm(app: &Router, files: &[(&str, &[u8])], options: Value) -> Vec<Value> {
    let o = options.to_string();
    let (status, body) = post_form(app, "/api/import/isdoc/confirm", form(files, Some(&o))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["results"].as_array().expect("results").clone()
}

pub fn zip_of(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default();
    for (name, bytes) in entries {
        w.start_file(*name, opts).expect("start entry");
        w.write_all(bytes).expect("write entry");
    }
    w.finish().expect("finish zip").into_inner()
}

/// Every entry of a zip, by name.
pub fn unzip(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut z = zip::ZipArchive::new(Cursor::new(bytes)).expect("zip");
    (0..z.len())
        .map(|i| {
            let mut e = z.by_index(i).expect("entry");
            let name = e.name().expect("entry name").into_owned();
            let mut buf = Vec::new();
            e.read_to_end(&mut buf).expect("read entry");
            (name, buf)
        })
        .collect()
}

/// `xmllint --schema` against the vendored ISDOC schema (`schema` file name).
pub fn assert_valid(xml: &[u8], schema: &str) {
    super::assert_xsd(
        xml,
        &format!("isdoc/schema/{schema}"),
        &String::from_utf8_lossy(xml),
    );
}
