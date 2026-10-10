//! The Pohoda summary of a document: its CZK recap in Pohoda's rate slots
//! (`priceNone`, `priceLow`, `priceHigh`, `price3`) and the foreign currency
//! block. Pure.

use rust_decimal::{Decimal, RoundingStrategy};

use super::doc::{HIGH, LOW, Rates, amount, rates};
use crate::isdoc::xml::Xml;

/// `(base, VAT)` per slot, in CZK, positive as stored.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Slots {
    pub none: Option<Decimal>,
    pub low: Option<(Decimal, Decimal)>,
    pub high: Option<(Decimal, Decimal)>,
    pub third: Option<(Decimal, Decimal)>,
}

/// The recap `(rate, base CZK, VAT CZK)` in slots: the standard classes
/// ([`rates`]: 21 % high, 12 % low, 0 % / VAT-free none) and `third` (the
/// first other active Settings rate) `price3`; any other rate → error (the
/// document is unexportable).
pub fn slots(
    recap: &[(Decimal, Decimal, Decimal)],
    vat_free: bool,
    third: Option<Decimal>,
) -> anyhow::Result<Slots> {
    let Rates {
        none,
        low,
        high,
        other,
    } = rates(recap, vat_free);
    let third = third.map(|t| t.normalize());
    let mut s = Slots {
        none,
        low,
        high,
        third: None,
    };
    for (rate, base, vat) in other {
        anyhow::ensure!(
            Some(rate) == third,
            "VAT rate {rate} % maps to no Pohoda rate slot"
        );
        s.third = Some((base, vat));
    }
    Ok(s)
}

/// The `price3` rate: the first active Settings rate (in Settings order,
/// `(rate, active)`) other than 0 / 12 / 21.
pub fn third_rate(settings: &[(Decimal, bool)]) -> Option<Decimal> {
    settings
        .iter()
        .filter(|(_, active)| *active)
        .map(|(r, _)| r.normalize())
        .find(|r| !r.is_zero() && *r != LOW && *r != HIGH)
}

/// How a document's rounding is told to Pohoda.
pub struct Rounding {
    /// `roundingDocument`; `None` = the element is left out.
    pub document: Option<&'static str>,
    /// `homeCurrency/round/priceRound` (CZK documents), unsigned.
    pub price_round: Option<Decimal>,
}

/// `roundingDocument` for a stored `total` (before rounding) and its
/// `rounding`: `none` when there is none, else the mathematical Pohoda mode
/// that gives exactly that rounding (`math2one` — what our "round the
/// total" does —, `math2half`, `math2tenth`), so Pohoda's total equals our
/// payable. No mode matches (an imported odd rounding) → `None`: the
/// element is left out rather than contradicting `priceRound`.
pub fn rounding_document(total: Decimal, rounding: Decimal) -> Option<&'static str> {
    if rounding.is_zero() {
        return Some("none");
    }
    let rounded = |step: Decimal| {
        (total / step).round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero) * step
            - total
    };
    [
        ("math2one", Decimal::ONE),
        ("math2half", Decimal::new(5, 1)),
        ("math2tenth", Decimal::new(1, 1)),
    ]
    .into_iter()
    .find(|(_, step)| rounded(*step) == rounding)
    .map(|(mode, _)| mode)
}

/// The foreign currency block (`typ:typeCurrencyForeign`).
pub struct Foreign<'a> {
    pub currency: &'a str,
    pub rate: Option<Decimal>,
    /// The total in the currency, signed.
    pub sum: Decimal,
}

/// The summary element's children: `roundingDocument`, `homeCurrency`
/// (the CZK slots × `sign`, `round` for CZK documents) and, for a foreign
/// document, `foreignCurrency`. `ns` is the agenda prefix (`inv` / `int`).
pub fn write(
    x: &mut Xml,
    ns: Ns,
    slots: &Slots,
    sign: Decimal,
    rounding: Rounding,
    foreign: Option<Foreign>,
) {
    let m = |v: Decimal| amount(v * sign);
    x.opt(&ns.name("roundingDocument"), rounding.document);
    x.open(&ns.name("homeCurrency"), &[]);
    if let Some(v) = slots.none {
        x.leaf("typ:priceNone", m(v));
    }
    for ((base, vat), slot) in [
        (("typ:priceLow", "typ:priceLowVAT"), slots.low),
        (("typ:priceHigh", "typ:priceHighVAT"), slots.high),
        (("typ:price3", "typ:price3VAT"), slots.third),
    ] {
        if let Some((b, v)) = slot {
            x.leaf(base, m(b)).leaf(vat, m(v));
        }
    }
    if let Some(r) = rounding.price_round {
        x.open("typ:round", &[])
            .leaf("typ:priceRound", m(r))
            .close();
    }
    x.close();
    if let Some(f) = foreign {
        x.open(&ns.name("foreignCurrency"), &[])
            .open("typ:currency", &[])
            .leaf("typ:ids", f.currency)
            .close()
            .opt("typ:rate", f.rate.map(|r| r.normalize().to_string()))
            .leaf("typ:amount", "1")
            .leaf("typ:priceSum", amount(f.sum))
            .close();
    }
}

/// The agenda namespace of a document's elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ns {
    /// `inv:invoice` (Vydané / Přijaté faktury).
    Inv,
    /// `int:intDoc` (Interní doklady).
    Int,
}

impl Ns {
    pub fn name(self, local: &str) -> String {
        match self {
            Ns::Inv => format!("inv:{local}"),
            Ns::Int => format!("int:{local}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Decimal {
        s.parse().expect("decimal")
    }

    #[test]
    fn rate_slots() {
        let recap = [
            (d("21.00"), d("1000"), d("210")),
            (d("12"), d("100"), d("12")),
            (d("0"), d("50"), d("0")),
        ];
        let s = slots(&recap, false, None).expect("slots");
        assert_eq!(s.high, Some((d("1000"), d("210"))));
        assert_eq!(s.low, Some((d("100"), d("12"))));
        assert_eq!(s.none, Some(d("50")));
        assert_eq!(s.third, None);

        let ten = [(d("10"), d("100"), d("10"))];
        assert!(slots(&ten, false, None).is_err(), "unmappable");
        assert!(slots(&ten, false, Some(d("15"))).is_err());
        let s = slots(&ten, false, Some(d("10.00"))).expect("third");
        assert_eq!(s.third, Some((d("100"), d("10"))));

        let free = slots(
            &[(d("21"), d("100"), d("0")), (d("10"), d("5"), d("0"))],
            true,
            None,
        )
        .expect("vat free");
        assert_eq!(
            free,
            Slots {
                none: Some(d("105")),
                ..Slots::default()
            }
        );
    }

    #[test]
    fn third_rates() {
        let r = |s: &str, a| (d(s), a);
        assert_eq!(
            third_rate(&[r("21", true), r("12", true), r("0", true)]),
            None
        );
        assert_eq!(
            third_rate(&[
                r("21", true),
                r("15", false),
                r("10.00", true),
                r("5", true)
            ]),
            Some(d("10"))
        );
    }

    #[test]
    fn rounding_modes() {
        assert_eq!(rounding_document(d("1210.40"), d("0")), Some("none"));
        assert_eq!(
            rounding_document(d("1210.40"), d("-0.40")),
            Some("math2one")
        );
        assert_eq!(rounding_document(d("1210.50"), d("0.50")), Some("math2one"));
        assert_eq!(
            rounding_document(d("1210.30"), d("0.20")),
            Some("math2half")
        );
        assert_eq!(
            rounding_document(d("1210.34"), d("-0.04")),
            Some("math2tenth")
        );
        assert_eq!(
            rounding_document(d("1210.40"), d("0.60")),
            None,
            "no mode rounds up by 0.60"
        );
    }

    #[test]
    fn negative_summary_with_foreign_block() {
        let s = Slots {
            high: Some((d("2500"), d("525"))),
            ..Slots::default()
        };
        let mut x = Xml::fragment();
        let f = Foreign {
            currency: "EUR",
            rate: Some(d("25.000000")),
            sum: d("-121"),
        };
        let r = Rounding {
            document: Some("none"),
            price_round: None,
        };
        write(&mut x, Ns::Int, &s, d("-1"), r, Some(f));
        assert_eq!(
            x.unclosed(),
            "<int:roundingDocument>none</int:roundingDocument><int:homeCurrency>\
             <typ:priceHigh>-2500.00</typ:priceHigh><typ:priceHighVAT>-525.00</typ:priceHighVAT>\
             </int:homeCurrency><int:foreignCurrency><typ:currency><typ:ids>EUR</typ:ids>\
             </typ:currency><typ:rate>25</typ:rate><typ:amount>1</typ:amount>\
             <typ:priceSum>-121.00</typ:priceSum></int:foreignCurrency>"
        );
    }
}
