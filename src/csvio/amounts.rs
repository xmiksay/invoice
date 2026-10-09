//! A row's VAT recap (in CZK) and total → the stored totals. Pure.

use rust_decimal::Decimal;

use crate::document::compute::{RecapRow, Totals, fits, round2};

/// One recap row in CZK (signs already normalised: never negative).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CzkRow {
    pub rate: Decimal,
    pub base: Decimal,
    pub vat: Decimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmountError {
    /// `total_mismatch`.
    Mismatch,
    /// The rate turns an amount into one no money column holds.
    Overflow,
}

fn div2(x: Decimal, rate: Decimal) -> Result<Decimal, AmountError> {
    x.checked_div(rate)
        .map(round2)
        .and_then(fits)
        .ok_or(AmountError::Overflow)
}

/// CZK: the recap as given, `Σ(base + vat) + rounding` must equal `total`.
/// Foreign currency (`rate` = CZK per unit): per row `round2(czk / rate)`;
/// the difference to `total − rounding` goes to the VAT of the highest rate
/// with VAT (else the base of the highest rate) when it is at most 0.01 per
/// row. `payable` = `total`; `totalCzk` = `round2(payable × rate)`.
pub fn totals(
    rows: &[CzkRow],
    rounding: Decimal,
    total: Decimal,
    rate: Option<Decimal>,
) -> Result<Totals, AmountError> {
    let mut sorted = rows.to_vec();
    sorted.sort_by_key(|r| std::cmp::Reverse(r.rate));
    let mut recap = Vec::with_capacity(sorted.len());
    for r in &sorted {
        recap.push(match rate {
            None => RecapRow {
                vat_rate: r.rate.normalize(),
                base: round2(r.base),
                vat: round2(r.vat),
                base_czk: None,
                vat_czk: None,
            },
            Some(k) => RecapRow {
                vat_rate: r.rate.normalize(),
                base: div2(r.base, k)?,
                vat: div2(r.vat, k)?,
                base_czk: Some(round2(r.base)),
                vat_czk: Some(round2(r.vat)),
            },
        });
    }
    // Amounts are < 10^12 and rows few: these sums cannot overflow.
    let sum = |recap: &[RecapRow]| recap.iter().map(|r| r.base + r.vat).sum::<Decimal>();
    let diff = total - rounding - sum(&recap);
    if !diff.is_zero() {
        let allowed = Decimal::new(1, 2) * Decimal::from(recap.len());
        if rate.is_none() || diff.abs() > allowed {
            return Err(AmountError::Mismatch);
        }
        // Candidates: the VAT of each rate with VAT, then each base, highest
        // rate first; the first one that stays ≥ 0 takes the difference.
        let vats = recap
            .iter()
            .enumerate()
            .filter(|(_, r)| !r.vat.is_zero())
            .map(|(i, r)| (i, true, r.vat));
        let bases = recap.iter().enumerate().map(|(i, r)| (i, false, r.base));
        let target = vats
            .chain(bases)
            .find(|(_, _, x)| *x + diff >= Decimal::ZERO)
            .ok_or(AmountError::Mismatch)?;
        let r = &mut recap[target.0];
        if target.1 {
            r.vat = round2(r.vat + diff);
        } else {
            r.base = round2(r.base + diff);
        }
    }
    let base: Decimal = recap.iter().map(|r| r.base).sum();
    let vat: Decimal = recap.iter().map(|r| r.vat).sum();
    let total_czk = match rate {
        Some(k) => Some(
            total
                .checked_mul(k)
                .map(round2)
                .and_then(fits)
                .ok_or(AmountError::Overflow)?,
        ),
        None => None,
    };
    Ok(Totals {
        recap,
        base: round2(base),
        vat: round2(vat),
        total: round2(base + vat),
        rounding: round2(rounding),
        payable: round2(total),
        total_czk,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Decimal {
        s.parse().expect("decimal")
    }

    fn row(rate: &str, base: &str, vat: &str) -> CzkRow {
        CzkRow {
            rate: d(rate),
            base: d(base),
            vat: d(vat),
        }
    }

    #[test]
    fn czk_must_add_up_exactly() {
        let rows = [row("12", "100", "12"), row("21", "1000", "210")];
        let t = totals(&rows, d("0.40"), d("1322.40"), None).expect("totals");
        assert_eq!(t.recap[0].vat_rate, d("21"));
        assert_eq!((t.base, t.vat, t.total), (d("1100"), d("222"), d("1322")));
        assert_eq!((t.rounding, t.payable), (d("0.40"), d("1322.40")));
        assert_eq!(t.recap[0].base_czk, None);
        assert_eq!(t.total_czk, None);
        assert_eq!(
            totals(&rows, d("0"), d("1322.01"), None),
            Err(AmountError::Mismatch)
        );
    }

    #[test]
    fn foreign_recap_is_derived_half_away_from_zero() {
        // 2433.50 / 24.335 = 100; 511.04 / 24.335 = 21.0002…
        let rows = [row("21", "2433.50", "511.04")];
        let t = totals(&rows, d("0"), d("121"), Some(d("24.335"))).expect("totals");
        assert_eq!((t.recap[0].base, t.recap[0].vat), (d("100.00"), d("21.00")));
        assert_eq!(t.recap[0].base_czk, Some(d("2433.50")));
        assert_eq!(t.recap[0].vat_czk, Some(d("511.04")));
        assert_eq!(t.payable, d("121.00"));
        assert_eq!(t.total_czk, Some(d("2944.54")));
        // 0.125 → 0.13 (away from zero).
        let t = totals(&[row("0", "0.25", "0")], d("0"), d("0.13"), Some(d("2"))).expect("t");
        assert_eq!(t.recap[0].base, d("0.13"));
    }

    #[test]
    fn foreign_difference_goes_to_the_highest_vat() {
        let rows = [
            row("21", "2500", "525"),
            row("12", "250", "30"),
            row("0", "100", "0"),
        ];
        // 100 + 21 + 10 + 1.2 + 4 = 136.20; the total says 136.22.
        let t = totals(&rows, d("0"), d("136.22"), Some(d("25"))).expect("t");
        assert_eq!(t.recap[0].vat, d("21.02"));
        assert_eq!(t.total, d("136.22"));
        // More than 0.01 per row → mismatch.
        assert_eq!(
            totals(&rows, d("0"), d("136.24"), Some(d("25"))),
            Err(AmountError::Mismatch)
        );
        // No VAT anywhere: the base of the highest rate.
        let t = totals(
            &[row("21", "250", "0"), row("0", "100", "0")],
            d("0"),
            d("13.99"),
            Some(d("25")),
        )
        .expect("t");
        assert_eq!((t.recap[0].base, t.recap[1].base), (d("9.99"), d("4.00")));
    }

    #[test]
    fn the_difference_never_turns_an_amount_negative() {
        // 0.20 / 25 = 0.01 and 0.10 / 25 = 0.00: the sum is 110.01, the total
        // 109.99. VAT 21 would go to −0.01, VAT 12 is 0: the base of 21 % takes it.
        let rows = [row("21", "2500", "0.20"), row("12", "250", "0.10")];
        let t = totals(&rows, d("0"), d("109.99"), Some(d("25"))).expect("t");
        assert_eq!((t.recap[0].base, t.recap[0].vat), (d("99.98"), d("0.01")));
        assert_eq!((t.recap[1].base, t.recap[1].vat), (d("10.00"), d("0.00")));
        assert!(
            t.recap
                .iter()
                .all(|r| r.base >= Decimal::ZERO && r.vat >= Decimal::ZERO)
        );
        assert_eq!(t.total, d("109.99"));
        // Nothing can take it: mismatch.
        assert_eq!(
            totals(&[row("0", "0.10", "0")], d("0"), d("-0.01"), Some(d("25"))),
            Err(AmountError::Mismatch)
        );
    }

    #[test]
    fn huge_conversions_overflow() {
        assert_eq!(
            totals(
                &[row("21", "999999999999", "0")],
                d("0"),
                d("1"),
                Some(d("0.000001"))
            ),
            Err(AmountError::Overflow)
        );
    }
}
