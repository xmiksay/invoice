use chrono::{NaiveDate, Utc};

use super::*;

fn date(m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, m, d).expect("date")
}

fn company(vat_payer: bool) -> company::Model {
    company::Model {
        id: 1,
        name: "Me".into(),
        ico: None,
        dic: None,
        vat_payer,
        street: String::new(),
        city: String::new(),
        zip: String::new(),
        country: "CZ".into(),
        email: None,
        phone: None,
        web: None,
        registration: None,
        default_due_days: 14,
        default_locale: "cs".into(),
        updated_at: Utc::now().into(),
    }
}

fn ctx(apply_defaults: bool, contact: Option<contact::Model>) -> Context {
    Context {
        today: date(10, 8),
        apply_defaults,
        company: company(true),
        contact,
        default_vat_rate: Some(Decimal::from(21)),
    }
}

fn contact_with_defaults() -> contact::Model {
    contact::Model {
        id: Uuid::from_u128(7),
        name: "C".into(),
        ico: None,
        dic: None,
        street: String::new(),
        city: String::new(),
        zip: String::new(),
        country: "CZ".into(),
        email: None,
        phone: None,
        note: None,
        default_due_days: Some(30),
        default_locale: Some("en".into()),
        default_currency: Some("EUR".into()),
        created_at: Utc::now().into(),
        updated_at: Utc::now().into(),
    }
}

fn fields(err: AppError) -> FieldErrors {
    match err {
        AppError::Validation(f) => f,
        other => panic!("expected validation, got {other:?}"),
    }
}

#[test]
fn post_defaults_from_company() {
    let (d, ev) = DocumentInput::default()
        .validate(&ctx(true, None))
        .expect("valid");
    assert_eq!(d.issue_date, date(10, 8));
    assert_eq!(d.tax_point_date, Some(date(10, 8)));
    assert_eq!(d.due_date, date(10, 22));
    assert_eq!((d.currency.as_str(), d.locale.as_str()), ("CZK", "cs"));
    assert_eq!(d.vat_mode, VatMode::Standard);
    assert_eq!(d.payment_method, PaymentMethod::BankTransfer);
    assert!(!d.round_total);
    assert!(ev.totals.recap.is_empty());
}

#[test]
fn post_prefers_contact_defaults() {
    let c = contact_with_defaults();
    let input = DocumentInput {
        contact_id: Some(c.id),
        issue_date: Some(date(10, 1)),
        exchange_rate: Some("25.1".into()),
        ..Default::default()
    };
    let (d, _) = input.validate(&ctx(true, Some(c))).expect("valid");
    assert_eq!(d.due_date, date(10, 31));
    assert_eq!((d.currency.as_str(), d.locale.as_str()), ("EUR", "en"));
    assert_eq!(d.exchange_rate, Some("25.1".parse().expect("d")));
}

#[test]
fn put_requires_fields_and_keeps_nulls() {
    let err = DocumentInput::default()
        .validate(&ctx(false, None))
        .expect_err("required");
    let mut expected = FieldErrors::new();
    for f in [
        "issueDate",
        "dueDate",
        "currency",
        "locale",
        "vatMode",
        "paymentMethod",
    ] {
        expected.add(f, "required");
    }
    assert_eq!(fields(err), expected);
}

#[test]
fn rejects_unknown_contact_and_other_doc_types() {
    let input = DocumentInput {
        contact_id: Some(Uuid::from_u128(1)),
        direction: Some("received".into()),
        exchange_rate: Some("0".into()),
        ..Default::default()
    };
    let mut expected = FieldErrors::new();
    expected.add("docType", "invalid");
    expected.add("contactId", "invalid");
    expected.add("exchangeRate", "invalid");
    assert_eq!(
        fields(input.validate(&ctx(true, None)).expect_err("e")),
        expected
    );
}

#[test]
fn czk_ignores_the_exchange_rate() {
    let input = DocumentInput {
        currency: Some("czk".into()),
        exchange_rate: Some("25".into()),
        ..Default::default()
    };
    let (d, _) = input.validate(&ctx(true, None)).expect("valid");
    assert_eq!((d.currency.as_str(), d.exchange_rate), ("CZK", None));
}

#[test]
fn compute_input_defaults() {
    let input = ComputeInput {
        lines: vec![LineInput {
            kind: "item".into(),
            quantity: Some("2".into()),
            unit_price: Some("10".into()),
            ..Default::default()
        }],
        ..Default::default()
    };
    let (lines, ev) = input
        .clone()
        .validate(true, Some(Decimal::from(21)))
        .expect("valid");
    assert_eq!(lines.len(), 1);
    assert_eq!(ev.totals.vat, "4.20".parse().expect("d"));
    let (_, ev) = input
        .validate(false, Some(Decimal::from(21)))
        .expect("valid");
    assert_eq!(
        ev.totals.vat,
        Decimal::ZERO,
        "non-payer company → 0 % lines"
    );
}
