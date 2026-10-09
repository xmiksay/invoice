use chrono::{DateTime, FixedOffset};
use serde_json::json;
use uuid::Uuid;

use super::*;
use crate::csvio::write::{body, header};

fn d(s: &str) -> Decimal {
    s.parse().expect("decimal")
}

fn date(m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, m, day).expect("date")
}

fn ts(n: i64) -> DateTime<FixedOffset> {
    DateTime::from_timestamp(1_790_000_000 + n, 0)
        .expect("timestamp")
        .fixed_offset()
}

fn snapshot(name: &str, ico: &str) -> serde_json::Value {
    json!({ "name": name, "ico": ico, "dic": format!("CZ{ico}"), "street": "Vzorová 1",
            "city": "Praha", "zip": "11000", "country": "CZ", "registration": null,
            "vatPayer": null })
}

fn doc() -> document::Model {
    document::Model {
        id: Uuid::new_v4(),
        direction: "issued".into(),
        doc_type: "invoice".into(),
        status: "issued".into(),
        number: Some("2026000001".into()),
        number_year: Some(2026),
        number_seq: Some(1),
        imported: false,
        contact_id: None,
        issue_date: date(3, 1),
        tax_point_date: Some(date(3, 1)),
        due_date: Some(date(3, 15)),
        currency: "CZK".into(),
        exchange_rate: None,
        exchange_rate_date: None,
        exchange_rate_source: None,
        locale: "cs".into(),
        vat_mode: "standard".into(),
        bank_account_id: None,
        payment_method: "bank_transfer".into(),
        variable_symbol: Some("2026000001".into()),
        constant_symbol: None,
        order_ref: None,
        header_note: Some("Hlavička".into()),
        footer_note: None,
        internal_note: Some("Interní".into()),
        round_total: true,
        supplier_snapshot: Some(snapshot("Dodavatel s.r.o.", "44444443")),
        customer_snapshot: Some(snapshot("Odběratel a.s.", "12345679")),
        bank_snapshot: None,
        total_base: d("1000"),
        total_vat: d("210.40"),
        total: d("1210.40"),
        rounding: d("-0.40"),
        payable: d("1210"),
        total_czk: None,
        paid: Decimal::ZERO,
        sent_at: None,
        cancelled_at: None,
        cancel_reason: None,
        related_document_id: None,
        payment_id: None,
        correction_reason: None,
        pdf_path: None,
        pdf_sha256: None,
        pdf_rendered_at: None,
        supplier_number: Some("ignored".into()),
        received_date: Some(date(3, 2)),
        vat_deductible: true,
        supplier_account: None,
        category_id: None,
        custom_fields: json!({}),
        original_path: None,
        original_sha256: None,
        original_size: None,
        original_uploaded_at: None,
        created_at: ts(0),
        updated_at: ts(0),
    }
}

fn recap(doc: &document::Model, rate: &str, base: &str, vat: &str) -> vat_recap::Model {
    vat_recap::Model {
        document_id: doc.id,
        vat_rate: d(rate),
        base: d(base),
        vat: d(vat),
        base_czk: None,
        vat_czk: None,
    }
}

fn pay(day: u32, amount: &str, created: i64) -> payment::Model {
    payment::Model {
        id: Uuid::new_v4(),
        document_id: Uuid::nil(),
        date: date(4, day),
        amount: d(amount),
        note: None,
        created_at: ts(created),
    }
}

fn row(s: &Source) -> Vec<String> {
    let rates = [d("21"), d("12"), d("0")];
    let text =
        String::from_utf8(body(&rates, &[out_row(s).expect("row")]).expect("csv")).expect("utf-8");
    let cells: Vec<String> = text
        .trim_end_matches("\r\n")
        .split(';')
        .map(str::to_string)
        .collect();
    assert_eq!(cells.len(), header(&rates).len());
    cells
}

fn cell<'a>(cells: &'a [String], name: &str) -> &'a str {
    let rates = [d("21"), d("12"), d("0")];
    let i = header(&rates)
        .iter()
        .position(|h| h == name)
        .expect("column");
    &cells[i]
}

#[test]
fn issued_czk_invoice() {
    let doc = doc();
    let recap = [recap(&doc, "21", "1000", "210.40")];
    let c = row(&Source {
        doc: &doc,
        recap: &recap,
        payments: &[],
        parent: None,
        category: Some("Služby"),
    });
    let get = |n| cell(&c, n);
    assert_eq!(get("direction"), "issued");
    assert_eq!(get("number"), "2026000001");
    assert_eq!(get("supplier_number"), "", "issued: no supplier number");
    assert_eq!(get("received_date"), "");
    assert_eq!(get("counterparty_name"), "Odběratel a.s.");
    assert_eq!(get("counterparty_ico"), "12345679");
    assert_eq!(get("exchange_rate"), "");
    assert_eq!((get("base_21"), get("vat_21")), ("1000,00", "210,40"));
    assert_eq!((get("base_12"), get("base_0")), ("", ""));
    assert_eq!(get("rounding"), "-0,40");
    assert_eq!((get("total"), get("total_czk")), ("1210,00", "1210,00"));
    assert_eq!(get("paid_date"), "");
    assert_eq!(get("vat_deductible"), "");
    assert_eq!(get("category"), "Služby");
    assert_eq!(get("note"), "Hlavička");
}

#[test]
fn received_foreign_credit_note_is_negative() {
    let mut parent = doc();
    parent.direction = "received".into();
    parent.supplier_number = Some("FV-1".into());
    let mut doc = doc();
    doc.direction = "received".into();
    doc.doc_type = "credit_note".into();
    doc.number = Some("PD-1".into());
    doc.supplier_number = Some("DOB-7".into());
    doc.currency = "EUR".into();
    doc.exchange_rate = Some(d("24.335000"));
    doc.rounding = Decimal::ZERO;
    doc.total = d("121");
    doc.payable = d("121");
    doc.total_czk = Some(d("2944.54"));
    doc.vat_deductible = false;
    let mut r = recap(&doc, "21", "100", "21");
    r.base_czk = Some(d("2433.50"));
    r.vat_czk = Some(d("511.04"));
    let mut legacy = recap(&doc, "12", "10", "1.2");
    legacy.vat_czk = Some(d("29.20"));
    let c = row(&Source {
        doc: &doc,
        recap: &[r, legacy],
        payments: &[],
        parent: Some(&parent),
        category: None,
    });
    let get = |n| cell(&c, n);
    assert_eq!(get("direction"), "received");
    assert_eq!((get("number"), get("supplier_number")), ("PD-1", "DOB-7"));
    assert_eq!(get("related_number"), "FV-1");
    assert_eq!(get("received_date"), "02.03.2026");
    assert_eq!(get("counterparty_name"), "Dodavatel s.r.o.");
    assert_eq!((get("currency"), get("exchange_rate")), ("EUR", "24,335"));
    assert_eq!((get("base_21"), get("vat_21")), ("-2433,50", "-511,04"));
    assert_eq!((get("base_12"), get("vat_12")), ("-243,35", "-29,20"));
    assert_eq!((get("total"), get("total_czk")), ("-121,00", "-2944,54"));
    assert_eq!(get("rounding"), "0,00");
    assert_eq!(get("vat_deductible"), "0");
    assert_eq!(get("note"), "Interní");
}

#[test]
fn contactless_simplified_has_no_counterparty() {
    let mut doc = doc();
    doc.doc_type = "simplified".into();
    doc.customer_snapshot = None;
    let c = row(&Source {
        doc: &doc,
        recap: &[],
        payments: &[],
        parent: None,
        category: None,
    });
    assert_eq!(cell(&c, "counterparty_name"), "");
    assert_eq!(cell(&c, "counterparty_country"), "");
}

#[test]
fn paid_date_is_the_payment_completing_it() {
    let ps = [pay(9, "500", 2), pay(3, "700", 1), pay(9, "10", 3)];
    let at = |paid, payable| paid_date("invoice", Status::Issued, d(paid), d(payable), &ps);
    assert_eq!(at("1210", "1200"), Some(date(4, 9)), "overpaid");
    assert_eq!(at("1200", "1200"), Some(date(4, 9)));
    assert_eq!(at("1200", "700"), Some(date(4, 3)), "first one suffices");
    assert_eq!(at("1200", "1300"), None, "partial");
    assert_eq!(
        paid_date("invoice", Status::Issued, d("0"), d("0"), &[]),
        None,
        "settled without a payment"
    );
    assert_eq!(
        paid_date("invoice", Status::Cancelled, d("1200"), d("1200"), &ps),
        None
    );
    assert_eq!(
        paid_date("advance_tax_doc", Status::Issued, d("0"), d("1200"), &[]),
        None
    );
}

#[test]
fn unknown_stored_values_fail() {
    let mut bad = doc();
    bad.direction = "sideways".into();
    let s = |d| Source {
        doc: d,
        recap: &[],
        payments: &[],
        parent: None,
        category: None,
    };
    assert!(out_row(&s(&bad)).is_err());
    let mut bad = doc();
    bad.customer_snapshot = Some(json!("x"));
    assert!(out_row(&s(&bad)).is_err());
    // Foreign, no stored CZK amount and no rate: never guessed.
    let mut bad = doc();
    bad.currency = "EUR".into();
    let r = [recap(&bad, "21", "100", "21")];
    let source = Source {
        recap: &r,
        ..s(&bad)
    };
    assert!(out_row(&source).is_err());
}
