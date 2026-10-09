//! The CSV / XLSX interchange format shared by the import (2b) and the
//! export (2c): column names, the dynamic rate columns, and how numbers,
//! dates and booleans are read and written. Pure.

use chrono::NaiveDate;
use rust_decimal::Decimal;

pub const DIRECTION: &str = "direction";
pub const DOC_TYPE: &str = "doc_type";
pub const NUMBER: &str = "number";
pub const SUPPLIER_NUMBER: &str = "supplier_number";
pub const RELATED_NUMBER: &str = "related_number";
pub const ISSUE_DATE: &str = "issue_date";
pub const TAX_DATE: &str = "tax_date";
pub const DUE_DATE: &str = "due_date";
pub const RECEIVED_DATE: &str = "received_date";
pub const COUNTERPARTY_NAME: &str = "counterparty_name";
pub const COUNTERPARTY_ICO: &str = "counterparty_ico";
pub const COUNTERPARTY_DIC: &str = "counterparty_dic";
pub const COUNTERPARTY_STREET: &str = "counterparty_street";
pub const COUNTERPARTY_CITY: &str = "counterparty_city";
pub const COUNTERPARTY_ZIP: &str = "counterparty_zip";
pub const COUNTERPARTY_COUNTRY: &str = "counterparty_country";
pub const CURRENCY: &str = "currency";
pub const EXCHANGE_RATE: &str = "exchange_rate";
pub const VAT_MODE: &str = "vat_mode";
pub const ROUNDING: &str = "rounding";
pub const TOTAL: &str = "total";
pub const TOTAL_CZK: &str = "total_czk";
pub const PAID_DATE: &str = "paid_date";
pub const VARIABLE_SYMBOL: &str = "variable_symbol";
pub const VAT_DEDUCTIBLE: &str = "vat_deductible";
pub const CATEGORY: &str = "category";
pub const NOTE: &str = "note";

/// The fixed columns written before the rate columns, in export order.
pub const BEFORE_RATES: [&str; 19] = [
    DIRECTION,
    DOC_TYPE,
    NUMBER,
    SUPPLIER_NUMBER,
    RELATED_NUMBER,
    ISSUE_DATE,
    TAX_DATE,
    DUE_DATE,
    RECEIVED_DATE,
    COUNTERPARTY_NAME,
    COUNTERPARTY_ICO,
    COUNTERPARTY_DIC,
    COUNTERPARTY_STREET,
    COUNTERPARTY_CITY,
    COUNTERPARTY_ZIP,
    COUNTERPARTY_COUNTRY,
    CURRENCY,
    EXCHANGE_RATE,
    VAT_MODE,
];

/// The fixed columns written after the rate columns, in export order.
pub const AFTER_RATES: [&str; 8] = [
    ROUNDING,
    TOTAL,
    TOTAL_CZK,
    PAID_DATE,
    VARIABLE_SYMBOL,
    VAT_DEDUCTIBLE,
    CATEGORY,
    NOTE,
];

pub fn is_fixed(name: &str) -> bool {
    BEFORE_RATES.contains(&name) || AFTER_RATES.contains(&name)
}

/// `{r}` of a rate column: the decimal point as `_`, trailing zeros dropped
/// (`21`, `12_5`, `0`).
pub fn rate_key(rate: Decimal) -> String {
    rate.normalize().to_string().replace('.', "_")
}

pub fn base_column(rate: Decimal) -> String {
    format!("base_{}", rate_key(rate))
}

pub fn vat_column(rate: Decimal) -> String {
    format!("vat_{}", rate_key(rate))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RatePart {
    Base,
    Vat,
}

/// A header naming a rate column: `None` when it is no rate column at all,
/// `Some(Err(()))` when it starts like one but the rate does not parse
/// (0–100, at most 2 dp; `_`, `.` or `,` as the decimal mark).
pub fn rate_column(name: &str) -> Option<Result<(RatePart, Decimal), ()>> {
    let (part, rest) = match name.strip_prefix("base_") {
        Some(r) => (RatePart::Base, r),
        None => (RatePart::Vat, name.strip_prefix("vat_")?),
    };
    let ok = !rest.is_empty()
        && rest
            .bytes()
            .all(|b| b.is_ascii_digit() || b"_.,".contains(&b))
        && rest.bytes().filter(|b| b"_.,".contains(b)).count() <= 1
        && rest.as_bytes()[0].is_ascii_digit()
        && rest.as_bytes()[rest.len() - 1].is_ascii_digit();
    let rate = ok
        .then(|| rest.replace(['_', ','], ".").parse::<Decimal>().ok())
        .flatten()
        .map(|d| d.normalize())
        .filter(|r| *r <= Decimal::ONE_HUNDRED && r.scale() <= 2);
    Some(rate.map(|r| (part, r)).ok_or(()))
}

fn is_space(c: char) -> bool {
    matches!(c, ' ' | '\u{a0}' | '\u{202f}' | '\t')
}

/// A number with a decimal comma or dot; spaces / NBSP between digit groups
/// are ignored; when both `,` and `.` occur the last one is the decimal mark
/// and the other one separates thousands. `None` when it does not parse.
pub fn parse_decimal(s: &str) -> Option<Decimal> {
    let s: String = s.chars().filter(|c| !is_space(*c)).collect();
    let (neg, body) = match s.strip_prefix(['-', '\u{2212}']) {
        Some(rest) => (true, rest),
        None => (false, s.strip_prefix('+').unwrap_or(&s)),
    };
    if body.is_empty()
        || !body
            .chars()
            .all(|c| c.is_ascii_digit() || c == ',' || c == '.')
    {
        return None;
    }
    let plain = match body.rfind([',', '.']) {
        None => body.to_string(),
        Some(i) => {
            let decimal = char::from(body.as_bytes()[i]);
            let thousands = if decimal == ',' { '.' } else { ',' };
            if body.matches(decimal).count() > 1 {
                return None;
            }
            body.replace(thousands, "").replace(decimal, ".")
        }
    };
    if plain.starts_with('.') || plain.ends_with('.') {
        return None;
    }
    let d: Decimal = plain.parse().ok()?;
    Some(if neg { -d } else { d })
}

/// An Excel numeric cell: Excel keeps 15 significant digits, so binary
/// noise beyond them (`0.1 + 0.2` = 0.30000000000000004) is dropped.
pub fn excel_number(f: f64) -> Option<Decimal> {
    if !f.is_finite() {
        return None;
    }
    Decimal::from_scientific(&format!("{f:.14e}"))
        .ok()
        .map(|d| d.normalize())
}

/// `d.m.yyyy` (spaces after the dots allowed) or `yyyy-mm-dd`.
pub fn parse_date(s: &str) -> Option<NaiveDate> {
    let s = s.trim();
    let num = |p: &str, max: usize| -> Option<u32> {
        let p = p.trim();
        (!p.is_empty() && p.len() <= max && p.bytes().all(|b| b.is_ascii_digit()))
            .then(|| p.parse().ok())
            .flatten()
    };
    let parts: Vec<&str> = if s.contains('-') {
        s.split('-').collect()
    } else {
        s.split('.').rev().collect()
    };
    let [y, m, d] = parts[..] else { return None };
    if y.trim().len() != 4 {
        return None;
    }
    let y = i32::try_from(num(y, 4)?).ok()?;
    NaiveDate::from_ymd_opt(y, num(m, 2)?, num(d, 2)?)
}

/// `1`/`0`, `ano`/`ne`, `true`/`false`, `yes`/`no` (any case).
pub fn parse_bool(s: &str) -> Option<bool> {
    match s.trim().to_lowercase().as_str() {
        "1" | "ano" | "true" | "yes" => Some(true),
        "0" | "ne" | "false" | "no" => Some(false),
        _ => None,
    }
}

/// Leading characters a spreadsheet may evaluate as a formula.
const FORMULA_START: [char; 6] = ['=', '+', '-', '@', '\t', '\r'];

/// Starts like a formula, possibly behind guards already (so a text that
/// itself begins with `'=` survives the round trip).
fn needs_guard(s: &str) -> bool {
    s.trim_start_matches('\'').starts_with(FORMULA_START)
}

/// A free-text cell as written: one `'` before anything a spreadsheet would
/// run as a formula (CSV injection). Amounts and dates are never guarded.
pub fn guard_text(s: &str) -> String {
    if needs_guard(s) {
        format!("'{s}")
    } else {
        s.to_string()
    }
}

/// The inverse of [`guard_text`] on import: one leading `'` is dropped when
/// a formula start follows.
pub fn unguard_text(s: &str) -> &str {
    match s.strip_prefix('\'') {
        Some(rest) if needs_guard(rest) => rest,
        _ => s,
    }
}

/// An amount as written: 2 dp, decimal comma, no thousands separators.
pub fn format_amount(x: Decimal) -> String {
    // A negated zero would print as `-0.00`.
    let mut x = if x.is_zero() {
        Decimal::ZERO
    } else {
        x.round_dp(2)
    };
    x.rescale(2);
    x.to_string().replace('.', ",")
}

/// An exchange rate as written: decimal comma, trailing zeros dropped.
pub fn format_rate(x: Decimal) -> String {
    x.normalize().to_string().replace('.', ",")
}

/// `dd.mm.yyyy`.
pub fn format_date(d: NaiveDate) -> String {
    d.format("%d.%m.%Y").to_string()
}

pub fn format_bool(b: bool) -> &'static str {
    if b { "1" } else { "0" }
}

#[cfg(test)]
#[path = "format_tests.rs"]
mod tests;
