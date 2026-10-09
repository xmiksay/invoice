//! CSV import: contact matching (IČO → DIČ → name) and categories (per
//! kind, inactive, created).

mod common;

use axum::http::{Method, StatusCode};
use common::csvio::{HEADER, confirm, doc_id, file, preview, row};
use common::documents::{create_contact, get_doc, set_company};
use common::received::create_category;
use common::{TestDb, call, router};
use serde_json::{Value, json};

#[tokio::test]
async fn contacts_match_by_ico_dic_or_name() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    set_company(&app, true).await;
    let alfa = create_contact(
        &app,
        json!({ "name": "Alfa s.r.o.", "ico": null, "dic": "CZ87654326" }),
    )
    .await;
    let beta = create_contact(&app, json!({ "name": "Beta s.r.o.", "ico": null })).await;
    let received = |number: &str, name: &str, ico: &str, dic: &str| {
        row(&[
            ("direction", "received"),
            ("doc_type", "invoice"),
            ("supplier_number", number),
            ("issue_date", "1.3.2026"),
            ("counterparty_name", name),
            ("counterparty_ico", ico),
            ("counterparty_dic", dic),
            ("base_0", "100"),
            ("total", "100"),
        ])
    };
    let rows = [
        received("A-1", "Jiné jméno", "87654326", "CZ87654326"),
        received("B-1", "  beta S.R.O. ", "", ""),
        received("C-1", "Gama s.r.o.", "12345679", ""),
        received("C-2", "Gama a.s.", "12345679", ""),
    ];
    let refs: Vec<&str> = rows.iter().map(String::as_str).collect();
    let bytes = file(HEADER, &refs);
    let entries = preview(&app, &bytes).await;
    let matches: Vec<&Value> = entries.iter().map(|e| &e["contactMatch"]).collect();
    assert_eq!(
        matches,
        [
            &json!("existing"),
            &json!("existing"),
            &json!("new"),
            &json!("new")
        ]
    );
    let results = confirm(&app, &bytes, &["row:2", "row:3", "row:4", "row:5"]).await;
    let contact = |key: &str| {
        let id = doc_id(&results, key);
        let app = app.clone();
        async move { get_doc(&app, &id).await["contactId"].clone() }
    };
    assert_eq!(contact("row:2").await, json!(alfa));
    assert_eq!(contact("row:3").await, json!(beta));
    assert_eq!(
        contact("row:4").await,
        contact("row:5").await,
        "one contact created"
    );
    let (_, list) = call(&app, Method::GET, "/api/contacts", None).await;
    assert_eq!(list["total"], 3);
}

#[tokio::test]
async fn categories_inactive_and_per_kind() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    set_company(&app, true).await;
    let (status, _) = call(
        &app,
        Method::POST,
        "/api/settings/categories",
        Some(json!({ "name": "Archiv", "kind": "income", "active": false })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    create_category(&app, "Služby", "income").await;
    let doc = |dir: &str, number: &str, category: &str| {
        row(&[
            ("direction", dir),
            ("doc_type", "invoice"),
            ("number", number),
            ("supplier_number", number),
            ("issue_date", "1.3.2026"),
            ("counterparty_name", "Fiktivní Odběratel s.r.o."),
            ("base_0", "10"),
            ("total", "10"),
            ("category", category),
        ])
    };
    let rows = [doc("issued", "1", "archiv"), doc("received", "2", "SLUŽBY")];
    let refs: Vec<&str> = rows.iter().map(String::as_str).collect();
    let bytes = file(HEADER, &refs);
    let entries = preview(&app, &bytes).await;
    assert_eq!(
        (&entries[0]["categoryMatch"], &entries[0]["warnings"]),
        (
            &Value::Null,
            &json!(["category_inactive", "contact_created"])
        )
    );
    assert_eq!(
        entries[1]["categoryMatch"], "new",
        "income Služby is no expense category"
    );
    let results = confirm(&app, &bytes, &["row:2", "row:3"]).await;
    assert_eq!(
        get_doc(&app, &doc_id(&results, "row:2")).await["categoryId"],
        Value::Null
    );
    let (_, cats) = call(&app, Method::GET, "/api/settings/categories", None).await;
    let expense: Vec<&Value> = cats
        .as_array()
        .expect("list")
        .iter()
        .filter(|c| c["kind"] == "expense")
        .collect();
    assert_eq!(expense.len(), 1);
    assert_eq!(
        (&expense[0]["name"], &expense[0]["active"]),
        (&json!("SLUŽBY"), &json!(true))
    );
}
