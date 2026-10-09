//! MCP tools over HTTP (raw JSON-RPC): each tool's happy path and the full
//! issue flow with the mdcast mock. Fictitious data only.

mod common;

use axum::Router;
use axum::extract::Path;
use axum::http::{Method, StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::get;
use common::documents::{create_bank, create_contact, create_doc, id, issuable, item, set_company};
use common::mcp::{rpc, tool};
use common::mdcast::get_raw;
use common::received::create_received;
use common::{TestDb, call, router, router_with_ares};
use serde_json::{Value, json};

fn pdf_url(doc: &Value) -> String {
    format!("/api/documents/{}/pdf", id(doc))
}

#[tokio::test]
async fn full_flow_from_contact_to_paid_and_sent_invoice() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    set_company(&app, true).await;
    let bank = create_bank(&app, "CZK").await;

    let contact = tool(
        &app,
        "create_contact",
        json!({ "name": "Zákazník Test s.r.o.", "ico": "12345679", "city": "Ostrava" }),
    )
    .await;
    assert_eq!(contact["name"], "Zákazník Test s.r.o.");
    assert_eq!(contact["country"], "CZ");

    let lines = json!([item("10", "100", "21"), item("1", "50", "12")]);
    let computed = tool(
        &app,
        "compute_document",
        json!({ "lines": lines.clone(), "contactId": contact["id"] }),
    )
    .await;
    assert_eq!(computed["totals"]["payable"], "1266.00");
    assert_eq!(computed["lines"][0]["base"], "1000.00");

    // Defaults as the UI: dates, currency, locale, VAT mode, bank, method.
    let draft = tool(
        &app,
        "create_draft",
        json!({ "contactId": contact["id"], "issueDate": "2026-10-01", "lines": lines.clone() }),
    )
    .await;
    assert_eq!(draft["status"], "draft");
    assert_eq!(draft["docType"], "invoice");
    assert_eq!(draft["dueDate"], "2026-10-15");
    assert_eq!(draft["taxPointDate"], "2026-10-01");
    assert_eq!(draft["currency"], "CZK");
    assert_eq!(draft["locale"], "cs");
    assert_eq!(draft["vatMode"], "standard");
    assert_eq!(draft["bankAccountId"], bank.as_str());
    assert_eq!(draft["paymentMethod"], "bank_transfer");
    assert_eq!(draft["totals"], computed["totals"]);
    assert_eq!(draft["pdfUrl"], pdf_url(&draft));

    // PUT semantics: the full input replaces the draft (twice → same result).
    let update = json!({ "id": draft["id"], "contactId": contact["id"], "issueDate": "2026-10-01",
        "taxPointDate": "2026-10-01", "dueDate": "2026-10-31", "currency": "CZK", "locale": "en",
        "vatMode": "standard", "bankAccountId": bank, "paymentMethod": "bank_transfer",
        "lines": [item("10", "100", "21")] });
    let updated = tool(&app, "update_draft", update.clone()).await;
    let again = tool(&app, "update_draft", update).await;
    assert_eq!(updated["dueDate"], "2026-10-31");
    assert_eq!(updated["locale"], "en");
    assert_eq!(updated["totals"]["payable"], "1210.00");
    assert_eq!(again["totals"], updated["totals"]);

    let issued = tool(&app, "issue_document", json!({ "id": draft["id"] })).await;
    assert_eq!(issued["status"], "issued");
    assert_eq!(issued["number"], "20260001");
    assert_eq!(issued["paymentState"], "unpaid");
    assert!(issued["pdf"]["sha256"].is_string(), "PDF archived at issue");
    assert_eq!(issued["pdfUrl"], pdf_url(&draft));

    let paid = tool(
        &app,
        "add_payment",
        json!({ "id": draft["id"], "date": "2026-10-20", "amount": "1210", "note": "bank" }),
    )
    .await;
    assert_eq!(paid["payment"]["amount"], "1210.00");
    assert_eq!(paid["payment"]["note"], "bank");
    assert_eq!(paid["payment"]["advanceDocumentId"], Value::Null);
    assert_eq!(paid["paymentState"], "paid");

    let sent_at = "2026-10-02T09:30:00+02:00";
    let sent = tool(
        &app,
        "mark_sent",
        json!({ "id": draft["id"], "sentAt": sent_at }),
    )
    .await;
    // Stored as timestamptz, returned in UTC.
    assert_eq!(sent["sentAt"], "2026-10-02T07:30:00Z");
    let resent = tool(
        &app,
        "mark_sent",
        json!({ "id": draft["id"], "sentAt": sent_at }),
    )
    .await;
    assert_eq!(resent["sentAt"], sent["sentAt"]);

    let doc = tool(&app, "get_document", json!({ "id": draft["id"] })).await;
    assert_eq!(doc["number"], "20260001");
    assert_eq!(doc["paid"], "1210.00");
    assert_eq!(doc["payments"].as_array().map(Vec::len), Some(1));
    assert_eq!(doc["payments"][0]["id"], paid["payment"]["id"]);
    assert_eq!(doc["pdfUrl"], pdf_url(&draft));

    // The PDF is downloaded over REST with the same token: the archive.
    let (status, headers, bytes) = get_raw(&app, &pdf_url(&draft)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers["content-type"], "application/pdf");
    assert!(!bytes.is_empty());

    let list = tool(&app, "list_documents", json!({ "direction": "issued" })).await;
    assert_eq!(list["total"], 1);
    assert_eq!(list["items"][0]["number"], "20260001");
    assert_eq!(list["items"][0]["pdfUrl"], pdf_url(&draft));
}

#[tokio::test]
async fn list_documents_filters_and_pages() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let body = issuable(&app).await;
    for _ in 0..3 {
        create_doc(&app, body.clone()).await;
    }
    let mut proforma = body.clone();
    proforma["docType"] = json!("proforma");
    create_doc(&app, proforma).await;
    let contact = body["contactId"].as_str().expect("contact");
    let received = create_received(&app, contact, json!({})).await;

    let page = tool(
        &app,
        "list_documents",
        json!({ "direction": "issued", "limit": 2, "offset": 1 }),
    )
    .await;
    assert_eq!(page["total"], 4);
    assert_eq!(page["items"].as_array().map(Vec::len), Some(2));

    let filtered = tool(
        &app,
        "list_documents",
        json!({ "direction": "issued", "docType": "proforma", "status": "draft",
                "contactId": contact, "from": "2026-10-01", "to": "2026-10-31" }),
    )
    .await;
    assert_eq!(filtered["total"], 1);
    assert_eq!(filtered["items"][0]["docType"], "proforma");

    // A received document without an uploaded original has nothing to download.
    let received_list = tool(&app, "list_documents", json!({ "direction": "received" })).await;
    assert_eq!(received_list["total"], 1);
    assert_eq!(received_list["items"][0]["pdfUrl"], Value::Null);
    let doc = tool(&app, "get_document", json!({ "id": received["id"] })).await;
    assert_eq!(doc["direction"], "received");
    assert_eq!(doc["pdfUrl"], Value::Null);
    assert_eq!(doc["payments"], json!([]));
}

#[tokio::test]
async fn proforma_payment_issues_the_ddpp() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let mut body = issuable(&app).await;
    body["docType"] = json!("proforma");
    let proforma = tool(&app, "create_draft", body).await;
    assert_eq!(proforma["docType"], "proforma");
    assert_eq!(proforma["taxPointDate"], Value::Null);
    tool(&app, "issue_document", json!({ "id": proforma["id"] })).await;

    let paid = tool(
        &app,
        "add_payment",
        json!({ "id": proforma["id"], "date": "2026-10-05", "amount": "1210" }),
    )
    .await;
    assert_eq!(paid["paymentState"], "paid");
    let ddpp_id = paid["payment"]["advanceDocumentId"]
        .as_str()
        .expect("DDPP issued");
    let ddpp = tool(&app, "get_document", json!({ "id": ddpp_id })).await;
    assert_eq!(ddpp["docType"], "advance_tax_doc");
    assert_eq!(ddpp["status"], "issued");
    assert_eq!(ddpp["pdfUrl"], format!("/api/documents/{ddpp_id}/pdf"));
}

#[tokio::test]
async fn contacts_catalog_and_settings() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    set_company(&app, true).await;
    create_bank(&app, "CZK").await;
    let a = create_contact(&app, json!({ "name": "Alfa Test a.s.", "ico": "12345679" })).await;
    create_contact(
        &app,
        json!({ "name": "Beta Test s.r.o.", "ico": "44444443" }),
    )
    .await;

    let all = tool(&app, "list_contacts", json!({})).await;
    assert_eq!(all["total"], 2);
    let found = tool(&app, "list_contacts", json!({ "q": "alfa", "limit": 1 })).await;
    assert_eq!(found["total"], 1);
    assert_eq!(found["items"][0]["id"], a.as_str());
    let contact = tool(&app, "get_contact", json!({ "id": a })).await;
    assert_eq!(contact["name"], "Alfa Test a.s.");

    for (name, active) in [("Konzultace", true), ("Starý tarif", false)] {
        let (status, it) = call(
            &app,
            Method::POST,
            "/api/catalog/items",
            Some(
                json!({ "name": name, "unit": "h", "unitPrice": "1500", "vatRate": "21",
                         "active": active }),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{it}");
    }
    let items = tool(&app, "list_catalog_items", json!({})).await;
    let names: Vec<&str> = items["items"]
        .as_array()
        .expect("items")
        .iter()
        .filter_map(|i| i["name"].as_str())
        .collect();
    assert_eq!(names, ["Konzultace"]);
    let none = tool(&app, "list_catalog_items", json!({ "q": "xyz" })).await;
    assert_eq!(none["items"], json!([]));

    let (status, rate) = call(
        &app,
        Method::POST,
        "/api/settings/vat-rates",
        Some(json!({ "rate": "5", "label": "Zrušená", "active": false })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{rate}");
    let settings = tool(&app, "get_settings", json!({})).await;
    assert_eq!(settings["company"]["name"], "Dodavatel s.r.o.");
    assert_eq!(settings["bankAccounts"][0]["currency"], "CZK");
    let rates = settings["vatRates"].as_array().expect("rates");
    assert!(!rates.is_empty());
    assert!(rates.iter().all(|r| r["active"] == true));
    assert!(rates.iter().all(|r| r["rate"] != "5"));
    assert!(
        settings["numberSeries"]
            .as_array()
            .is_some_and(|s| !s.is_empty())
    );
}

#[tokio::test]
async fn lookup_ares_returns_the_contact_draft() {
    async fn subject(Path(ico): Path<String>) -> axum::response::Response {
        let path = format!(
            "{}/tests/fixtures/ares/{ico}.json",
            env!("CARGO_MANIFEST_DIR")
        );
        match std::fs::read_to_string(path) {
            Ok(body) => ([(header::CONTENT_TYPE, "application/json")], body).into_response(),
            Err(_) => StatusCode::NOT_FOUND.into_response(),
        }
    }
    let mock = Router::new().route("/rest/ekonomicke-subjekty/{ico}", get(subject));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock ARES");
    let addr = listener.local_addr().expect("mock address");
    tokio::spawn(async move { axum::serve(listener, mock).await.expect("mock ARES") });

    let db = TestDb::new().await;
    let app = router_with_ares(db.conn.clone(), &format!("http://{addr}/rest/"));
    let subject = tool(&app, "lookup_ares", json!({ "ico": "12345679" })).await;
    assert_eq!(subject["ico"], "12345679");
    assert!(subject["name"].as_str().is_some_and(|n| !n.is_empty()));

    let resp = rpc(
        &app,
        "tools/call",
        json!({ "name": "lookup_ares", "arguments": { "ico": "11111119" } }),
    )
    .await;
    assert_eq!(resp["result"]["isError"], true);
    assert_eq!(
        resp["result"]["structuredContent"],
        json!({ "code": "ares_not_found" })
    );
}
