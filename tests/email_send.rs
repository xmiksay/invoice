//! Sending a document by e-mail: prefill, send through the mock SMTP server,
//! the log, `sentAt`, SMTP failures, state and validation rules.

mod common;

use axum::http::{Method, StatusCode};
use common::documents::{create_doc, create_issued, get_doc, id};
use common::mdcast::get_raw;
use common::received::create_received;
use common::smtp::{
    COMPANY_EMAIL, CONTACT_EMAIL, EmailEnv, FROM, history, issuable_with_emails, post_email,
    send_body,
};
use common::{TestDb, call};
use serde_json::{Value, json};

#[tokio::test]
async fn prefill_then_send_with_both_attachments() {
    let db = TestDb::new().await;
    let env = EmailEnv::new().await;
    let app = env.router(db.conn.clone());
    let doc = create_issued(&app, issuable_with_emails(&app).await).await;
    let doc_id = id(&doc);
    let number = doc["number"].as_str().expect("number").to_string();

    let (status, pre) = call(
        &app,
        Method::GET,
        &format!("/api/documents/{doc_id}/email"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{pre}");
    assert_eq!(pre["configured"], true);
    assert_eq!(pre["locale"], "cs");
    assert_eq!(pre["to"], json!([CONTACT_EMAIL]));
    assert_eq!(
        (&pre["cc"], &pre["bcc"]),
        (&json!([]), &json!([COMPANY_EMAIL]))
    );
    assert_eq!(
        pre["subject"],
        format!("Faktura – daňový doklad {number} – Dodavatel s.r.o.")
    );
    let body = pre["body"].as_str().expect("body");
    assert!(body.starts_with("Dobrý den,"), "{body}");
    assert!(body.contains("na částku 1\u{a0}210,00\u{a0}Kč"), "{body}");
    assert!(
        body.contains(&format!("pod variabilním symbolem {number}")),
        "{body}"
    );
    assert!(body.ends_with("Dodavatel s.r.o."), "{body}");
    assert_eq!(
        pre["attachments"],
        json!({ "pdf": { "available": true, "filename": format!("{number}.pdf") },
                "isdoc": { "available": true, "filename": format!("{number}.isdoc") } })
    );
    let (_, en) = call(
        &app,
        Method::GET,
        &format!("/api/documents/{doc_id}/email?locale=en"),
        None,
    )
    .await;
    assert!(
        en["subject"]
            .as_str()
            .expect("subject")
            .starts_with("Invoice – tax document")
    );
    let (status, e) = call(
        &app,
        Method::GET,
        &format!("/api/documents/{doc_id}/email?locale=de"),
        None,
    )
    .await;
    assert_eq!(
        (status, &e["fields"]["locale"]),
        (StatusCode::UNPROCESSABLE_ENTITY, &json!("invalid"))
    );

    let (status, entry) = post_email(
        &app,
        &doc_id,
        json!({
            "to": [format!("Odběratel <{CONTACT_EMAIL}>")], "cc": ["kopie@odberatel.cz"],
            "bcc": ["archiv@dodavatel.cz"], "subject": pre["subject"], "body": pre["body"],
            "attachPdf": true, "attachIsdoc": true
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{entry}");
    assert_eq!(entry["ok"], true);
    assert_eq!(entry["error"], Value::Null);
    assert_eq!(entry["to"], json!([format!("Odběratel <{CONTACT_EMAIL}>")]));
    assert_eq!(entry["bcc"], json!(["archiv@dodavatel.cz"]));
    assert_eq!(
        entry["attachments"],
        json!([format!("{number}.pdf"), format!("{number}.isdoc")])
    );
    let message_id = entry["messageId"].as_str().expect("message id");
    assert!(message_id.ends_with("@dodavatel.cz>"), "{message_id}");

    let mail = env.smtp.last();
    assert_eq!(mail.mail_from, "<faktury@dodavatel.cz>");
    assert_eq!(
        mail.rcpt,
        [
            format!("<{CONTACT_EMAIL}>"),
            "<kopie@odberatel.cz>".into(),
            "<archiv@dodavatel.cz>".into()
        ]
    );
    assert_eq!(mail.header("From").as_deref(), Some(FROM));
    assert_eq!(mail.header("Reply-To").as_deref(), Some(COMPANY_EMAIL));
    assert!(mail.header("To").expect("To").contains(CONTACT_EMAIL));
    assert_eq!(mail.header("Cc").as_deref(), Some("kopie@odberatel.cz"));
    assert_eq!(mail.header("Bcc"), None);
    assert!(
        !mail.data.contains("archiv@dodavatel.cz"),
        "Bcc only in the envelope"
    );
    assert_eq!(mail.header("Message-ID").as_deref(), Some(message_id));
    assert_eq!(mail.header("MIME-Version").as_deref(), Some("1.0"));
    assert!(mail.header("Date").is_some());
    assert!(
        mail.header("Content-Type")
            .expect("type")
            .starts_with("multipart/mixed")
    );
    assert!(
        mail.data
            .contains("Content-Type: text/plain; charset=utf-8")
    );

    let (pdf_head, pdf) = mail.attachment(&format!("{number}.pdf")).expect("pdf part");
    assert!(
        pdf_head.contains("Content-Type: application/pdf"),
        "{pdf_head}"
    );
    assert!(
        pdf_head.contains("Content-Transfer-Encoding: base64"),
        "{pdf_head}"
    );
    let (_, _, archived) = get_raw(&app, &format!("/api/documents/{doc_id}/pdf")).await;
    assert_eq!(pdf, archived);
    let (isdoc_head, isdoc) = mail
        .attachment(&format!("{number}.isdoc"))
        .expect("isdoc part");
    assert!(
        isdoc_head.contains("Content-Type: application/xml"),
        "{isdoc_head}"
    );
    let xml = String::from_utf8(isdoc).expect("utf-8 xml");
    assert!(xml.contains("<Invoice"), "{xml}");
    assert!(
        !xml.contains("SupplementsList"),
        "plain XML, no PDF supplement"
    );

    assert!(get_doc(&app, &doc_id).await["sentAt"].is_string());
    let (status, log) = history(&app, &doc_id).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(log, json!([entry]));

    // Re-sending is allowed; the history is newest first.
    let (status, second) = post_email(&app, &doc_id, send_body(json!({}))).await;
    assert_eq!(status, StatusCode::OK, "{second}");
    let (_, log) = history(&app, &doc_id).await;
    assert_eq!(log.as_array().map(Vec::len), Some(2));
    assert_eq!(log[0]["id"], second["id"]);
    assert_eq!(env.smtp.last().attachment_names(), Vec::<String>::new());
}

#[tokio::test]
async fn smtp_rejection_is_logged_and_leaves_sent_at() {
    let db = TestDb::new().await;
    let env = EmailEnv::new().await;
    let app = env.router(db.conn.clone());
    let doc = create_issued(&app, issuable_with_emails(&app).await).await;
    env.smtp.reject_with("550 5.7.1 relay denied");

    let (status, e) = post_email(&app, &id(&doc), send_body(json!({}))).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{e}");
    assert_eq!(e["code"], "smtp_failed");
    let detail = e["detail"].as_str().expect("detail");
    assert!(
        detail.contains("550") && detail.contains("relay denied"),
        "{detail}"
    );

    let (_, log) = history(&app, &id(&doc)).await;
    assert_eq!(log.as_array().map(Vec::len), Some(1));
    assert_eq!(
        (&log[0]["ok"], &log[0]["error"]),
        (&json!(false), &json!(detail))
    );
    assert_eq!(log[0]["messageId"], Value::Null);
    assert_eq!(get_doc(&app, &id(&doc)).await["sentAt"], Value::Null);
    assert!(env.smtp.mails().is_empty());
}

#[tokio::test]
async fn not_configured_draft_received_and_unknown() {
    let db = TestDb::new().await;
    let env = EmailEnv::new().await;
    let app = env.router(db.conn.clone());
    let off = env.unconfigured(db.conn.clone());
    let base = issuable_with_emails(&app).await;
    let doc = create_issued(&app, base.clone()).await;

    let (status, pre) = call(
        &off,
        Method::GET,
        &format!("/api/documents/{}/email", id(&doc)),
        None,
    )
    .await;
    assert_eq!(
        (status, &pre["configured"]),
        (StatusCode::OK, &json!(false))
    );
    let (status, e) = post_email(&off, &id(&doc), send_body(json!({}))).await;
    assert_eq!(
        (status, &e["code"]),
        (
            StatusCode::SERVICE_UNAVAILABLE,
            &json!("smtp_not_configured")
        )
    );

    let draft = create_doc(&app, base.clone()).await;
    for (method, path) in [(Method::GET, "email"), (Method::POST, "email")] {
        let uri = format!("/api/documents/{}/{path}", id(&draft));
        let body = (method == Method::POST).then(|| send_body(json!({})));
        let (status, e) = call(&app, method, &uri, body).await;
        assert_eq!(
            (status, &e["code"]),
            (StatusCode::CONFLICT, &json!("invalid_state"))
        );
    }
    assert_eq!(
        history(&app, &id(&draft)).await,
        (StatusCode::OK, json!([]))
    );

    let received = create_received(
        &app,
        base["contactId"].as_str().expect("contact"),
        json!({}),
    )
    .await;
    let unknown = uuid::Uuid::new_v4().to_string();
    for doc_id in [id(&received), unknown] {
        let (status, _) = call(
            &app,
            Method::GET,
            &format!("/api/documents/{doc_id}/email"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (status, _) = post_email(&app, &doc_id, send_body(json!({}))).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(history(&app, &doc_id).await.0, StatusCode::NOT_FOUND);
    }
    assert!(env.smtp.mails().is_empty());
}
