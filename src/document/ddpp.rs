//! Pure DDPP amounts: a received advance payment split by the proforma's VAT
//! rates with VAT computed "from above" (§37 ZDPH).
//!
//! A DDPP's recap comes only from [`totals`] here, never from the
//! `base × rate` path in `compute.rs`: DDPPs are created issued and never pass
//! through save or issue, and reads return the stored recap.

use rust_decimal::Decimal;

use super::compute::{Overflow, RecapRow, Totals, fits, round2};

/// One rate's share of the payment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Share {
    pub vat_rate: Decimal,
    pub base: Decimal,
    pub vat: Decimal,
}

impl Share {
    pub fn gross(&self) -> Option<Decimal> {
        self.base.checked_add(self.vat)
    }
}

/// Split `amount` by the proforma recap `(rate, base, vat)`: gross per rate
/// `G_r = base_r + vat_r`; every rate except the highest-gross one gets
/// `round2(amount × G_r / ΣG)`, the highest-gross rate (ties: the higher
/// rate) gets the remainder so the shares sum to `amount` exactly. Then
/// `vat_r = round2(P_r × rate / (100 + rate))`, `base_r = P_r − vat_r`.
/// Zero shares are dropped. A non-positive `ΣG` gives everything to the
/// highest-gross rate. `None` on overflow.
pub fn split(amount: Decimal, recap: &[(Decimal, Decimal, Decimal)]) -> Option<Vec<Share>> {
    let gross: Vec<(Decimal, Decimal)> = recap
        .iter()
        .map(|(rate, base, vat)| base.checked_add(*vat).map(|g| (*rate, g)))
        .collect::<Option<_>>()?;
    let total = gross
        .iter()
        .try_fold(Decimal::ZERO, |acc, (_, g)| acc.checked_add(*g))?;
    let top = gross
        .iter()
        .enumerate()
        .max_by(|(_, (rate_a, gross_a)), (_, (rate_b, gross_b))| {
            gross_a.cmp(gross_b).then(rate_a.cmp(rate_b))
        })
        .map(|(i, _)| i)?;
    let mut parts = vec![Decimal::ZERO; gross.len()];
    if total > Decimal::ZERO {
        for (i, (_, g)) in gross.iter().enumerate() {
            if i != top {
                parts[i] = round2(amount.checked_mul(*g)?.checked_div(total)?);
            }
        }
    }
    let others = parts
        .iter()
        .try_fold(Decimal::ZERO, |acc, p| acc.checked_add(*p))?;
    parts[top] = round2(amount.checked_sub(others)?);
    gross
        .iter()
        .zip(parts)
        .filter(|(_, p)| !p.is_zero())
        .map(|((rate, _), p)| {
            let vat = round2(
                p.checked_mul(*rate)?
                    .checked_div(Decimal::ONE_HUNDRED.checked_add(*rate)?)?,
            );
            let base = round2(p.checked_sub(vat)?);
            Some(Share {
                vat_rate: rate.normalize(),
                base,
                vat,
            })
        })
        .collect()
}

/// The DDPP's stored totals: the recap is exactly the shares (sorted rate
/// desc), CZK amounts at `rate` when the currency is foreign.
pub fn totals(shares: &[Share], rate: Option<Decimal>) -> Result<Totals, Overflow> {
    let czk = |x: Decimal| -> Result<Option<Decimal>, Overflow> {
        rate.map(|r| {
            x.checked_mul(r)
                .map(round2)
                .and_then(fits)
                .ok_or(Overflow::ExchangeRate)
        })
        .transpose()
    };
    let mut sorted = shares.to_vec();
    sorted.sort_by_key(|s| std::cmp::Reverse(s.vat_rate));
    let mut recap = Vec::with_capacity(sorted.len());
    let (mut base, mut vat) = (Decimal::ZERO, Decimal::ZERO);
    for s in sorted {
        base = base.checked_add(s.base).ok_or(Overflow::Lines)?;
        vat = vat.checked_add(s.vat).ok_or(Overflow::Lines)?;
        recap.push(RecapRow {
            vat_rate: s.vat_rate,
            base: s.base,
            vat: s.vat,
            base_czk: czk(s.base)?,
            vat_czk: czk(s.vat)?,
        });
    }
    let total =
        fits(round2(base.checked_add(vat).ok_or(Overflow::Lines)?)).ok_or(Overflow::Lines)?;
    Ok(Totals {
        recap,
        base: round2(base),
        vat: round2(vat),
        total,
        rounding: round2(Decimal::ZERO),
        payable: total,
        total_czk: czk(total)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Decimal {
        s.parse().expect("decimal")
    }

    fn share(rate: &str, base: &str, vat: &str) -> Share {
        Share {
            vat_rate: d(rate),
            base: d(base),
            vat: d(vat),
        }
    }

    #[test]
    fn single_rate_vat_from_above() {
        // 1210 incl. 21 %: 1210 × 21/121 = 210.
        let s = split(d("1210"), &[(d("21"), d("1000"), d("210"))]).expect("split");
        assert_eq!(s, vec![share("21", "1000.00", "210.00")]);
        // 100 incl. 21 %: 17.355… → 17.36, base 82.64 (not 100/1.21 × 0.21 per base).
        let s = split(d("100"), &[(d("21"), d("1000"), d("210"))]).expect("split");
        assert_eq!(s, vec![share("21", "82.64", "17.36")]);
    }

    #[test]
    fn splits_proportionally_with_remainder_to_highest_gross() {
        // Gross: 21 % → 121, 12 % → 224 (highest), T = 345; pay 100.
        // 21 %: round2(100 × 121 / 345) = 35.07; 12 % gets 64.93.
        let recap = [(d("21"), d("100"), d("21")), (d("12"), d("200"), d("24"))];
        let s = split(d("100"), &recap).expect("split");
        // 35.07 × 21/121 = 6.0865… → 6.09; 64.93 × 12/112 = 6.9567… → 6.96.
        assert_eq!(
            s,
            vec![share("21", "28.98", "6.09"), share("12", "57.97", "6.96")]
        );
        let sum: Decimal = s.iter().map(|x| x.gross().expect("g")).sum();
        assert_eq!(sum, d("100"));
    }

    #[test]
    fn remainder_absorbs_rounding_of_three_equal_rates() {
        let recap = [
            (d("21"), d("100"), d("21")),
            (d("12"), d("108.04"), d("12.96")),
            (d("0"), d("121"), d("0")),
        ];
        // All gross 121 → tie: the highest rate (21 %) is "highest gross".
        let s = split(d("100"), &recap).expect("split");
        let parts: Vec<Decimal> = s.iter().map(|x| x.gross().expect("g")).collect();
        assert_eq!(parts, vec![d("33.34"), d("33.33"), d("33.33")]);
        assert_eq!(s[2], share("0", "33.33", "0.00"));
    }

    #[test]
    fn zero_shares_are_dropped_and_zero_total_goes_to_top() {
        let recap = [(d("21"), d("100"), d("21")), (d("12"), d("0"), d("0"))];
        let s = split(d("50"), &recap).expect("split");
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].vat_rate, d("21"));
        let recap = [
            (d("21"), d("10"), d("2.10")),
            (d("12"), d("-10"), d("-2.10")),
        ];
        let s = split(d("50"), &recap).expect("split");
        assert_eq!(
            s.iter().map(|x| x.vat_rate).collect::<Vec<_>>(),
            vec![d("21")]
        );
        assert!(split(d("1"), &[]).is_none());
    }

    #[test]
    fn remainder_goes_to_highest_gross_not_highest_rate() {
        // 21 % gross 12.10 (smallest); 12 % and 0 % both gross 500 → tie → 12 %.
        let recap = [
            (d("21"), d("10"), d("2.10")),
            (d("12"), d("446.43"), d("53.57")),
            (d("0"), d("500"), d("0")),
        ];
        // T = 1012.10; 21 %: round2(100 × 12.10 / 1012.10) = 1.20;
        // 0 %: round2(100 × 500 / 1012.10) = 49.40; 12 % gets 49.40.
        let s = split(d("100"), &recap).expect("split");
        let parts: Vec<(Decimal, Decimal)> = s
            .iter()
            .map(|x| (x.vat_rate, x.gross().expect("g")))
            .collect();
        assert_eq!(
            parts,
            vec![
                (d("21"), d("1.20")),
                (d("12"), d("49.40")),
                (d("0"), d("49.40"))
            ]
        );
        // A single dominant lower rate takes the rounding remainder.
        let recap = [(d("21"), d("1"), d("0.21")), (d("12"), d("100"), d("12"))];
        let s = split(d("10"), &recap).expect("split");
        // 21 %: round2(10 × 1.21 / 113.21) = 0.11; 12 % gets 9.89.
        assert_eq!(s[1].gross(), Some(d("9.89")));
        // ΣG ≤ 0: everything to the highest gross (12 %), not to 21 %.
        let recap = [
            (d("21"), d("-20"), d("-4.20")),
            (d("12"), d("10"), d("1.20")),
        ];
        let s = split(d("5"), &recap).expect("split");
        assert_eq!(
            s.iter().map(|x| x.vat_rate).collect::<Vec<_>>(),
            vec![d("12")]
        );
    }

    #[test]
    fn overflow_is_none() {
        let big = d("9999999999999999.99");
        assert!(split(big, &[(d("21"), big, big), (d("12"), big, d("0"))]).is_none());
    }

    #[test]
    fn stored_recap_is_exactly_the_shares() {
        // base 82.64 × 21 % would be 17.35; the DDPP keeps 17.36.
        let t = totals(&[share("21", "82.64", "17.36")], None).expect("totals");
        assert_eq!((t.recap[0].base, t.recap[0].vat), (d("82.64"), d("17.36")));
        assert_eq!(
            (t.total, t.payable, t.total_czk),
            (d("100"), d("100"), None)
        );
        let t = totals(
            &[share("12", "10", "1.20"), share("21", "10", "2.10")],
            Some(d("25.125")),
        )
        .expect("totals");
        assert_eq!(t.recap[0].vat_rate, d("21"));
        assert_eq!(t.recap[0].base_czk, Some(d("251.25")));
        assert_eq!(t.recap[0].vat_czk, Some(d("52.76")));
        assert_eq!(t.total_czk, Some(d("585.41")));
    }
}
