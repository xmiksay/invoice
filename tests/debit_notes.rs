//! Debit notes: creation from an invoice / simplified document, issue rules,
//! the credit cap they raise, cancel guard and PDF.

mod common;

use axum::http::{Method, StatusCode};
use common::documents::{
    create_contact, create_doc, create_issued, get_doc, id, issuable, issue, item, post_action,
    set_company,
};
use common::mdcast::PdfEnv;
use common::{TestDb, call, router};
use serde_json::{Value, json};

async fn correction(
    app: &axum::Router,
    kind: &str,
    original: &str,
    body: Value,
) -> (StatusCode, Value) {
    call(
        app,
        Method::POST,
        &format!("/api/documents/{original}/{kind}"),
        Some(body),
    )
    .await
}

/// Create a note of `kind`, give it `lines` and a reason, issue it.
async fn issued_note(app: &axum::Router, kind: &str, original: &str, lines: Value) -> Value {
    let (status, draft) =
        correction(app, kind, original, json!({ "correctionReason": "Oprava" })).await;
    assert_eq!(status, StatusCode::CREATED, "{draft}");
    let mut put = draft.clone();
    put["lines"] = lines;
    let uri = format!("/api/documents/{}", id(&draft));
    let (status, saved) = call(app, Method::PUT, &uri, Some(put)).await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    let (status, issued) = issue(app, &id(&draft)).await;
    assert_eq!(status, StatusCode::OK, "{issued}");
    issued
}

#[tokio::test]
async fn creates_an_empty_draft_and_issues_it() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let body = issuable(&app).await;
    let invoice = create_issued(&app, body.clone()).await;

    let (status, dn) = correction(&app, "debit-note", &id(&invoice), json!({})).await;
    assert_eq!(status, StatusCode::CREATED, "{dn}");
    assert_eq!(
        (&dn["docType"], &dn["status"], &dn["sign"]),
        (&json!("debit_note"), &json!("draft"), &json!(1))
    );
    assert_eq!(dn["lines"], json!([]));
    assert_eq!(dn["relatedDocumentId"], json!(id(&invoice)));
    assert_eq!(dn["taxPointDate"], dn["issueDate"]);
    for f in [
        "contactId",
        "currency",
        "locale",
        "vatMode",
        "bankAccountId",
        "paymentMethod",
    ] {
        assert_eq!(dn[f], invoice[f], "{f}");
    }
    let (status, err) = issue(&app, &id(&dn)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        err["fields"],
        json!({ "lines": "required", "correctionReason": "required" })
    );

    // Bound to the invoice; no advance lines.
    let uri = format!("/api/documents/{}", id(&dn));
    let mut put = dn.clone();
    put["vatMode"] = json!("exempt");
    put["lines"] = json!([{ "kind": "advance", "advanceDocumentId": id(&invoice) }]);
    let (status, err) = call(&app, Method::PUT, &uri, Some(put)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(err["fields"]["vatMode"], "invalid");
    let mut put = dn.clone();
    put["lines"] = json!([{ "kind": "advance", "advanceDocumentId": id(&invoice) }]);
    let (_, err) = call(&app, Method::PUT, &uri, Some(put)).await;
    assert_eq!(err["fields"], json!({ "lines.0.kind": "invalid" }));

    // No cap: a debit note may exceed the invoice.
    let issued = issued_note(
        &app,
        "debit-note",
        &id(&invoice),
        json!([item("50", "100", "21")]),
    )
    .await;
    let number = issued["number"].as_str().expect("number");
    assert!(
        number.starts_with('V') && number.ends_with("0001"),
        "{number}"
    );
    assert_eq!(issued["totals"]["payable"], "6050.00");
    assert_eq!(issued["paymentState"], "unpaid");
    let inv = get_doc(&app, &id(&invoice)).await;
    assert_eq!(inv["relatedDocuments"][0]["docType"], "debit_note");
    // The empty draft and the issued one.
    let (_, list) = call(&app, Method::GET, "/api/documents?docType=debit_note", None).await;
    assert_eq!(
        (list["total"].clone(), list["items"][0]["sign"].clone()),
        (json!(2), json!(1))
    );

    // Neither a debit note nor drafts / proformas can be corrected.
    let draft = create_doc(&app, body.clone()).await;
    let mut pbody = body;
    pbody["docType"] = json!("proforma");
    let proforma = create_issued(&app, pbody).await;
    for doc in [&issued, &draft, &proforma] {
        for kind in ["debit-note", "credit-note"] {
            let (status, err) = correction(&app, kind, &id(doc), json!({})).await;
            assert_eq!(
                (status, err),
                (StatusCode::CONFLICT, json!({ "code": "invalid_state" })),
                "{kind} on {}",
                doc["docType"]
            );
        }
    }
    let (status, err) = correction(
        &app,
        "debit-note",
        &id(&invoice),
        json!({ "correctionReason": "x".repeat(501) }),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(err["fields"], json!({ "correctionReason": "too_long" }));
}

#[tokio::test]
async fn debit_notes_raise_the_credit_cap_and_guard_their_cancel() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let body = issuable(&app).await;
    let invoice = create_issued(&app, body).await; // 1000 @ 21 %
    let inv = id(&invoice);

    // 1200 @ 21 % exceeds the invoice alone.
    let (_, cn) = correction(
        &app,
        "credit-note",
        &inv,
        json!({ "correctionReason": "Vratka" }),
    )
    .await;
    let uri = format!("/api/documents/{}", id(&cn));
    let mut put = cn.clone();
    put["lines"] = json!([item("12", "100", "21"), item("1", "50", "12")]);
    call(&app, Method::PUT, &uri, Some(put)).await;
    let (status, err) = issue(&app, &id(&cn)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(err["fields"], json!({ "lines": "exceeds_original" }));

    // Debit notes add 200 @ 21 % and a new rate (50 @ 12 %): now it fits.
    let d1 = issued_note(&app, "debit-note", &inv, json!([item("2", "100", "21")])).await;
    let d2 = issued_note(&app, "debit-note", &inv, json!([item("1", "50", "12")])).await;
    let (status, credited) = issue(&app, &id(&cn)).await;
    assert_eq!(status, StatusCode::OK, "{credited}");

    // Cancelling a debit note would leave the credit note over the cap.
    for d in [&d1, &d2] {
        let (status, err) = post_action(&app, &id(d), "cancel").await;
        assert_eq!(
            (status, err),
            (StatusCode::CONFLICT, json!({ "code": "exceeds_original" }))
        );
    }
    let (status, _) = post_action(&app, &id(&credited), "cancel").await;
    assert_eq!(status, StatusCode::OK);
    let (status, c) = post_action(&app, &id(&d1), "cancel").await;
    assert_eq!(
        (status, &c["status"]),
        (StatusCode::OK, &json!("cancelled"))
    );
}

#[tokio::test]
async fn pdf_title_reference_and_qr() {
    let db = TestDb::new().await;
    let env = PdfEnv::new();
    let app = env.router(db.conn.clone(), None);
    set_company(&app, true).await;
    let contact = create_contact(&app, json!({})).await;
    let (status, bank) = call(
        &app,
        Method::POST,
        "/api/settings/bank-accounts",
        Some(json!({ "currency": "CZK", "iban": "CZ6508000000192000145399" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{bank}");
    let invoice = create_issued(
        &app,
        json!({ "contactId": contact, "bankAccountId": bank["id"], "lines": [item("1", "100", "21")] }),
    )
    .await;
    issued_note(
        &app,
        "debit-note",
        &id(&invoice),
        json!([item("1", "10", "21")]),
    )
    .await;
    let data = env.mock.last().data;
    assert_eq!(data["docType"], "debit_note");
    assert_eq!(data["title"], "Opravný daňový doklad – vrubopis");
    let number = invoice["number"].as_str().expect("number");
    assert_eq!(
        data["reference"],
        format!("Opravný daňový doklad – vrubopis k faktuře {number}\nDůvod opravy: Oprava")
    );
    assert_eq!(data["qr"]["image"], "qr.svg", "a debit note is payable");
}

#[tokio::test]
async fn issue_rechecks_the_original_and_imports_count_in_the_cap() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let base = issuable(&app).await;
    let invoice = create_issued(&app, base.clone()).await; // 1000 @ 21 %
    let inv = id(&invoice);

    // The original cancelled after the draft was made → 409 at issue.
    let (_, dn) = correction(&app, "debit-note", &inv, json!({ "correctionReason": "x" })).await;
    let mut put = dn.clone();
    put["lines"] = json!([item("1", "100", "21")]);
    let uri = format!("/api/documents/{}", id(&dn));
    call(&app, Method::PUT, &uri, Some(put)).await;
    let other = create_issued(&app, base.clone()).await;
    let (status, _) = post_action(&app, &inv, "cancel").await;
    assert_eq!(status, StatusCode::OK);
    let (status, err) = issue(&app, &id(&dn)).await;
    assert_eq!(
        (status, err),
        (StatusCode::CONFLICT, json!({ "code": "invalid_state" }))
    );

    // An imported debit note raises the cap: cancelling it is re-checked.
    let mut imp = base;
    imp["imported"] = json!(true);
    imp["docType"] = json!("debit_note");
    imp["number"] = json!("V-OLD-9");
    imp["relatedDocumentId"] = json!(id(&other));
    imp["lines"] = json!([item("2", "100", "21")]);
    let imp = create_doc(&app, imp).await;
    let (status, imp) = issue(&app, &id(&imp)).await;
    assert_eq!(status, StatusCode::OK, "{imp}");
    issued_note(
        &app,
        "credit-note",
        &id(&other),
        json!([item("12", "100", "21")]),
    )
    .await;
    let (status, err) = post_action(&app, &id(&imp), "cancel").await;
    assert_eq!(
        (status, err),
        (StatusCode::CONFLICT, json!({ "code": "exceeds_original" }))
    );
}
