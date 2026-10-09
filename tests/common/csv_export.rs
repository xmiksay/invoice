//! CSV export requests, a minimal reader of the exported file, and an
//! "import everything" helper for building fixtures through the 2b import.

use axum::Router;
use axum::http::{HeaderMap, StatusCode};
use serde_json::Value;

use super::csvio::{confirm, preview};
use super::{TEST_TOKEN, get, send};

/// An exported file split into header and rows (fixtures never quote).
pub struct Csv {
    pub header: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl Csv {
    fn index(&self, name: &str) -> usize {
        self.header
            .iter()
            .position(|h| h == name)
            .unwrap_or_else(|| panic!("no column {name} in {:?}", self.header))
    }

    pub fn cell(&self, row: usize, name: &str) -> &str {
        &self.rows[row][self.index(name)]
    }

    pub fn column(&self, name: &str) -> Vec<&str> {
        let i = self.index(name);
        self.rows.iter().map(|r| r[i].as_str()).collect()
    }

    /// The row whose `column` is `value`.
    pub fn find(&self, column: &str, value: &str) -> usize {
        let i = self.index(column);
        self.rows
            .iter()
            .position(|r| r[i] == value)
            .unwrap_or_else(|| panic!("no row with {column} = {value}"))
    }

    /// The rate columns of the header (`base_*` / `vat_*`).
    pub fn rate_columns(&self) -> Vec<&str> {
        self.header
            .iter()
            .filter(|h| h.starts_with("base_") || h.starts_with("vat_"))
            .filter(|h| *h != "vat_mode" && *h != "vat_deductible")
            .map(String::as_str)
            .collect()
    }
}

pub fn parse(bytes: &[u8]) -> Csv {
    let text = std::str::from_utf8(bytes).expect("UTF-8");
    let text = text.strip_prefix('\u{feff}').expect("BOM");
    let body = text.strip_suffix("\r\n").expect("ends with CRLF");
    let mut lines = body.split("\r\n").map(|l| {
        assert!(!l.contains('"'), "fixtures never need quoting: {l}");
        l.split(';').map(str::to_string).collect::<Vec<_>>()
    });
    let header = lines.next().expect("header");
    let rows: Vec<Vec<String>> = lines.collect();
    for r in &rows {
        assert_eq!(r.len(), header.len(), "{r:?}");
    }
    Csv { header, rows }
}

pub async fn get_raw(app: &Router, uri: &str) -> (StatusCode, HeaderMap, Vec<u8>) {
    let resp = send(app.clone(), get(uri, Some(TEST_TOKEN))).await;
    let status = resp.status();
    let headers = resp.headers().clone();
    let bytes = axum::body::to_bytes(resp.into_body(), 1 << 26)
        .await
        .expect("read body");
    (status, headers, bytes.to_vec())
}

/// GET an export; asserts 200 + the CSV headers and returns the raw body.
pub async fn export_bytes(app: &Router, uri: &str) -> Vec<u8> {
    let (status, headers, bytes) = get_raw(app, uri).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&bytes)
    );
    assert_eq!(headers["content-type"], "text/csv; charset=utf-8");
    bytes
}

pub async fn export(app: &Router, uri: &str) -> Csv {
    parse(&export_bytes(app, uri).await)
}

/// The JSON error of a failed export.
pub async fn export_error(app: &Router, uri: &str) -> (StatusCode, Value) {
    let (status, _, bytes) = get_raw(app, uri).await;
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// Preview + confirm every row of `file`; asserts each one is imported and
/// returns the results.
pub async fn import_all(app: &Router, file: &[u8]) -> Vec<Value> {
    let entries = preview(app, file).await;
    for e in &entries {
        assert_eq!(e["status"], "ok", "{e}");
    }
    let keys: Vec<&str> = entries.iter().filter_map(|e| e["key"].as_str()).collect();
    let results = confirm(app, file, &keys).await;
    for r in &results {
        assert_eq!(r["status"], "imported", "{r}");
    }
    results
}

/// Issued `FA-1` (paid, category), its credit note `DB-1`, a proforma
/// `ZF-1`, a EUR invoice `FA-3` and a received EUR invoice `FV-42`.
pub fn fixture() -> Vec<u8> {
    use super::csvio::{HEADER, file, row};
    let customer = [
        ("counterparty_name", "Fiktivní Odběratel s.r.o."),
        ("counterparty_ico", "12345679"),
    ];
    let issued = |rest: &[(&'static str, &'static str)]| {
        let mut r = vec![("direction", "issued")];
        r.extend(customer);
        r.extend(rest);
        row(&r)
    };
    let rows = [
        issued(&[
            ("doc_type", "invoice"),
            ("number", "FA-1"),
            ("issue_date", "15.01.2026"),
            ("due_date", "29.01.2026"),
            ("base_21", "1000,00"),
            ("vat_21", "210,00"),
            ("total", "1210,00"),
            ("paid_date", "27.01.2026"),
            ("variable_symbol", "1"),
            ("category", "Služby"),
            ("note", "Děkujeme"),
        ]),
        issued(&[
            ("doc_type", "credit_note"),
            ("number", "DB-1"),
            ("related_number", "FA-1"),
            ("issue_date", "02.02.2026"),
            ("base_21", "-100,00"),
            ("vat_21", "-21,00"),
            ("total", "-121,00"),
        ]),
        issued(&[
            ("doc_type", "proforma"),
            ("number", "ZF-1"),
            ("issue_date", "10.01.2026"),
            ("base_21", "500"),
            ("vat_21", "105"),
            ("total", "605"),
        ]),
        row(&[
            ("direction", "issued"),
            ("doc_type", "invoice"),
            ("number", "FA-3"),
            ("issue_date", "05.01.2026"),
            ("counterparty_name", "Foreign Customer GmbH"),
            ("currency", "EUR"),
            ("exchange_rate", "25"),
            ("base_21", "2500"),
            ("vat_21", "525"),
            ("total", "121"),
        ]),
        row(&[
            ("direction", "received"),
            ("doc_type", "invoice"),
            ("supplier_number", "FV-42"),
            ("issue_date", "20.01.2026"),
            ("received_date", "22.01.2026"),
            ("counterparty_name", "Vzorový Dodavatel a.s."),
            ("counterparty_ico", "87654326"),
            ("counterparty_dic", "CZ87654326"),
            ("currency", "EUR"),
            ("exchange_rate", "24,335"),
            ("base_21", "2433,50"),
            ("vat_21", "511,04"),
            ("total", "121,00"),
            ("vat_deductible", "ne"),
            ("category", "Software"),
        ]),
    ];
    let refs: Vec<&str> = rows.iter().map(String::as_str).collect();
    file(HEADER, &refs)
}
