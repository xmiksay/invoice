//! Credit notes: creation from an invoice, copied rate, cap per rate, issue.

mod common;

use axum::http::{Method, StatusCode};
use common::documents::{
    create_bank, create_contact, create_doc, create_issued, get_doc, id, issuable, item,
    post_action,
};
use common::{TestDb, call, router};
use serde_json::{Value, json};

async fn credit_note(app: &axum::Router, invoice: &str, body: Value) -> (StatusCode, Value) {
    call(
        app,
        Method::POST,
        &format!("/api/documents/{invoice}/credit-note"),
        Some(body),
    )
    .await
}

fn text(description: &str) -> Value {
    json!({ "kind": "text", "description": description })
}

#[tokio::test]
async fn creates_a_draft_copy_of_the_invoice() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let mut body = issuable(&app).await;
    body["lines"] = json!([
        item("10", "100", "21"),
        text("Poznámka"),
        item("1", "500", "12"),
        { "kind": "subtotal", "description": "S", "refs": [1], "collapse": true },
    ]);
    let invoice = create_issued(&app, body.clone()).await;

    let (status, cn) = credit_note(
        &app,
        &id(&invoice),
        json!({ "correctionReason": " Reklamace " }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{cn}");
    assert_eq!(
        (&cn["docType"], &cn["status"]),
        (&json!("credit_note"), &json!("draft"))
    );
    assert_eq!(cn["sign"], -1);
    assert_eq!(cn["correctionReason"], "Reklamace");
    assert_eq!(cn["relatedDocumentId"], json!(id(&invoice)));
    assert_eq!(cn["taxPointDate"], cn["issueDate"]);
    assert_eq!(cn["exchangeRate"], Value::Null);
    for f in [
        "contactId",
        "currency",
        "locale",
        "vatMode",
        "bankAccountId",
        "paymentMethod",
    ] {
        assert_eq!(cn[f], invoice[f], "{f}");
    }
    assert_eq!(cn["lines"], invoice["lines"]);
    assert_eq!(cn["totals"]["payable"], "1770.00");

    let inv = get_doc(&app, &id(&invoice)).await;
    assert_eq!(inv["relatedDocuments"][0]["id"], cn["id"]);
    assert_eq!(inv["relatedDocuments"][0]["docType"], "credit_note");

    // Drafts, proformas and cancelled invoices cannot be credited.
    let draft = create_doc(&app, body.clone()).await;
    let mut pbody = body.clone();
    pbody["docType"] = json!("proforma");
    let proforma = create_issued(&app, pbody).await;
    let cancelled = create_issued(&app, body).await;
    post_action(&app, &id(&cancelled), "cancel").await;
    for doc in [&draft, &proforma, &cancelled, &cn] {
        let (status, err) = credit_note(&app, &id(doc), json!({})).await;
        assert_eq!(
            (status, err),
            (StatusCode::CONFLICT, json!({ "code": "invalid_state" }))
        );
    }
    let (status, err) = credit_note(
        &app,
        &id(&invoice),
        json!({ "correctionReason": "x".repeat(501) }),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(err["fields"], json!({ "correctionReason": "too_long" }));
}

#[tokio::test]
async fn copies_the_original_rate() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    common::documents::set_company(&app, true).await;
    let contact = create_contact(&app, json!({ "defaultDueDays": 30 })).await;
    let other = create_contact(&app, json!({ "name": "Jiný", "ico": "27074358" })).await;
    create_bank(&app, "EUR").await;
    let invoice = create_issued(
        &app,
        json!({ "contactId": contact, "currency": "EUR", "exchangeRate": "25.5",
                "issueDate": "2026-10-01", "lines": [item("1", "100", "21")] }),
    )
    .await;
    let (_, cn) = credit_note(&app, &id(&invoice), json!({ "correctionReason": "Sleva" })).await;
    assert_eq!(
        (&cn["exchangeRate"], &cn["exchangeRateSource"]),
        (&json!("25.5"), &json!("original"))
    );
    assert_eq!(cn["totals"]["totalCzk"], "3085.50");
    // Due days: the contact's default wins over the company's 14.
    let issue_date =
        chrono::NaiveDate::parse_from_str(cn["issueDate"].as_str().expect("date"), "%Y-%m-%d")
            .expect("date");
    let due = issue_date + chrono::Days::new(30);
    assert_eq!(cn["dueDate"], json!(due.format("%Y-%m-%d").to_string()));

    // The compute preview uses the invoice's rate too.
    let (status, c) = call(
        &app,
        Method::POST,
        "/api/documents/compute",
        Some(
            json!({ "documentId": id(&cn), "currency": "EUR", "exchangeRate": "30",
                     "lines": [item("1", "100", "21")] }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{c}");
    assert_eq!(c["totals"]["totalCzk"], "3085.50");

    let mut put = cn.clone();
    put["exchangeRate"] = json!("30");
    let uri = format!("/api/documents/{}", id(&cn));
    let (status, saved) = call(&app, Method::PUT, &uri, Some(put.clone())).await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(
        (&saved["exchangeRate"], &saved["exchangeRateSource"]),
        (&json!("25.5"), &json!("original"))
    );
    // Contact and VAT mode stay the invoice's.
    let mut bad = put.clone();
    bad["contactId"] = json!(other);
    bad["vatMode"] = json!("exempt");
    let (status, err) = call(&app, Method::PUT, &uri, Some(bad)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        err["fields"],
        json!({ "contactId": "invalid", "vatMode": "invalid" })
    );
    put["currency"] = json!("USD");
    let (status, err) = call(&app, Method::PUT, &uri, Some(put)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(err["fields"]["currency"], "invalid");

    let (status, issued) = post_action(&app, &id(&cn), "issue").await;
    assert_eq!(status, StatusCode::OK, "{issued}");
    assert_eq!(
        (&issued["exchangeRate"], &issued["exchangeRateSource"]),
        (&json!("25.5"), &json!("original"))
    );
}

#[tokio::test]
async fn issue_checks_reason_and_cap_per_rate() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let mut body = issuable(&app).await;
    body["lines"] = json!([item("10", "100", "21"), item("1", "500", "12")]);
    let invoice = create_issued(&app, body).await;

    // Missing reason.
    let (_, cn) = credit_note(&app, &id(&invoice), json!({})).await;
    let (status, err) = post_action(&app, &id(&cn), "issue").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(err["fields"], json!({ "correctionReason": "required" }));

    // Partial credit (600 @ 21 %) issues and is numbered from its series.
    let uri = format!("/api/documents/{}", id(&cn));
    let mut put = cn.clone();
    put["correctionReason"] = json!("Sleva");
    put["lines"] = json!([item("6", "100", "21")]);
    let (status, _) = call(&app, Method::PUT, &uri, Some(put)).await;
    assert_eq!(status, StatusCode::OK);
    let (status, first) = post_action(&app, &id(&cn), "issue").await;
    assert_eq!(status, StatusCode::OK, "{first}");
    let number = first["number"].as_str().expect("number");
    assert!(
        number.starts_with('D') && number.ends_with("0001"),
        "{number}"
    );
    assert_eq!(first["sign"], -1);
    assert_eq!(first["paymentState"], "unpaid");

    // The full copy would credit 1000 more @ 21 % → above the original's 1000.
    let (_, full) = credit_note(&app, &id(&invoice), json!({ "correctionReason": "Vratka" })).await;
    let (status, err) = post_action(&app, &id(&full), "issue").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(err["fields"], json!({ "lines": "exceeds_original" }));

    // A rate the invoice does not have.
    let uri = format!("/api/documents/{}", id(&full));
    let mut put = full.clone();
    put["lines"] = json!([item("1", "1", "0")]);
    call(&app, Method::PUT, &uri, Some(put.clone())).await;
    let (status, err) = post_action(&app, &id(&full), "issue").await;
    assert_eq!(
        (status, &err["fields"]),
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            &json!({ "lines": "exceeds_original" })
        )
    );

    // Exactly the rest of both rates fits; the list shows the sign.
    put["lines"] = json!([item("4", "100", "21"), item("1", "500", "12")]);
    call(&app, Method::PUT, &uri, Some(put)).await;
    let (status, second) = post_action(&app, &id(&full), "issue").await;
    assert_eq!(status, StatusCode::OK, "{second}");
    let (_, list) = call(
        &app,
        Method::GET,
        "/api/documents?docType=credit_note",
        None,
    )
    .await;
    assert_eq!(list["total"], 2);
    assert_eq!(list["items"][0]["sign"], -1);
    assert_eq!(list["items"][0]["relatedDocumentId"], json!(id(&invoice)));

    // Cancelling one frees its share again.
    post_action(&app, &id(&first), "cancel").await;
    let (_, again) = credit_note(&app, &id(&invoice), json!({ "correctionReason": "x" })).await;
    let mut put = again.clone();
    put["lines"] = json!([item("6", "100", "21")]);
    call(
        &app,
        Method::PUT,
        &format!("/api/documents/{}", id(&again)),
        Some(put),
    )
    .await;
    assert_eq!(
        post_action(&app, &id(&again), "issue").await.0,
        StatusCode::OK
    );
}
