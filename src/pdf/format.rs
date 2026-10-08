//! Locale-specific display strings for the PDF payload: money, quantities,
//! percentages, exchange rates and dates. The template does no formatting.

use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;

use crate::document::compute::round2;

/// Non-breaking space: keeps "1 234,50 Kč" on one line.
pub const NBSP: char = '\u{a0}';

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    Cs,
    En,
}

impl Locale {
    pub fn parse(s: &str) -> Option<Locale> {
        match s {
            "cs" => Some(Locale::Cs),
            "en" => Some(Locale::En),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Locale::Cs => "cs",
            Locale::En => "en",
        }
    }

    fn separators(self) -> (char, char) {
        match self {
            Locale::Cs => (NBSP, ','),
            Locale::En => (',', '.'),
        }
    }
}

/// `"1234567"` → `"1 234 567"` with `sep` between groups of three.
fn group(int: &str, sep: char) -> String {
    let len = int.chars().count();
    let mut out = String::with_capacity(len + len / 3);
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            out.push(sep);
        }
        out.push(c);
    }
    out
}

/// `|x|` as a plain decimal string (no exponent), grouped, with the locale's
/// separators; the sign is returned separately.
fn number(x: Decimal, locale: Locale) -> (bool, String) {
    let (thousands, decimal) = locale.separators();
    let negative = x.is_sign_negative() && !x.is_zero();
    let plain = x.abs().to_string();
    let (int, frac) = plain.split_once('.').unwrap_or((plain.as_str(), ""));
    let mut out = group(int, thousands);
    if !frac.is_empty() {
        out.push(decimal);
        out.push_str(frac);
    }
    (negative, out)
}

fn signed(x: Decimal, locale: Locale) -> String {
    let (negative, digits) = number(x, locale);
    if negative {
        format!("-{digits}")
    } else {
        digits
    }
}

/// `cs`: `1 234,50 Kč` (symbol for CZK/EUR/USD, else the code after);
/// `en`: `CZK 1,234.50`.
pub fn money(x: Decimal, currency: &str, locale: Locale) -> String {
    let amount = signed(round2(x), locale);
    match locale {
        Locale::Cs => {
            let unit = match currency {
                "CZK" => "Kč",
                "EUR" => "€",
                "USD" => "$",
                other => other,
            };
            format!("{amount}{NBSP}{unit}")
        }
        Locale::En => format!("{currency}{NBSP}{amount}"),
    }
}

/// Trailing zeros trimmed, then the unit: `2 ks`, `1,5 h`.
pub fn quantity(q: Decimal, unit: Option<&str>, locale: Locale) -> String {
    let n = signed(q.normalize(), locale);
    match unit.map(str::trim).filter(|u| !u.is_empty()) {
        Some(u) => format!("{n}{NBSP}{u}"),
        None => n,
    }
}

/// `cs` `21 %`, `en` `21%`.
pub fn percent(p: Decimal, locale: Locale) -> String {
    let n = signed(p.normalize(), locale);
    match locale {
        Locale::Cs => format!("{n}{NBSP}%"),
        Locale::En => format!("{n}%"),
    }
}

/// An exchange rate with at least three decimals (`24,400`).
pub fn rate(r: Decimal, locale: Locale) -> String {
    let mut r = r.normalize();
    if r.scale() < 3 {
        r.rescale(3);
    }
    signed(r, locale)
}

const MONTHS_EN: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// `cs` `8. 10. 2026`, `en` `8 Oct 2026`.
pub fn date(d: NaiveDate, locale: Locale) -> String {
    match locale {
        Locale::Cs => format!("{}. {}. {}", d.day(), d.month(), d.year()),
        Locale::En => {
            let month = MONTHS_EN
                .get(d.month0() as usize)
                .copied()
                .unwrap_or_default();
            format!("{} {month} {}", d.day(), d.year())
        }
    }
}

/// IBAN in groups of four for reading (`CZ65 0800 …`).
pub fn iban(s: &str) -> String {
    let compact: Vec<char> = s.chars().filter(|c| !c.is_whitespace()).collect();
    compact
        .chunks(4)
        .map(|c| c.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Decimal {
        s.parse().expect("decimal")
    }

    #[test]
    fn money_cs_and_en() {
        assert_eq!(
            money(d("1234.5"), "CZK", Locale::Cs),
            "1\u{a0}234,50\u{a0}Kč"
        );
        assert_eq!(money(d("1234.5"), "CZK", Locale::En), "CZK\u{a0}1,234.50");
        assert_eq!(money(d("0"), "EUR", Locale::Cs), "0,00\u{a0}€");
        assert_eq!(money(d("99.999"), "USD", Locale::Cs), "100,00\u{a0}$");
        assert_eq!(money(d("12"), "GBP", Locale::Cs), "12,00\u{a0}GBP");
        assert_eq!(
            money(d("-1234567.891"), "EUR", Locale::En),
            "EUR\u{a0}-1,234,567.89"
        );
        assert_eq!(money(d("-0.001"), "CZK", Locale::Cs), "0,00\u{a0}Kč");
        assert_eq!(money(d("-100"), "CZK", Locale::Cs), "-100,00\u{a0}Kč");
    }

    #[test]
    fn quantities_are_trimmed() {
        assert_eq!(quantity(d("2.0000"), Some("ks"), Locale::Cs), "2\u{a0}ks");
        assert_eq!(quantity(d("1.5"), Some("h"), Locale::Cs), "1,5\u{a0}h");
        assert_eq!(quantity(d("1.5"), None, Locale::En), "1.5");
        assert_eq!(quantity(d("1500"), Some(" "), Locale::En), "1,500");
        assert_eq!(quantity(d("1500.25"), None, Locale::Cs), "1\u{a0}500,25");
    }

    #[test]
    fn percent_and_rate() {
        assert_eq!(percent(d("21.00"), Locale::Cs), "21\u{a0}%");
        assert_eq!(percent(d("12.5"), Locale::Cs), "12,5\u{a0}%");
        assert_eq!(percent(d("10"), Locale::En), "10%");
        assert_eq!(rate(d("24.4"), Locale::Cs), "24,400");
        assert_eq!(rate(d("24.385000"), Locale::En), "24.385");
        assert_eq!(rate(d("0.161235"), Locale::En), "0.161235");
    }

    #[test]
    fn dates() {
        let day = NaiveDate::from_ymd_opt(2026, 10, 8).expect("date");
        assert_eq!(date(day, Locale::Cs), "8. 10. 2026");
        assert_eq!(date(day, Locale::En), "8 Oct 2026");
    }

    #[test]
    fn grouping_and_iban() {
        assert_eq!(group("1", ' '), "1");
        assert_eq!(group("123", ' '), "123");
        assert_eq!(group("1234", ' '), "1 234");
        assert_eq!(group("123456", ' '), "123 456");
        assert_eq!(
            iban("CZ6508000000192000145399"),
            "CZ65 0800 0000 1920 0014 5399"
        );
        assert_eq!(Locale::parse("de"), None);
        assert_eq!(Locale::parse("en").map(Locale::as_str), Some("en"));
    }
}
