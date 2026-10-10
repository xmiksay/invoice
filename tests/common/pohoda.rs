//! Pohoda XML export: requests, XSD validation against the vendored
//! Stormware schemas, and the fixture of every exported type in both
//! directions shared with the Money S3 tests (fictitious parties:
//! 44444443 / 12345679 / 87654326).

use axum::Router;
use axum::http::StatusCode;

use super::csv_export::get_raw;
use super::csvio::{file, row_in};

pub const JANUARY: &str = "/api/export/accountant?from=2026-01-01&to=2026-01-31&format=pohoda";

/// `xmllint --schema data.xsd` (the vendored Pohoda schemas, version 2).
pub fn assert_valid(xml: &[u8]) {
    super::assert_xsd(xml, "pohoda/schema/data.xsd", &decode(xml));
}

/// The file as text (it is Windows-1250).
pub fn decode(bytes: &[u8]) -> String {
    let (text, _, bad) = encoding_rs::WINDOWS_1250.decode(bytes);
    assert!(!bad, "not Windows-1250");
    text.into_owned()
}

/// GET a Pohoda export; asserts 200, the headers and XSD validity, and
/// returns the decoded text.
pub async fn export(app: &Router, uri: &str) -> String {
    let (status, headers, bytes) = get_raw(app, uri).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&bytes)
    );
    assert_eq!(
        headers["content-type"],
        "application/xml; charset=windows-1250"
    );
    assert_valid(&bytes);
    decode(&bytes)
}

/// The `dat:dataPackItem`s of a file, in order.
pub fn items(xml: &str) -> Vec<&str> {
    xml.split("<dat:dataPackItem ").skip(1).collect()
}

/// The item whose `text` element is `text`.
pub fn item<'a>(xml: &'a str, text: &str) -> &'a str {
    let needle = format!(":text>{text}</");
    items(xml)
        .into_iter()
        .find(|i| i.contains(&needle))
        .unwrap_or_else(|| panic!("no item {text} in\n{xml}"))
}

/// The columns of [`fixture`] (and of extra rows built with `row_in`).
pub const HEADER: &str = "direction;doc_type;number;supplier_number;related_number;issue_date;\
tax_date;due_date;received_date;counterparty_name;counterparty_ico;counterparty_dic;\
counterparty_street;counterparty_city;counterparty_zip;counterparty_country;currency;\
exchange_rate;vat_mode;base_21;vat_21;base_12;vat_12;base_10;vat_10;base_0;rounding;total;\
variable_symbol;vat_deductible";

/// January 2026, issued (customer 12345679): invoice `FA-1`, credit note
/// `DB-1` and debit note `VB-1` to it, proforma `ZF-1` (never exported),
/// DDPP `DD-1` and its correction `OD-1`, contactless simplified `ZJ-1`,
/// EUR invoice `FA-3` (a German customer). Received (supplier 87654326):
/// EUR invoice `FV-42` (VAT not deductible), CZK invoice `FV-44` with credit note `FV-45` and
/// debit note `FV-46`, DDPP `FV-47` with correction `FV-48`, simplified
/// `FV-49`, and `FV-50` at 10 % (no Pohoda slot until a third rate is set).
pub fn fixture() -> Vec<u8> {
    let r = |pairs: &[(&str, &str)]| row_in(HEADER, pairs);
    let issued = |rest: &[(&'static str, &'static str)]| {
        let mut p = vec![
            ("direction", "issued"),
            ("counterparty_name", "Fiktivní Odběratel s.r.o."),
            ("counterparty_ico", "12345679"),
            ("counterparty_dic", "CZ12345679"),
            ("counterparty_street", "Zkušební 12"),
            ("counterparty_city", "Brno"),
            ("counterparty_zip", "60200"),
            ("issue_date", "15.01.2026"),
        ];
        p.extend(rest);
        r(&p)
    };
    let received = |rest: &[(&'static str, &'static str)]| {
        let mut p = vec![
            ("direction", "received"),
            ("counterparty_name", "Vzorový Dodavatel a.s."),
            ("counterparty_ico", "87654326"),
            ("counterparty_dic", "CZ87654326"),
            ("issue_date", "20.01.2026"),
            ("received_date", "22.01.2026"),
        ];
        p.extend(rest);
        r(&p)
    };
    let rows = [
        issued(&[
            ("doc_type", "invoice"),
            ("number", "FA-1"),
            ("variable_symbol", "1"),
            ("due_date", "29.01.2026"),
            ("base_21", "1000"),
            ("vat_21", "210"),
            ("total", "1210"),
        ]),
        issued(&[
            ("doc_type", "credit_note"),
            ("number", "DB-1"),
            ("related_number", "FA-1"),
            ("base_21", "-100"),
            ("vat_21", "-21"),
            ("total", "-121"),
        ]),
        issued(&[
            ("doc_type", "debit_note"),
            ("number", "VB-1"),
            ("related_number", "FA-1"),
            ("base_21", "50"),
            ("vat_21", "10,50"),
            ("total", "60,50"),
        ]),
        issued(&[
            ("doc_type", "proforma"),
            ("number", "ZF-1"),
            ("base_21", "500"),
            ("vat_21", "105"),
            ("total", "605"),
        ]),
        issued(&[
            ("doc_type", "advance_tax_doc"),
            ("number", "DD-1"),
            ("related_number", "ZF-1"),
            ("base_21", "500"),
            ("vat_21", "105"),
            ("total", "605"),
        ]),
        issued(&[
            ("doc_type", "advance_credit_note"),
            ("number", "OD-1"),
            ("related_number", "DD-1"),
            ("base_21", "-100"),
            ("vat_21", "-21"),
            ("total", "-121"),
        ]),
        r(&[
            ("direction", "issued"),
            ("doc_type", "simplified"),
            ("number", "ZJ-1"),
            ("issue_date", "16.01.2026"),
            ("base_12", "100"),
            ("vat_12", "12,40"),
            ("rounding", "-0,40"),
            ("total", "112"),
        ]),
        r(&[
            ("direction", "issued"),
            ("doc_type", "invoice"),
            ("number", "FA-3"),
            ("issue_date", "05.01.2026"),
            ("counterparty_name", "Foreign Customer GmbH"),
            ("counterparty_street", "Улица 1"),
            ("counterparty_city", "München"),
            ("counterparty_country", "DE"),
            ("currency", "EUR"),
            ("exchange_rate", "25"),
            ("base_21", "2500"),
            ("vat_21", "525"),
            ("total", "121"),
        ]),
        received(&[
            ("doc_type", "invoice"),
            ("supplier_number", "FV-42"),
            ("vat_deductible", "ne"),
            ("currency", "EUR"),
            ("exchange_rate", "24,335"),
            ("base_21", "2433,50"),
            ("vat_21", "511,04"),
            ("total", "121,00"),
        ]),
        received(&[
            ("doc_type", "invoice"),
            ("supplier_number", "FV-44"),
            ("base_21", "2000"),
            ("vat_21", "420"),
            ("total", "2420"),
        ]),
        received(&[
            ("doc_type", "credit_note"),
            ("supplier_number", "FV-45"),
            ("related_number", "FV-44"),
            ("base_21", "-200"),
            ("vat_21", "-42"),
            ("total", "-242"),
        ]),
        received(&[
            ("doc_type", "debit_note"),
            ("supplier_number", "FV-46"),
            ("related_number", "FV-44"),
            ("base_21", "100"),
            ("vat_21", "21"),
            ("total", "121"),
        ]),
        received(&[
            ("doc_type", "advance_tax_doc"),
            ("supplier_number", "FV-47"),
            ("base_21", "1000"),
            ("vat_21", "210"),
            ("total", "1210"),
        ]),
        received(&[
            ("doc_type", "advance_credit_note"),
            ("supplier_number", "FV-48"),
            ("related_number", "FV-47"),
            ("base_21", "-100"),
            ("vat_21", "-21"),
            ("total", "-121"),
        ]),
        received(&[
            ("doc_type", "simplified"),
            ("supplier_number", "FV-49"),
            ("base_0", "100"),
            ("total", "100"),
        ]),
        received(&[
            ("doc_type", "invoice"),
            ("supplier_number", "FV-50"),
            ("base_10", "100"),
            ("vat_10", "10"),
            ("total", "110"),
        ]),
    ];
    let refs: Vec<&str> = rows.iter().map(String::as_str).collect();
    file(HEADER, &refs)
}
