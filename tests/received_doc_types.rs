//! Received and imported documents of the 1f-a types: numbering, required
//! fields, the related-link table, no caps / reasons on imports.

mod common;

use axum::http::{Method, StatusCode};
use common::documents::{
    create_contact, create_doc, create_issued, id, issuable, issue, item, set_company,
};
use common::received::{create_received, received_body};
use common::{TestDb, call, router};
use serde_json::{Value, json};

#[tokio::test]
async fn received_types_numbering_and_links() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    set_company(&app, true).await;
    let supplier = create_contact(&app, json!({ "name": "Dodavatel", "ico": "27074358" })).await;
    let inv = create_received(&app, &supplier, json!({})).await;
    let simplified = create_received(&app, &supplier, json!({ "docType": "simplified" })).await;
    assert_eq!(simplified["number"], "PZD20260001");
    let ddpp = create_received(
        &app,
        &supplier,
        json!({ "docType": "advance_tax_doc", "dueDate": null }),
    )
    .await;

    let dn = create_received(
        &app,
        &supplier,
        json!({ "docType": "debit_note", "relatedDocumentId": id(&simplified) }),
    )
    .await;
    assert_eq!(
        (&dn["number"], &dn["sign"]),
        (&json!("PV20260001"), &json!(1))
    );
    let cn = create_received(
        &app,
        &supplier,
        json!({ "docType": "credit_note", "relatedDocumentId": id(&simplified) }),
    )
    .await;
    assert_eq!(cn["sign"], -1);
    let oc = create_received(
        &app,
        &supplier,
        json!({ "docType": "advance_credit_note", "relatedDocumentId": id(&ddpp) }),
    )
    .await;
    assert_eq!(
        (&oc["number"], &oc["sign"]),
        (&json!("POP20260001"), &json!(-1))
    );
    assert_eq!(oc["parent"]["docType"], "advance_tax_doc");

    let post = |extra: Value| {
        let app = app.clone();
        let body = received_body(&supplier, extra);
        async move { call(&app, Method::POST, "/api/documents", Some(body)).await }
    };
    // Wrong link targets.
    for (t, target) in [
        ("advance_credit_note", id(&inv)),
        ("debit_note", id(&ddpp)),
        ("simplified", id(&inv)),
    ] {
        let (status, err) = post(json!({ "docType": t, "relatedDocumentId": target })).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{t}");
        assert_eq!(
            err["fields"],
            json!({ "relatedDocumentId": "invalid" }),
            "{t}"
        );
    }
    // A received DDPP correction needs a due date; a received simplified
    // document needs its supplier.
    let (status, err) = post(json!({ "docType": "advance_credit_note", "dueDate": null })).await;
    assert_eq!(
        (status, &err["fields"]),
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            &json!({ "dueDate": "required" })
        )
    );
    let (status, err) = post(json!({ "docType": "simplified", "contactId": null })).await;
    assert_eq!(
        (status, &err["fields"]),
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            &json!({ "contactId": "required" })
        )
    );
    let (_, list) = call(
        &app,
        Method::GET,
        "/api/documents?direction=received&docType=advance_credit_note",
        None,
    )
    .await;
    assert_eq!(
        (list["total"].clone(), list["items"][0]["sign"].clone()),
        (json!(1), json!(-1))
    );
}

fn imported(base: &Value, doc_type: &str, number: &str) -> Value {
    let mut b = base.clone();
    b["imported"] = json!(true);
    b["docType"] = json!(doc_type);
    b["number"] = json!(number);
    b
}

#[tokio::test]
async fn imported_types_skip_caps_and_reasons() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let base = issuable(&app).await;
    let invoice = create_issued(&app, base.clone()).await; // 1000 @ 21 %

    // An imported simplified document may have no contact.
    let mut s = imported(&base, "simplified", "ZD-OLD-1");
    s["contactId"] = Value::Null;
    let s = create_doc(&app, s).await;
    let (status, s) = issue(&app, &id(&s)).await;
    assert_eq!(
        (status, &s["number"]),
        (StatusCode::OK, &json!("ZD-OLD-1")),
        "{s}"
    );
    assert_eq!(s["customer"], Value::Null);

    // Imported corrections: no reason, no cap, manual rate.
    let mut cn = imported(&base, "credit_note", "D-OLD-1");
    cn["relatedDocumentId"] = json!(id(&s));
    cn["lines"] = json!([item("100", "100", "21")]);
    let cn = create_doc(&app, cn).await;
    assert_eq!(issue(&app, &id(&cn)).await.0, StatusCode::OK);
    let mut dn = imported(&base, "debit_note", "V-OLD-1");
    dn["relatedDocumentId"] = json!(id(&invoice));
    let dn = create_doc(&app, dn).await;
    let (status, dn) = issue(&app, &id(&dn)).await;
    assert_eq!(status, StatusCode::OK, "{dn}");
    // Cancelling an imported debit note re-checks the cap (no credit notes here).
    let uri = format!("/api/documents/{}/cancel", id(&dn));
    assert_eq!(call(&app, Method::POST, &uri, None).await.0, StatusCode::OK);

    let mut ddpp = imported(&base, "advance_tax_doc", "DP-OLD-1");
    ddpp["lines"] = json!([item("1", "100", "21")]);
    let ddpp = create_doc(&app, ddpp).await;
    let (_, ddpp) = issue(&app, &id(&ddpp)).await;
    let mut oc = imported(&base, "advance_credit_note", "OP-OLD-1");
    oc["relatedDocumentId"] = json!(id(&ddpp));
    oc["lines"] = json!([item("5", "100", "21")]);
    let oc = create_doc(&app, oc).await;
    let (status, oc) = issue(&app, &id(&oc)).await;
    assert_eq!((status, &oc["sign"]), (StatusCode::OK, &json!(-1)), "{oc}");
    // The imported DDPP now has a live correction: it cannot be cancelled.
    let uri = format!("/api/documents/{}/cancel", id(&ddpp));
    let (status, err) = call(&app, Method::POST, &uri, None).await;
    assert_eq!(
        (status, err),
        (StatusCode::CONFLICT, json!({ "code": "advance_in_use" }))
    );

    // Link table on imports: a DDPP correction → DDPP only.
    let mut bad = imported(&base, "advance_credit_note", "OP-OLD-2");
    bad["relatedDocumentId"] = json!(id(&invoice));
    let (status, err) = call(&app, Method::POST, "/api/documents", Some(bad)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(err["fields"], json!({ "relatedDocumentId": "invalid" }));

    // Native POST may not create corrections.
    for t in ["debit_note", "advance_credit_note"] {
        let mut b = base.clone();
        b["docType"] = json!(t);
        let (status, err) = call(&app, Method::POST, "/api/documents", Some(b)).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{t}");
        assert_eq!(err["fields"], json!({ "docType": "invalid" }));
    }
}
