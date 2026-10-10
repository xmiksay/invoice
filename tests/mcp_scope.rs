//! MCP tools: numbers for decimal inputs, the draft scope of update / issue,
//! the manual DDPP exchange rate. Fictitious data only.

mod common;

use axum::http::Method;
use common::documents::{
    create_bank, create_contact, create_issued, dead_url, id, issuable, item, set_company,
};
use common::mcp::{tool, tool_error};
use common::{TestDb, call, router, router_with_cnb};
use serde_json::json;

#[tokio::test]
async fn decimal_inputs_take_numbers_or_strings() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let body = issuable(&app).await;

    let lines = json!([
        { "kind": "item", "description": "Integer", "quantity": 10, "unitPrice": 100, "vatRate": 21 },
        { "kind": "item", "description": "Float", "quantity": 0.1, "unitPrice": 1234.56,
          "discountPct": 0, "vatRate": "21" },
        { "kind": "item", "description": "String", "quantity": "1", "unitPrice": "0.1", "vatRate": "21" }
    ]);
    let computed = tool(&app, "compute_document", json!({ "lines": lines.clone() })).await;
    // 1000 + 123.46 (0.1 × 1234.56 = 123.456 → 123.46) + 0.10
    assert_eq!(computed["totals"]["base"], "1123.56");

    let draft = tool(
        &app,
        "create_draft",
        json!({ "contactId": body["contactId"], "issueDate": "2026-10-01", "lines": lines }),
    )
    .await;
    assert_eq!(draft["lines"][1]["quantity"], "0.1");
    assert_eq!(draft["lines"][1]["unitPrice"], "1234.56");
    assert_eq!(draft["totals"], computed["totals"]);

    // Exponents and too many decimal places are the usual `invalid`.
    let err = tool_error(
        &app,
        "compute_document",
        json!({ "lines": [{ "kind": "item", "quantity": 1e21, "unitPrice": 0.00001, "vatRate": 21 }] }),
    )
    .await;
    assert_eq!(err["fields"]["lines.0.quantity"], "invalid");
    assert_eq!(err["fields"]["lines.0.unitPrice"], "invalid");

    tool(&app, "issue_document", json!({ "id": draft["id"] })).await;
    let paid = tool(
        &app,
        "add_payment",
        json!({ "id": draft["id"], "date": "2026-10-05", "amount": 1000 }),
    )
    .await;
    assert_eq!(paid["payment"]["amount"], "1000.00");
    assert_eq!(paid["paymentState"], "partial");
    let paid = tool(
        &app,
        "add_payment",
        json!({ "id": draft["id"], "date": "2026-10-06", "amount": 359.51 }),
    )
    .await;
    assert_eq!(paid["payment"]["amount"], "359.51");
    assert_eq!(paid["paymentState"], "paid");
}

#[tokio::test]
async fn corrections_and_settlement_drafts_are_out_of_scope() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let body = issuable(&app).await;
    let invoice = create_issued(&app, body.clone()).await;
    let (_, credit) = call(
        &app,
        Method::POST,
        &format!("/api/documents/{}/credit-note", id(&invoice)),
        None,
    )
    .await;
    assert_eq!(credit["docType"], "credit_note");

    let mut proforma = body.clone();
    proforma["docType"] = json!("proforma");
    let proforma = create_issued(&app, proforma).await;
    let (_, settlement) = call(
        &app,
        Method::POST,
        &format!("/api/documents/{}/settle", id(&proforma)),
        None,
    )
    .await;
    assert_eq!(settlement["docType"], "invoice");
    assert_eq!(settlement["status"], "draft");

    for draft in [&credit, &settlement] {
        let err = tool_error(&app, "issue_document", json!({ "id": id(draft) })).await;
        assert_eq!(err, json!({ "code": "invalid_state" }));
        let mut update = body.clone();
        update["id"] = json!(id(draft));
        let err = tool_error(&app, "update_draft", update).await;
        assert_eq!(err, json!({ "code": "invalid_state" }));
    }
    // Still readable.
    let doc = tool(&app, "get_document", json!({ "id": id(&credit) })).await;
    assert_eq!(doc["status"], "draft");
}

#[tokio::test]
async fn ddpp_takes_a_manual_rate_when_cnb_is_down() {
    let db = TestDb::new().await;
    let app = router_with_cnb(db.conn.clone(), &dead_url());
    set_company(&app, true).await;
    let contact = create_contact(&app, json!({})).await;
    create_bank(&app, "EUR").await;
    let proforma = create_issued(
        &app,
        json!({ "docType": "proforma", "contactId": contact, "currency": "EUR",
                "exchangeRate": "25", "issueDate": "2026-10-01",
                "lines": [item("1", "100", "21")] }),
    )
    .await;

    let args = json!({ "id": id(&proforma), "date": "2026-09-01", "amount": 121 });
    let err = tool_error(&app, "add_payment", args.clone()).await;
    assert_eq!(
        err,
        json!({ "code": "validation", "fields": { "exchangeRate": "required" } })
    );

    let mut manual = args;
    manual["exchangeRate"] = json!(24.5);
    let paid = tool(&app, "add_payment", manual).await;
    let ddpp = tool(
        &app,
        "get_document",
        json!({ "id": paid["payment"]["advanceDocumentId"] }),
    )
    .await;
    assert_eq!(ddpp["exchangeRate"], "24.5");
    assert_eq!(ddpp["exchangeRateSource"], "manual");
}
