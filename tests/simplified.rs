//! Simplified tax documents: optional customer, numbering, no advances,
//! corrections, list, compute and PDF.

mod common;

use axum::http::{Method, StatusCode};
use common::documents::{
    create_bank, create_doc, create_issued, id, issuable, issue, item, set_company,
};
use common::mdcast::PdfEnv;
use common::{TestDb, call, router};
use serde_json::{Value, json};

#[tokio::test]
async fn issues_without_a_customer() {
    let db = TestDb::new().await;
    let env = PdfEnv::new();
    let app = env.router(db.conn.clone(), None);
    set_company(&app, true).await;
    create_bank(&app, "CZK").await;
    let draft = create_doc(
        &app,
        json!({ "docType": "simplified", "lines": [item("1", "1000", "21")] }),
    )
    .await;
    assert_eq!(
        (&draft["docType"], &draft["contactId"], &draft["sign"]),
        (&json!("simplified"), &Value::Null, &json!(1))
    );
    let (status, doc) = issue(&app, &id(&draft)).await;
    assert_eq!(status, StatusCode::OK, "{doc}");
    let number = doc["number"].as_str().expect("number");
    assert!(
        number.starts_with("ZD") && number.ends_with("0001"),
        "{number}"
    );
    assert_eq!(doc["customer"], Value::Null);
    assert_eq!(doc["totals"]["payable"], "1210.00");
    let data = env.mock.last().data;
    assert_eq!(data["title"], "Zjednodušený daňový doklad");
    assert_eq!(data["customer"], Value::Null, "no customer block");

    // The UI-only 10 000 CZK limit: the server never refuses.
    let big = create_issued(
        &app,
        json!({ "docType": "simplified", "lines": [item("100", "1000", "21")] }),
    )
    .await;
    assert_eq!(big["totals"]["payable"], "121000.00");

    let (_, list) = call(&app, Method::GET, "/api/documents?docType=simplified", None).await;
    assert_eq!(list["total"], 2);
    assert_eq!(list["items"][0]["sign"], 1);
}

#[tokio::test]
async fn no_advance_lines_and_settle_still_makes_an_invoice() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let mut body = issuable(&app).await;
    body["docType"] = json!("simplified");
    body["lines"] = json!([item("1", "100", "21"),
        { "kind": "advance", "advanceDocumentId": "00000000-0000-0000-0000-000000000001" }]);
    let (status, err) = call(&app, Method::POST, "/api/documents", Some(body.clone())).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(err["fields"], json!({ "lines.1.kind": "invalid" }));

    let (status, c) = call(
        &app,
        Method::POST,
        "/api/documents/compute",
        Some(json!({ "docType": "simplified", "lines": [item("1", "100", "21")] })),
    )
    .await;
    assert_eq!(
        (status, &c["totals"]["total"]),
        (StatusCode::OK, &json!("121.00"))
    );
    for t in ["debit_note", "advance_credit_note"] {
        let (status, _) = call(
            &app,
            Method::POST,
            "/api/documents/compute",
            Some(json!({ "docType": t, "lines": [item("1", "100", "21")] })),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{t}");
    }

    let mut pbody = body.clone();
    pbody["docType"] = json!("proforma");
    pbody["taxPointDate"] = Value::Null;
    pbody["lines"] = json!([item("1", "100", "21")]);
    let p = create_issued(&app, pbody).await;
    let (status, inv) = call(
        &app,
        Method::POST,
        &format!("/api/documents/{}/settle", id(&p)),
        None,
    )
    .await;
    assert_eq!(
        (status, &inv["docType"]),
        (StatusCode::CREATED, &json!("invoice"))
    );
}

#[tokio::test]
async fn credit_and_debit_notes_reference_the_simplified_document() {
    let db = TestDb::new().await;
    let env = PdfEnv::new();
    let app = env.router(db.conn.clone(), None);
    let mut body = issuable(&app).await;
    body["docType"] = json!("simplified");
    let doc = create_issued(&app, body).await;
    let number = doc["number"].as_str().expect("number").to_string();

    let (status, cn) = call(
        &app,
        Method::POST,
        &format!("/api/documents/{}/credit-note", id(&doc)),
        Some(json!({ "correctionReason": "Sleva" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{cn}");
    assert_eq!(cn["docType"], "credit_note");
    let (status, issued) = issue(&app, &id(&cn)).await;
    assert_eq!(status, StatusCode::OK, "{issued}");
    assert_eq!(
        env.mock.last().data["reference"],
        format!(
            "Opravný daňový doklad k zjednodušenému daňovému dokladu {number}\nDůvod opravy: Sleva"
        )
    );
    let (status, dn) = call(
        &app,
        Method::POST,
        &format!("/api/documents/{}/debit-note", id(&doc)),
        None,
    )
    .await;
    assert_eq!(
        (status, &dn["docType"]),
        (StatusCode::CREATED, &json!("debit_note"))
    );
}

#[tokio::test]
async fn corrections_of_a_simplified_document_without_customer_issue() {
    let db = TestDb::new().await;
    let env = PdfEnv::new();
    let app = env.router(db.conn.clone(), None);
    set_company(&app, true).await;
    create_bank(&app, "CZK").await;
    let doc = create_issued(
        &app,
        json!({ "docType": "simplified", "lines": [item("1", "1000", "21")] }),
    )
    .await;
    for kind in ["credit-note", "debit-note"] {
        let (status, note) = call(
            &app,
            Method::POST,
            &format!("/api/documents/{}/{kind}", id(&doc)),
            Some(json!({ "correctionReason": "Oprava" })),
        )
        .await;
        assert_eq!(
            (status, &note["contactId"]),
            (StatusCode::CREATED, &Value::Null)
        );
        let mut put = note.clone();
        put["lines"] = json!([item("1", "100", "21")]);
        let uri = format!("/api/documents/{}", id(&note));
        assert_eq!(
            call(&app, Method::PUT, &uri, Some(put)).await.0,
            StatusCode::OK
        );
        let (status, issued) = issue(&app, &id(&note)).await;
        assert_eq!(status, StatusCode::OK, "{kind}: {issued}");
        assert_eq!(issued["customer"], Value::Null);
        assert_eq!(
            env.mock.last().data["customer"],
            Value::Null,
            "{kind}: no empty party"
        );
    }
}
