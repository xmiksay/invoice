//! Field validators shared by the issued and received document bodies.

use rust_decimal::Decimal;

use super::line_input::decimal;
use crate::document::line::{MAX_RATE, VatMode};
use crate::validation::{self as v, Check};

/// Digits only, at most `max` (variable / constant symbol).
pub fn digits(s: Option<&str>, max: usize) -> Check<Option<String>> {
    let s = v::opt_text(s, max)?;
    match s {
        Some(d) if !d.bytes().all(|c| c.is_ascii_digit()) => Err("invalid"),
        other => Ok(other),
    }
}

pub fn exchange_rate(s: Option<&str>) -> Check<Option<Decimal>> {
    match s.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(s) => {
            let r = decimal(s, 6, MAX_RATE)?;
            if r <= Decimal::ZERO {
                Err("invalid")
            } else {
                Ok(Some(r))
            }
        }
    }
}

pub fn parse_vat_mode(s: &str) -> Check<VatMode> {
    VatMode::parse(s.trim()).ok_or("invalid")
}
