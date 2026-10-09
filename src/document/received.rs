//! Pure totals of a received document: the VAT recap is entered as on the
//! supplier's document (never recomputed from base × rate), `payable` too.

use rust_decimal::Decimal;

use super::compute::{Overflow, RecapRow, Totals, fits, round2};

/// One entered recap row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnteredRow {
    pub rate: Decimal,
    pub base: Decimal,
    pub vat: Decimal,
}

/// `total = Σ(base + vat)` (as for issued documents; the entered `rounding`
/// is kept beside it, and `total + rounding` must fit); `payable` as entered; CZK amounts
/// (foreign currency with a rate) `round2(x × rate)` per row and for the
/// payable. Rows sorted by rate, highest first. Checked arithmetic.
pub fn totals(
    rows: &[EnteredRow],
    rounding: Decimal,
    payable: Decimal,
    rate: Option<Decimal>,
) -> Result<Totals, Overflow> {
    use Overflow::{ExchangeRate, Lines};
    let czk = |x: Decimal| -> Result<Option<Decimal>, Overflow> {
        rate.map(|r| {
            x.checked_mul(r)
                .map(round2)
                .and_then(fits)
                .ok_or(ExchangeRate)
        })
        .transpose()
    };
    let add = |a: Decimal, b: Decimal| a.checked_add(b).and_then(fits).ok_or(Lines);
    let mut sorted = rows.to_vec();
    sorted.sort_by_key(|r| std::cmp::Reverse(r.rate));
    let (mut base, mut vat) = (Decimal::ZERO, Decimal::ZERO);
    let mut recap = Vec::with_capacity(sorted.len());
    for r in sorted {
        base = add(base, r.base)?;
        vat = add(vat, r.vat)?;
        recap.push(RecapRow {
            vat_rate: r.rate.normalize(),
            base: round2(r.base),
            vat: round2(r.vat),
            base_czk: czk(r.base)?,
            vat_czk: czk(r.vat)?,
        });
    }
    let total = add(base, vat)?;
    add(total, rounding)?;
    Ok(Totals {
        recap,
        base: round2(base),
        vat: round2(vat),
        total: round2(total),
        rounding: round2(rounding),
        payable: round2(payable),
        total_czk: czk(payable)?,
    })
}

/// The stored amounts of a received document (recap by rate, highest first).
pub struct Stored<'a> {
    pub recap: &'a [RecapRow],
    pub rounding: Decimal,
    pub payable: Decimal,
    pub total_czk: Option<Decimal>,
    pub rate: Option<Decimal>,
}

/// A save that changes none of the entered amounts nor the rate keeps the
/// stored CZK amounts. They equal `round2(x × rate)` for a document entered
/// by hand, but an ISDOC import stores the supplier's own CZK amounts, which
/// a no-op save must not move by a rounding difference.
pub fn keep_stored_czk(t: &mut Totals, rate: Option<Decimal>, s: Stored) {
    let same_rows = t.recap.len() == s.recap.len()
        && t.recap.iter().zip(s.recap).all(|(a, b)| {
            a.vat_rate.normalize() == b.vat_rate.normalize() && a.base == b.base && a.vat == b.vat
        });
    if !(same_rows && t.rounding == s.rounding && t.payable == s.payable && rate == s.rate) {
        return;
    }
    for (a, b) in t.recap.iter_mut().zip(s.recap) {
        a.base_czk = b.base_czk;
        a.vat_czk = b.vat_czk;
    }
    t.total_czk = s.total_czk;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::line::MAX_AMOUNT;

    fn d(s: &str) -> Decimal {
        s.parse().expect("decimal")
    }

    fn row(rate: &str, base: &str, vat: &str) -> EnteredRow {
        EnteredRow {
            rate: d(rate),
            base: d(base),
            vat: d(vat),
        }
    }

    #[test]
    fn sums_rows_and_rounding() {
        let t = totals(
            &[row("12", "100", "12"), row("21", "1000", "210.01")],
            d("-0.01"),
            d("1322"),
            None,
        )
        .expect("totals");
        assert_eq!(t.recap[0].vat_rate, d("21"));
        assert_eq!(t.recap[1].vat_rate, d("12"));
        assert_eq!((t.base, t.vat), (d("1100.00"), d("222.01")));
        assert_eq!(t.total, d("1322.01"));
        assert_eq!((t.rounding, t.payable), (d("-0.01"), d("1322.00")));
        assert_eq!(t.total_czk, None);
        assert_eq!(t.recap[0].base_czk, None);
    }

    #[test]
    fn converts_to_czk() {
        let t = totals(
            &[row("21", "100", "21")],
            d("0"),
            d("121"),
            Some(d("24.335")),
        )
        .expect("totals");
        assert_eq!(t.recap[0].base_czk, Some(d("2433.50")));
        assert_eq!(t.recap[0].vat_czk, Some(d("511.04")));
        assert_eq!(t.total_czk, Some(d("2944.54")));
    }

    #[test]
    fn unchanged_amounts_keep_the_stored_czk() {
        let fresh = || {
            totals(
                &[row("21", "100", "21")],
                d("0"),
                d("121"),
                Some(d("24.335")),
            )
            .expect("totals")
        };
        let mut stored = fresh().recap;
        stored[0].base_czk = Some(d("2433.51"));
        let s = |rate: &str| Stored {
            recap: &stored,
            rounding: d("0.00"),
            payable: d("121.00"),
            total_czk: Some(d("2944.55")),
            rate: Some(d(rate)),
        };
        let mut t = fresh();
        keep_stored_czk(&mut t, Some(d("24.335")), s("24.335"));
        assert_eq!(t.total_czk, Some(d("2944.55")));
        assert_eq!(t.recap[0].base_czk, Some(d("2433.51")));
        // Another rate, payable or recap: recomputed.
        let mut t = fresh();
        keep_stored_czk(&mut t, Some(d("24.335")), s("25"));
        assert_eq!(t.total_czk, Some(d("2944.54")));
        let mut t = totals(
            &[row("21", "100", "21")],
            d("0"),
            d("120"),
            Some(d("24.335")),
        )
        .expect("t");
        keep_stored_czk(&mut t, Some(d("24.335")), s("24.335"));
        assert_eq!(t.total_czk, Some(d("2920.20")));
    }

    #[test]
    fn overflow_is_an_error() {
        let big = MAX_AMOUNT - Decimal::ONE;
        assert_eq!(
            totals(
                &[row("0", &big.to_string(), "0"), row("21", "10", "0")],
                d("0"),
                d("0"),
                None
            ),
            Err(Overflow::Lines)
        );
        assert_eq!(
            totals(
                &[row("21", "1000000000000", "0")],
                d("0"),
                d("1"),
                Some(d("100000"))
            ),
            Err(Overflow::ExchangeRate)
        );
    }
}
