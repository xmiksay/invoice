//! Unit tests of the payload builder: header, parties, lines, QR.

use super::*;
use crate::document::line::Status;
use crate::pdf::test_fixtures::*;

#[test]
fn standard_invoice() {
    let f = standard();
    let p = build(&f.input()).expect("payload");
    assert_eq!(p.title, "Faktura – daňový doklad");
    assert_eq!(p.number.as_deref(), Some("20260001"));
    assert!(p.show_vat && p.has_discount && !p.draft);
    assert_eq!(p.watermark, None);
    assert_eq!(p.supplier.lines, ["Hlavní 1", "110 00 Praha"]);
    assert_eq!(p.supplier.ids, ["IČO: 44444443"]);
    assert_eq!(p.supplier.contact, ["a@example.com"]);
    let customer = p.customer.as_ref().expect("customer");
    assert_eq!(customer.lines.last().map(String::as_str), Some("Německo"));
    let dates: Vec<&str> = p.dates.iter().map(|r| r.label.as_str()).collect();
    assert_eq!(
        dates,
        [
            "Datum vystavení",
            "Datum zdanitelného plnění",
            "Datum splatnosti"
        ]
    );
    let pay: Vec<(&str, &str)> = p
        .payment
        .iter()
        .map(|r| (r.label.as_str(), r.value.as_str()))
        .collect();
    assert_eq!(
        pay,
        [
            ("Způsob platby", "Bankovní převod"),
            ("Číslo účtu", "19-2000145399/0800"),
            ("IBAN", "CZ65 0800 0000 1920 0014 5399"),
            ("BIC/SWIFT", "GIBACZPX"),
            ("Variabilní symbol", "20260001"),
        ]
    );
    // Members 2 and 3 of the collapsed group are hidden.
    let kinds: Vec<(&str, bool)> = p.lines.iter().map(|l| (l.kind, l.strong)).collect();
    assert_eq!(
        kinds,
        [
            ("item", false),
            ("subtotal", false),
            ("text", false),
            ("subtotal", true)
        ]
    );
    let first = &p.lines[0];
    assert_eq!(first.quantity, Some(format!("2{NB}ks")));
    assert_eq!(first.unit_price, Some(format!("100,00{NB}Kč")));
    assert_eq!(first.discount, Some(format!("10{NB}%")));
    assert_eq!(first.vat_rate, Some(format!("21{NB}%")));
    assert_eq!(p.lines[1].quantity, None);
    assert_eq!(p.lines[1].base, Some(format!("150,00{NB}Kč")));
    assert_eq!(p.lines[2].base, None);
    let recap = p.vat_recap.as_ref().expect("recap");
    assert_eq!(recap.rows[0].total, format!("399,30{NB}Kč"));
    assert_eq!(recap.total.rate, "Celkem");
    assert_eq!(p.vat_recap_czk, None);
    assert_eq!(
        values(&p.totals),
        [
            ("Celkem bez DPH", "330,00\u{a0}Kč", false),
            ("DPH", "69,30\u{a0}Kč", false),
            ("Celkem s DPH", "399,30\u{a0}Kč", false),
            ("Zaokrouhlení", "0,70\u{a0}Kč", false),
            ("K úhradě", "400,00\u{a0}Kč", true),
        ]
    );
    assert_eq!(p.legal_note, None);
    assert_eq!(p.paid_note, None);
    assert_eq!(p.header_note.as_deref(), Some("Hlavička"));
    assert_eq!(p.footer.registration.as_deref(), Some("C 1 u MS v Praze"));
    assert_eq!(p.qr.as_ref().map(|q| q.image), Some("qr.svg"));
    let spayd = p.spayd.as_deref().expect("spayd");
    assert!(spayd.contains("*AM:400.00*CC:CZK*DT:20261022*X-VS:20260001*"));
    let json = serde_json::to_value(&p).expect("json");
    assert!(json.get("spayd").is_none());
    assert_eq!(json["vatRecapCzk"], serde_json::Value::Null);
    assert_eq!(
        json["assets"],
        serde_json::json!({ "logo": null, "signature": null })
    );
}

#[test]
fn nested_collapsed_subtotals_hide_their_members() {
    let lines = vec![
        item(1, "1", "0", "21", "2"),
        subtotal(2, vec![1], false, "2"),
        item(3, "1", "0", "21", "2"),
        subtotal(4, vec![2, 3], true, "4"),
    ];
    let f = Fixture::new(lines, totals("4", "0.84", "0", "4.84"));
    let p = build(&f.input()).expect("payload");
    let desc: Vec<&str> = p.lines.iter().map(|l| l.description.as_str()).collect();
    assert_eq!(desc, ["Group 4"]);
}

#[test]
fn draft_has_watermark_and_no_qr() {
    let f = standard();
    let mut i = f.input();
    i.status = Status::Draft;
    i.number = None;
    let p = build(&i).expect("payload");
    assert!(p.draft);
    assert_eq!(p.watermark.as_deref(), Some("NÁVRH"));
    assert_eq!(p.qr, None);
    assert_eq!(p.spayd, None);
    assert_eq!(p.number, None);
}

#[test]
fn qr_needs_bank_transfer_iban_and_payable() {
    let f = standard();
    let mut i = f.input();
    i.payment_method = "cash";
    let p = build(&i).expect("payload");
    assert_eq!(p.qr, None);
    assert_eq!(p.payment.len(), 2, "no bank rows for cash");
    let mut no_iban = standard();
    no_iban.bank.iban = None;
    assert_eq!(build(&no_iban.input()).expect("payload").qr, None);
    let paid = Fixture::new(vec![], totals("0", "0", "0", "0"));
    assert_eq!(build(&paid.input()).expect("payload").qr, None);
}

#[test]
fn legal_notes_and_missing_customer() {
    let f = standard();
    let mut i = f.input();
    i.vat_mode = VatMode::Exempt;
    i.customer = None;
    // Issued without a customer snapshot → no block; a draft without a
    // contact → an empty party.
    assert_eq!(build(&i).expect("payload").customer, None);
    i.status = Status::Draft;
    let p = build(&i).expect("payload");
    assert_eq!(p.legal_note.as_deref(), Some("Plnění osvobozené od DPH."));
    let customer = p.customer.expect("empty party");
    assert_eq!(customer.title, "Odběratel");
    assert!(customer.name.is_empty() && customer.lines.is_empty());
    // A simplified document without a customer prints no customer block,
    // nor do its corrections.
    i.doc_type = "simplified";
    assert_eq!(build(&i).expect("payload").customer, None);
    i.doc_type = "credit_note";
    i.parent_doc_type = Some("simplified");
    assert_eq!(build(&i).expect("payload").customer, None);
    i.status = Status::Issued;
    assert_eq!(build(&i).expect("payload").customer, None);
    i.status = Status::Draft;
    i.parent_doc_type = Some("invoice");
    assert!(
        build(&i).expect("payload").customer.is_some(),
        "draft keeps the empty party"
    );
}

#[test]
fn correction_references_and_titles() {
    let f = standard();
    let mut i = f.input();
    i.doc_type = "debit_note";
    i.parent_number = Some("ZD20260001");
    i.parent_doc_type = Some("simplified");
    i.correction_reason = Some("Doúčtování");
    let p = build(&i).expect("payload");
    assert_eq!(p.title, "Opravný daňový doklad – vrubopis");
    assert_eq!(
        p.reference.as_deref(),
        Some(
            "Opravný daňový doklad – vrubopis k zjednodušenému daňovému dokladu ZD20260001\nDůvod opravy: Doúčtování"
        )
    );
    assert!(p.qr.is_some(), "a debit note is payable");
    i.doc_type = "advance_credit_note";
    i.parent_doc_type = Some("advance_tax_doc");
    let p = build(&i).expect("payload");
    assert_eq!(p.qr, None, "a refund has no QR");
    assert!(p.totals.iter().any(|t| t.value.starts_with('-')), "negated");
}

#[test]
fn cancelled_has_storno_watermark_and_no_qr() {
    let f = standard();
    let mut i = f.input();
    i.status = Status::Cancelled;
    let p = build(&i).expect("payload");
    assert!(!p.draft);
    assert_eq!(p.watermark.as_deref(), Some("STORNO"));
    assert_eq!(p.qr, None);
    assert_eq!(p.number.as_deref(), Some("20260001"));
    i.locale = Locale::En;
    let p = build(&i).expect("payload");
    assert_eq!(p.watermark.as_deref(), Some("CANCELLED"));
}
