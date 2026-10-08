//! Pure money computation for documents: line bases, the per-rate VAT recap
//! (§37 ZDPH — VAT per rate, not per line), rounding and CZK conversion.
//! Authoritative: the stored totals and the compute endpoint both come from here.
//!
//! All arithmetic is checked: `Decimal`'s operators panic on overflow, and
//! huge inputs must end as a 422, never a crashed request.

use rust_decimal::{Decimal, RoundingStrategy};

use super::line::{LineData, MAX_AMOUNT, VatMode};
use super::subtotals;
use crate::error::FieldErrors;

pub fn round2(x: Decimal) -> Decimal {
    let mut r = x.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero);
    r.rescale(2);
    r
}

fn round0(x: Decimal) -> Decimal {
    round2(x.round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero))
}

/// `round2(quantity × unitPrice × (1 − discount/100))`, half away from zero;
/// `None` on overflow or when the base would not fit a money column.
pub fn item_base(quantity: Decimal, unit_price: Decimal, discount_pct: Decimal) -> Option<Decimal> {
    let base = quantity
        .checked_mul(unit_price)?
        .checked_mul(Decimal::ONE_HUNDRED.checked_sub(discount_pct)?)?
        .checked_div(Decimal::ONE_HUNDRED)?;
    fits(round2(base))
}

/// `Some(x)` when `x` fits a `numeric(18,2)` column.
fn fits(x: Decimal) -> Option<Decimal> {
    (x.abs() < MAX_AMOUNT).then_some(x)
}

/// Which input made the totals unrepresentable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overflow {
    Lines,
    ExchangeRate,
}

impl Overflow {
    pub fn field(self) -> &'static str {
        match self {
            Overflow::Lines => "lines",
            Overflow::ExchangeRate => "exchangeRate",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecapRow {
    pub vat_rate: Decimal,
    pub base: Decimal,
    pub vat: Decimal,
    pub base_czk: Option<Decimal>,
    pub vat_czk: Option<Decimal>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Totals {
    /// Sorted by rate, highest first.
    pub recap: Vec<RecapRow>,
    pub base: Decimal,
    pub vat: Decimal,
    pub total: Decimal,
    pub rounding: Decimal,
    pub payable: Decimal,
    pub total_czk: Option<Decimal>,
}

#[derive(Debug, Clone, Copy)]
pub struct Params {
    pub vat_mode: VatMode,
    pub is_czk: bool,
    /// CZK per unit of a foreign currency, when known.
    pub exchange_rate: Option<Decimal>,
    pub round_total: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evaluated {
    /// `(vat_rate, base)` per line; `None` for text lines.
    pub lines: subtotals::Resolved,
    pub totals: Totals,
}

/// Validate the computation rules and compute everything. Field errors use
/// `lines.<index>.<field>` with the 0-based index of the line; amounts too
/// large to store are `lines: invalid` (or `exchangeRate: invalid` when only
/// the CZK conversion overflows).
pub fn evaluate(lines: &[LineData], p: Params) -> Result<Evaluated, FieldErrors> {
    let mut e = FieldErrors::new();
    if p.round_total && !p.is_czk {
        e.add("roundTotal", "invalid");
    }
    let mut bases = Vec::with_capacity(lines.len());
    for (i, line) in lines.iter().enumerate() {
        let base = match line {
            LineData::Item(item) => {
                if p.vat_mode == VatMode::NonPayer && !item.vat_rate.is_zero() {
                    e.add(&format!("lines.{i}.vatRate"), "invalid");
                }
                item_base(item.quantity, item.unit_price, item.discount_pct).unwrap_or_else(|| {
                    e.add("lines", "invalid");
                    Decimal::ZERO
                })
            }
            _ => Decimal::ZERO,
        };
        bases.push(base);
    }
    let resolved = subtotals::resolve(lines, &bases);
    if let Err(i) = &resolved {
        e.add(&format!("lines.{i}.refs"), "invalid");
    }
    let resolved = match resolved {
        Ok(r) if e.is_empty() => r,
        _ => return Err(e),
    };
    let items = lines.iter().zip(&bases).filter_map(|(l, b)| match l {
        LineData::Item(item) => Some((item.vat_rate, *b)),
        _ => None,
    });
    let totals = totals(items, p).map_err(|o| {
        e.add(o.field(), "invalid");
        e.clone()
    })?;
    Ok(Evaluated {
        lines: resolved,
        totals,
    })
}

/// Recap + totals from `(vat_rate, base)` of the item lines.
pub fn totals(
    items: impl IntoIterator<Item = (Decimal, Decimal)>,
    p: Params,
) -> Result<Totals, Overflow> {
    use Overflow::{ExchangeRate, Lines};
    let mut groups: Vec<(Decimal, Decimal)> = Vec::new();
    for (rate, base) in items {
        match groups.iter_mut().find(|(r, _)| *r == rate) {
            Some((_, sum)) => *sum = sum.checked_add(base).ok_or(Lines)?,
            None => groups.push((rate, base)),
        }
    }
    groups.sort_by_key(|g| std::cmp::Reverse(g.0));
    let fx = if p.is_czk { None } else { p.exchange_rate };
    let czk = |x: Decimal| -> Result<Option<Decimal>, Overflow> {
        fx.map(|r| {
            x.checked_mul(r)
                .map(round2)
                .and_then(fits)
                .ok_or(ExchangeRate)
        })
        .transpose()
    };
    let mut recap = Vec::with_capacity(groups.len());
    let (mut base_sum, mut vat_sum) = (Decimal::ZERO, Decimal::ZERO);
    for (rate, base) in groups {
        let base = fits(round2(base)).ok_or(Lines)?;
        let vat = if p.vat_mode.charges_vat() {
            round2(base.checked_mul(rate).ok_or(Lines)? / Decimal::ONE_HUNDRED)
        } else {
            round2(Decimal::ZERO)
        };
        base_sum = base_sum.checked_add(base).ok_or(Lines)?;
        vat_sum = vat_sum.checked_add(vat).ok_or(Lines)?;
        recap.push(RecapRow {
            vat_rate: rate.normalize(),
            base,
            vat,
            base_czk: czk(base)?,
            vat_czk: czk(vat)?,
        });
    }
    let base = fits(round2(base_sum)).ok_or(Lines)?;
    let vat = fits(round2(vat_sum)).ok_or(Lines)?;
    let total = fits(round2(base.checked_add(vat).ok_or(Lines)?)).ok_or(Lines)?;
    let payable = if p.round_total && p.is_czk {
        fits(round0(total)).ok_or(Lines)?
    } else {
        total
    };
    Ok(Totals {
        recap,
        base,
        vat,
        total,
        rounding: round2(payable - total),
        payable,
        total_czk: czk(payable)?,
    })
}

#[cfg(test)]
#[path = "compute_tests.rs"]
mod tests;
