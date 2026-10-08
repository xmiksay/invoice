//! Original PDF upload, replace, delete and download.

mod common;

use axum::http::{Method, StatusCode};
use common::documents::{create_contact, create_issued, id, issuable, set_company};
use common::mdcast::{PdfEnv, get_raw};
use common::received::{create_received, multipart, upload, upload_raw};
use common::{TestDb, call};
use serde_json::{Value, json};

const PDF: &[u8] = b"%PDF-1.4\n% synthetic test original\n%%EOF\n";

#[tokio::test]
async fn upload_replace_serve_delete() {
    let db = TestDb::new().await;
    let env = PdfEnv::new();
    let app = env.router(db.conn.clone(), None);
    set_company(&app, true).await;
    let s = create_contact(&app, json!({ "name": "Dodavatel Test s.r.o." })).await;
    let doc = create_received(&app, &s, json!({})).await;
    let doc_id = id(&doc);
    let uri = format!("/api/documents/{doc_id}/pdf");

    let (status, _, body) = get_raw(&app, &uri).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let err: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(err["code"], "pdf_missing");

    let (status, body) = upload(&app, &doc_id, PDF).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "{body}");
    let (_, d) = call(&app, Method::GET, &format!("/api/documents/{doc_id}"), None).await;
    assert_eq!(
        d["original"]["sha256"],
        json!(invoice::pdf::storage::sha256_hex(PDF))
    );
    assert_eq!(d["original"]["size"], json!(PDF.len()));
    assert_eq!(d["pdf"], Value::Null);
    let sha = invoice::pdf::storage::sha256_hex(PDF);
    let rel = format!("documents/2026/{doc_id}-original-{}.pdf", &sha[..8]);
    let dir = env.storage.path().join("documents/2026");
    let files = |prefix: String| {
        std::fs::read_dir(&dir)
            .map(|d| {
                d.filter_map(|e| e.ok())
                    .filter(|e| e.file_name().to_string_lossy().starts_with(&prefix))
                    .count()
            })
            .unwrap_or(0)
    };
    assert_eq!(
        std::fs::read(env.storage.path().join(&rel)).expect("stored"),
        PDF
    );
    let (status, headers, bytes) = get_raw(&app, &uri).await;
    assert_eq!((status, bytes.as_slice()), (StatusCode::OK, PDF));
    assert_eq!(
        headers["content-disposition"],
        "inline; filename=\"P20260001.pdf\""
    );
    let (_, list) = call(&app, Method::GET, "/api/documents?direction=received", None).await;
    assert_eq!(list["items"][0]["hasPdf"], true);

    // Replace.
    let other = b"%PDF-1.7\nreplacement\n";
    let (status, _) = upload(&app, &doc_id, other).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, _, bytes) = get_raw(&app, &uri).await;
    assert_eq!(bytes.as_slice(), other);
    assert_eq!(env.mock.count(), 0, "never rendered");
    // The replaced file is gone; a same-content re-upload keeps the one file.
    assert!(!env.storage.path().join(&rel).exists());
    assert_eq!(files(format!("{doc_id}-original")), 1);
    let (status, _) = upload(&app, &doc_id, other).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, _, bytes) = get_raw(&app, &uri).await;
    assert_eq!(bytes.as_slice(), other);
    assert_eq!(files(format!("{doc_id}-original")), 1);

    // Delete (idempotent).
    for _ in 0..2 {
        let (status, _) = call(
            &app,
            Method::DELETE,
            &format!("/api/documents/{doc_id}/original"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
    }
    assert_eq!(files(format!("{doc_id}-original")), 0);
    let (status, _, _) = get_raw(&app, &uri).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Deleting the document removes its original.
    upload(&app, &doc_id, PDF).await;
    let (status, _) = call(
        &app,
        Method::DELETE,
        &format!("/api/documents/{doc_id}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(!env.storage.path().join(&rel).exists());
}

#[tokio::test]
async fn rejects_bad_uploads() {
    let db = TestDb::new().await;
    let env = PdfEnv::new();
    let app = env.router(db.conn.clone(), None);
    set_company(&app, true).await;
    let s = create_contact(
        &app,
        json!({ "name": "Dodavatel Test s.r.o.", "ico": "27074358" }),
    )
    .await;
    let doc = id(&create_received(&app, &s, json!({})).await);

    let (status, e) = upload(&app, &doc, b"<html>not a pdf</html>").await;
    assert_eq!(
        (status, &e["fields"]["file"]),
        (StatusCode::UNPROCESSABLE_ENTITY, &json!("invalid"))
    );
    let (status, e) = upload_raw(&app, &doc, multipart("other", PDF)).await;
    assert_eq!(
        (status, &e["fields"]["file"]),
        (StatusCode::UNPROCESSABLE_ENTITY, &json!("required"))
    );
    // Just over 20 MiB: our own limit.
    let mut big = PDF.to_vec();
    big.resize(20 * 1024 * 1024 + 1, b' ');
    let (status, e) = upload(&app, &doc, &big).await;
    assert_eq!(
        (status, &e["code"]),
        (StatusCode::PAYLOAD_TOO_LARGE, &json!("too_large"))
    );
    // Far beyond the route's body limit.
    big.resize(22 * 1024 * 1024, b' ');
    let (status, e) = upload(&app, &doc, &big).await;
    assert_eq!(
        (status, &e["code"]),
        (StatusCode::PAYLOAD_TOO_LARGE, &json!("too_large"))
    );
    // Exactly 20 MiB is fine.
    big.truncate(20 * 1024 * 1024);
    let (status, _) = upload(&app, &doc, &big).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // A native document's PDF is its rendered archive.
    let body = issuable(&app).await;
    let native = create_issued(&app, body).await;
    let (status, e) = upload(&app, &id(&native), PDF).await;
    assert_eq!(
        (status, &e["code"]),
        (StatusCode::CONFLICT, &json!("invalid_state"))
    );
    let (status, _) = upload(&app, "00000000-0000-0000-0000-000000000000", PDF).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
