//! Final invoice from a proforma (`settle`) and the `advance` line rules.

mod common;

use axum::http::{Method, StatusCode};
use common::documents::{
    create_doc, create_issued, get_doc, id, issuable, item, pay, post_action, set_company,
};
use common::{TestDb, call, router};
use serde_json::{Value, json};

fn advance(doc_id: &str) -> Value {
    json!({ "kind": "advance", "advanceDocumentId": doc_id })
}

#[tokio::test]
async fn payer_settlement_deducts_ddpps() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let mut body = issuable(&app).await; // 10 × 100 @ 21 % → 1210
    body["docType"] = json!("proforma");
    let p = create_issued(&app, body.clone()).await;
    let (_, payment) = pay(&app, &id(&p), json!({ "amount": "1210" })).await;
    let ddpp = payment["advanceDocumentId"]
        .as_str()
        .expect("ddpp")
        .to_string();

    let (status, inv) = post_action(&app, &id(&p), "settle").await;
    assert_eq!(status, StatusCode::CREATED, "{inv}");
    assert_eq!(
        (&inv["docType"], &inv["status"]),
        (&json!("invoice"), &json!("draft"))
    );
    assert_eq!(inv["relatedDocumentId"], json!(id(&p)));
    assert_eq!(inv["parent"]["id"], json!(id(&p)));
    assert_eq!(inv["parent"]["status"], "issued");
    assert!(inv["taxPointDate"].is_string());
    assert_eq!(inv["taxPointDate"], inv["issueDate"]);
    let lines = inv["lines"].as_array().expect("lines");
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["kind"], "item");
    assert_eq!(
        lines[1],
        json!({ "kind": "advance", "position": 2, "advanceDocumentId": ddpp,
                "description": "Odpočet zálohy DP20260001", "base": "-1000.00",
                "recap": [{ "vatRate": "21", "base": "-1000.00", "vat": "-210.00" }] })
    );
    assert_eq!(
        inv["totals"]["recap"],
        json!([{ "vatRate": "21", "base": "0.00", "vat": "0.00", "baseCzk": null, "vatCzk": null }])
    );
    assert_eq!(inv["totals"]["payable"], "0.00");

    let proforma = get_doc(&app, &id(&p)).await;
    assert_eq!(proforma["settled"], true);
    let (status, err) = post_action(&app, &id(&p), "settle").await;
    assert_eq!(
        (status, err),
        (StatusCode::CONFLICT, json!({ "code": "invalid_state" }))
    );

    // The DDPP is taken: another invoice may not deduct it.
    let mut other = body.clone();
    other["docType"] = json!("invoice");
    other["lines"] = json!([item("1", "10", "21"), advance(&ddpp)]);
    let (status, err) = call(&app, Method::POST, "/api/documents", Some(other.clone())).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        err["fields"],
        json!({ "lines.1.advanceDocumentId": "duplicate" })
    );

    let (status, issued) = post_action(&app, &id(&inv), "issue").await;
    assert_eq!(status, StatusCode::OK, "{issued}");
    assert_eq!(issued["paymentState"], "paid");
    assert_eq!(issued["lines"][1]["base"], "-1000.00");

    // Cancelling the final invoice frees the proforma and its DDPP.
    let (status, _) = post_action(&app, &id(&inv), "cancel").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(get_doc(&app, &id(&p)).await["settled"], false);
    let freed = create_doc(&app, other).await;
    assert_eq!(freed["totals"]["payable"], "-1197.90");
    assert_eq!(freed["paymentState"], Value::Null);
}

#[tokio::test]
async fn advance_line_rules() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let body = issuable(&app).await;
    let mut pbody = body.clone();
    pbody["docType"] = json!("proforma");
    let p = create_issued(&app, pbody.clone()).await;
    let (_, payment) = pay(&app, &id(&p), json!({ "amount": "121" })).await;
    let ddpp = payment["advanceDocumentId"]
        .as_str()
        .expect("ddpp")
        .to_string();

    let mut b = body.clone();
    b["lines"] = json!([
        item("1", "10", "21"),
        advance(&ddpp),
        advance(&ddpp),
        advance(&id(&p)),
        { "kind": "advance" },
        advance("00000000-0000-0000-0000-000000000001"),
    ]);
    let (status, err) = call(&app, Method::POST, "/api/documents", Some(b)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        err["fields"],
        json!({ "lines.4.advanceDocumentId": "required" })
    );

    let mut b = body.clone();
    b["lines"] = json!([
        item("1", "10", "21"),
        advance(&ddpp),
        advance(&ddpp),
        advance(&id(&p)),
        advance("00000000-0000-0000-0000-000000000001"),
    ]);
    let (status, err) = call(&app, Method::POST, "/api/documents", Some(b.clone())).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        err["fields"],
        json!({ "lines.2.advanceDocumentId": "duplicate", "lines.3.advanceDocumentId": "invalid",
                "lines.4.advanceDocumentId": "invalid" })
    );

    // Another customer or currency cannot deduct it; proformas hold no advances.
    let mut foreign = body.clone();
    foreign["currency"] = json!("EUR");
    foreign["bankAccountId"] = Value::Null;
    foreign["lines"] = json!([advance(&ddpp)]);
    let (_, err) = call(&app, Method::POST, "/api/documents", Some(foreign)).await;
    assert_eq!(
        err["fields"],
        json!({ "lines.0.advanceDocumentId": "invalid" })
    );
    pbody["lines"] = json!([advance(&ddpp)]);
    let (_, err) = call(&app, Method::POST, "/api/documents", Some(pbody)).await;
    assert_eq!(err["fields"], json!({ "lines.0.kind": "invalid" }));

    // The DDPP already taxed the advance: only a standard-mode invoice deducts it.
    for mode in ["reverse_charge", "exempt"] {
        let mut b = body.clone();
        b["vatMode"] = json!(mode);
        b["lines"] = json!([item("1", "100", "21"), advance(&ddpp)]);
        let (status, err) = call(&app, Method::POST, "/api/documents", Some(b)).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{mode}");
        assert_eq!(
            err["fields"],
            json!({ "lines.1.advanceDocumentId": "invalid" }),
            "{mode}"
        );
    }

    // Compute applies the same rules and deducts.
    let compute = json!({ "contactId": body["contactId"], "lines": [item("2", "100", "21"), advance(&ddpp)] });
    let (status, c) = call(&app, Method::POST, "/api/documents/compute", Some(compute)).await;
    assert_eq!(status, StatusCode::OK, "{c}");
    assert_eq!(c["lines"][1]["description"], "Odpočet zálohy DP20260001");
    assert_eq!(c["totals"]["payable"], "121.00");
    let compute = json!({ "lines": [advance(&ddpp)] });
    let (status, err) = call(&app, Method::POST, "/api/documents/compute", Some(compute)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        err["fields"],
        json!({ "lines.0.advanceDocumentId": "invalid" })
    );

    // A draft may keep (re-save) its own reference.
    let mut own = body.clone();
    own["lines"] = json!([item("1", "100", "21"), advance(&ddpp)]);
    let draft = create_doc(&app, own.clone()).await;
    let uri = format!("/api/documents/{}", id(&draft));
    let (status, saved) = call(&app, Method::PUT, &uri, Some(draft.clone())).await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(saved["totals"]["payable"], "0.00");
}

#[tokio::test]
async fn non_payer_settlement_deducts_the_paid_amount() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let mut body = issuable(&app).await;
    set_company(&app, false).await;
    body["docType"] = json!("proforma");
    body["lines"] = json!([item("1", "1000", "0")]);
    let p = create_issued(&app, body).await;
    let (_, payment) = pay(&app, &id(&p), json!({ "amount": "400" })).await;

    let (status, inv) = post_action(&app, &id(&p), "settle").await;
    assert_eq!(status, StatusCode::CREATED, "{inv}");
    assert_eq!(
        inv["lines"][1],
        json!({ "kind": "advance", "position": 2, "advanceDocumentId": id(&p),
                "description": "Odpočet zálohy Z20260001", "base": "-400.00",
                "recap": [{ "vatRate": "0", "base": "-400.00", "vat": "0.00" }] })
    );
    assert_eq!(inv["totals"]["payable"], "600.00");
    let (status, issued) = post_action(&app, &id(&inv), "issue").await;
    assert_eq!(status, StatusCode::OK, "{issued}");
    assert_eq!(issued["paymentState"], "unpaid");

    let uri = format!(
        "/api/documents/{}/payments/{}",
        id(&p),
        payment["id"].as_str().expect("payment id")
    );
    let (status, err) = call(&app, Method::DELETE, &uri, None).await;
    assert_eq!(
        (status, err),
        (StatusCode::CONFLICT, json!({ "code": "advance_settled" }))
    );
}

#[tokio::test]
async fn settle_requires_an_issued_proforma() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let body = issuable(&app).await;
    let invoice = create_issued(&app, body.clone()).await;
    let mut pbody = body;
    pbody["docType"] = json!("proforma");
    let draft = create_doc(&app, pbody.clone()).await;
    for doc in [&invoice, &draft] {
        let (status, err) = post_action(&app, &id(doc), "settle").await;
        assert_eq!(
            (status, err),
            (StatusCode::CONFLICT, json!({ "code": "invalid_state" }))
        );
    }
    // Unpaid proforma: plain copy without advance lines.
    let p = create_issued(&app, pbody).await;
    let (status, inv) = post_action(&app, &id(&p), "settle").await;
    assert_eq!(status, StatusCode::CREATED, "{inv}");
    assert_eq!(inv["lines"].as_array().map(Vec::len), Some(1));
    assert_eq!(inv["totals"]["payable"], "1210.00");
    let (status, _) = post_action(&app, "00000000-0000-0000-0000-000000000001", "settle").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
