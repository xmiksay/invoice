//! CSV import: preview and confirm of issued + received rows (all mapping
//! rules), duplicates, contacts, categories, payments and related links.

mod common;

use axum::http::Method;
use common::csvio::{HEADER, by_key, confirm, doc_id, file, preview, row};
use common::documents::{create_bank, create_contact, get_doc, set_company};
use common::received::create_category;
use common::{TestDb, call, router};
use serde_json::{Value, json};

fn invoice() -> String {
    row(&[
        ("direction", "issued"),
        ("doc_type", "invoice"),
        ("number", "2026000001"),
        ("issue_date", "15.01.2026"),
        ("counterparty_name", "Fiktivní Odběratel s.r.o."),
        ("counterparty_ico", "12345679"),
        ("base_21", "1000,00"),
        ("vat_21", "210,00"),
        ("total", "1210,00"),
        ("paid_date", "27.01.2026"),
        ("variable_symbol", "2026000001"),
        ("category", "Služby"),
        ("note", "Děkujeme"),
    ])
}

fn batch() -> Vec<u8> {
    let rows = [
        invoice(),
        row(&[
            ("direction", "received"),
            ("doc_type", "invoice"),
            ("number", "ignored"),
            ("supplier_number", "FV-42"),
            ("issue_date", "2026-01-20"),
            ("received_date", "22. 1. 2026"),
            ("counterparty_name", "Vzorový Dodavatel a.s."),
            ("counterparty_ico", "87654326"),
            ("counterparty_dic", "CZ87654326"),
            ("currency", "EUR"),
            ("exchange_rate", "24,335"),
            ("base_21", "2 433,50"),
            ("vat_21", "511,04"),
            ("total", "121,00"),
            ("vat_deductible", "ne"),
            ("category", "software"),
        ]),
        row(&[
            ("direction", "issued"),
            ("doc_type", "credit_note"),
            ("number", "2026000002"),
            ("related_number", "2026000001"),
            ("issue_date", "02.02.2026"),
            ("counterparty_name", "Fiktivní Odběratel s.r.o."),
            ("counterparty_ico", "12345679"),
            ("base_21", "-100,00"),
            ("vat_21", "-21,00"),
            ("total", "-121,00"),
            ("paid_date", "05.02.2026"),
        ]),
        invoice(),
        row(&[
            ("direction", "issued"),
            ("doc_type", "invoice"),
            ("number", "2026000003"),
            ("issue_date", "15.01.2026"),
            ("counterparty_name", "X"),
            ("base_21", "100"),
            ("total", "99"),
        ]),
        row(&[
            ("direction", "received"),
            ("doc_type", "debit_note"),
            ("supplier_number", "FV-43"),
            ("issue_date", "31.02.2026"),
            ("counterparty_name", "Vzorový Dodavatel a.s."),
            ("counterparty_ico", "87654326"),
        ]),
        row(&[
            ("direction", "issued"),
            ("doc_type", "proforma"),
            ("number", "ZF-2026-1"),
            ("issue_date", "10.01.2026"),
            ("counterparty_name", "Fiktivní Odběratel s.r.o."),
            ("counterparty_ico", "12345679"),
            ("base_21", "500"),
            ("vat_21", "105"),
            ("total", "605"),
            ("paid_date", "12.01.2026"),
        ]),
        row(&[
            ("direction", "issued"),
            ("doc_type", "simplified"),
            ("number", "UD-1"),
            ("issue_date", "11.01.2026"),
            ("base_12", "100"),
            ("vat_12", "12"),
            ("total", "112"),
        ]),
    ];
    let refs: Vec<&str> = rows.iter().map(String::as_str).collect();
    file(HEADER, &refs)
}

const ALL: [&str; 8] = [
    "row:2", "row:3", "row:4", "row:5", "row:6", "row:7", "row:8", "row:9",
];

async fn setup(app: &axum::Router) -> (String, String) {
    set_company(app, true).await;
    let bank = create_bank(app, "CZK").await;
    let customer = create_contact(app, json!({ "name": "Odběratel", "ico": "12345679" })).await;
    create_category(app, "Software", "expense").await;
    (bank, customer)
}

#[tokio::test]
async fn preview_reports_every_row() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    setup(&app).await;
    let rows = preview(&app, &batch()).await;
    let keys: Vec<&str> = rows
        .iter()
        .map(|r| r["key"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(keys, ALL);

    let i = by_key(&rows, "row:2");
    assert_eq!(i["status"], "ok");
    assert_eq!((&i["row"], &i["field"]), (&json!(2), &Value::Null));
    assert_eq!(
        (&i["direction"], &i["docType"]),
        (&json!("issued"), &json!("invoice"))
    );
    assert_eq!(i["number"], "2026000001");
    assert_eq!(i["contactMatch"], "existing");
    assert_eq!(i["categoryMatch"], "new");
    assert_eq!(i["warnings"], json!(["category_created"]));
    assert_eq!(
        (&i["taxPointDate"], &i["dueDate"]),
        (&json!("2026-01-15"), &json!("2026-01-15"))
    );
    assert_eq!(
        (&i["total"], &i["currency"]),
        (&json!("1210.00"), &json!("CZK"))
    );
    assert_eq!(i["hasPdf"], false);

    let r = by_key(&rows, "row:3");
    assert_eq!(
        (&r["status"], &r["number"]),
        (&json!("ok"), &json!("FV-42"))
    );
    assert_eq!(
        r["counterparty"],
        json!({ "name": "Vzorový Dodavatel a.s.", "ico": "87654326" })
    );
    assert_eq!(r["contactMatch"], "new");
    assert_eq!(r["categoryMatch"], "existing");
    assert_eq!(r["warnings"], json!(["contact_created"]));
    assert_eq!(
        (&r["total"], &r["currency"]),
        (&json!("121.00"), &json!("EUR"))
    );

    let c = by_key(&rows, "row:4");
    assert_eq!(
        (&c["status"], &c["total"]),
        (&json!("ok"), &json!("121.00"))
    );
    assert_eq!(
        (&c["relatedNumber"], &c["relatedFound"]),
        (&json!("2026000001"), &json!(true))
    );

    assert_eq!(by_key(&rows, "row:5")["status"], "duplicate");
    let m = by_key(&rows, "row:6");
    assert_eq!(
        (&m["status"], &m["error"], &m["field"]),
        (&json!("error"), &json!("total_mismatch"), &json!("total"))
    );
    let e = by_key(&rows, "row:7");
    assert_eq!(
        (&e["error"], &e["field"]),
        (&json!("invalid_date"), &json!("issue_date"))
    );
    assert_eq!(
        (&e["direction"], &e["docType"], &e["number"]),
        (&json!("received"), &json!("debit_note"), &json!("FV-43"))
    );
    assert_eq!(e["counterparty"]["ico"], "87654326");
    assert_eq!(by_key(&rows, "row:8")["taxPointDate"], Value::Null);
    let s = by_key(&rows, "row:9");
    assert_eq!(
        (&s["status"], &s["counterparty"], &s["contactMatch"]),
        (&json!("ok"), &Value::Null, &Value::Null)
    );
}

#[tokio::test]
async fn confirm_imports_links_pays_and_then_finds_duplicates() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let (bank, customer) = setup(&app).await;
    let results = confirm(&app, &batch(), &ALL).await;
    for key in ["row:5", "row:6", "row:7"] {
        assert_eq!(by_key(&results, key)["status"], "skipped", "{key}");
    }

    let inv = get_doc(&app, &doc_id(&results, "row:2")).await;
    assert_eq!(
        (&inv["imported"], &inv["status"]),
        (&json!(true), &json!("issued"))
    );
    assert_eq!(
        (&inv["number"], &inv["contactId"]),
        (&json!("2026000001"), &json!(customer))
    );
    assert_eq!(
        inv["customer"]["name"], "Fiktivní Odběratel s.r.o.",
        "snapshot as written"
    );
    assert_eq!(
        (&inv["supplier"]["ico"], &inv["supplier"]["vatPayer"]),
        (&json!("44444443"), &json!(true))
    );
    assert_eq!(inv["bankAccountId"], json!(bank));
    assert_eq!(inv["bankSnapshot"]["accountNumber"], "19-2000145399/0800");
    assert_eq!(
        (&inv["paymentMethod"], &inv["locale"]),
        (&json!("bank_transfer"), &json!("cs"))
    );
    assert_eq!(
        (&inv["headerNote"], &inv["variableSymbol"]),
        (&json!("Děkujeme"), &json!("2026000001"))
    );
    assert_eq!(
        (&inv["paid"], &inv["paymentState"]),
        (&json!("1210.00"), &json!("paid"))
    );
    assert_eq!(inv["lines"], json!([]));
    assert_eq!(
        inv["totals"]["recap"],
        json!([{ "vatRate": "21", "base": "1000.00", "vat": "210.00", "baseCzk": null, "vatCzk": null }])
    );
    let (_, cats) = call(&app, Method::GET, "/api/settings/categories", None).await;
    let created = cats
        .as_array()
        .expect("categories")
        .iter()
        .find(|c| c["name"] == "Služby")
        .expect("created");
    assert_eq!(
        (&created["kind"], &created["active"]),
        (&json!("income"), &json!(true))
    );
    assert_eq!(inv["categoryId"], created["id"]);

    let rec = get_doc(&app, &doc_id(&results, "row:3")).await;
    assert_eq!(
        (&rec["direction"], &rec["supplierNumber"]),
        (&json!("received"), &json!("FV-42"))
    );
    assert_ne!(rec["number"], "ignored");
    assert_eq!(
        (&rec["exchangeRate"], &rec["exchangeRateSource"]),
        (&json!("24.335"), &json!("manual"))
    );
    assert_eq!(
        rec["totals"]["recap"][0],
        json!({ "vatRate": "21", "base": "100.00", "vat": "21.00", "baseCzk": "2433.50", "vatCzk": "511.04" })
    );
    assert_eq!(
        (&rec["totals"]["payable"], &rec["totals"]["totalCzk"]),
        (&json!("121.00"), &json!("2944.54"))
    );
    assert_eq!(
        (&rec["receivedDate"], &rec["vatDeductible"]),
        (&json!("2026-01-22"), &json!(false))
    );
    assert_eq!(rec["supplier"]["dic"], "CZ87654326");
    assert_eq!(rec["paid"], "0.00");
    assert!(rec["contactId"].is_string(), "contact created");
    assert!(rec["categoryId"].is_string());

    let credit = get_doc(&app, &doc_id(&results, "row:4")).await;
    assert_eq!(
        credit["relatedDocumentId"],
        json!(doc_id(&results, "row:2"))
    );
    assert_eq!(
        (&credit["totals"]["payable"], &credit["paid"]),
        (&json!("121.00"), &json!("121.00"))
    );
    let proforma = get_doc(&app, &doc_id(&results, "row:8")).await;
    assert_eq!(proforma["paid"], "605.00");
    assert_eq!(
        proforma["relatedDocuments"],
        json!([]),
        "no DDPP for an imported proforma"
    );
    let simplified = get_doc(&app, &doc_id(&results, "row:9")).await;
    assert_eq!(
        (&simplified["customer"], &simplified["contactId"]),
        (&Value::Null, &Value::Null)
    );

    let again = preview(&app, &batch()).await;
    for key in ["row:2", "row:3", "row:4", "row:8", "row:9"] {
        assert_eq!(by_key(&again, key)["status"], "duplicate", "{key}");
    }
    let results = confirm(&app, &batch(), &["row:3"]).await;
    assert_eq!(by_key(&results, "row:3")["status"], "skipped");
}
