//! The amount-carrying parts of the ISDOC export: lines, deposits, the VAT
//! recap and the monetary totals.

use rust_decimal::Decimal;

use super::export_xml::{Deposit, W, m, n};
use crate::document::compute::{item_base, round2};
use crate::document::line::{AdvanceRow, LineData};

/// Four amounts of one rate (document currency + CZK).
#[derive(Default, Clone, Copy)]
struct Amounts {
    base: Decimal,
    vat: Decimal,
    base_czk: Decimal,
    vat_czk: Decimal,
}

impl Amounts {
    fn add(&mut self, o: Amounts) {
        self.base += o.base;
        self.vat += o.vat;
        self.base_czk += o.base_czk;
        self.vat_czk += o.vat_czk;
    }
}

#[derive(Default, Clone, Copy)]
struct Rate {
    rate: Decimal,
    net: Amounts,
    taxed: Amounts,
    nontaxed: Amounts,
}

impl W<'_> {
    pub(super) fn lines(&mut self) {
        self.x.open("InvoiceLines", &[]);
        let mut id = 0;
        for line in &self.s.lines {
            let (description, item) = match line {
                LineData::Item(i) => (i.description.as_str(), Some(i)),
                LineData::Text { description } => (description.as_str(), None),
                _ => continue,
            };
            id += 1;
            self.x.open("InvoiceLine", &[]).leaf("ID", id.to_string());
            let Some(i) = item else {
                for e in [
                    "LineExtensionAmount",
                    "LineExtensionAmountTaxInclusive",
                    "LineExtensionTaxAmount",
                    "UnitPrice",
                    "UnitPriceTaxInclusive",
                ] {
                    self.x.leaf(e, "0");
                }
                self.tax_category("ClassifiedTaxCategory", Decimal::ZERO);
                self.x
                    .open("Item", &[])
                    .leaf("Description", description)
                    .close()
                    .close();
                continue;
            };
            let base = item_base(i.quantity, i.unit_price, i.discount_pct).unwrap_or_default();
            let vat = if self.s.vat_mode.charges_vat() {
                round2(base * i.vat_rate / Decimal::ONE_HUNDRED)
            } else {
                Decimal::ZERO
            };
            let unit = i.unit.as_deref().unwrap_or_default();
            self.x
                .leaf_attrs("InvoicedQuantity", &[("unitCode", unit)], &n(i.quantity));
            let (base_czk, vat_czk) = (self.czk(base), self.czk(vat));
            if self.foreign {
                self.x.leaf("LineExtensionAmountCurr", m(base));
            }
            self.x.leaf("LineExtensionAmount", m(base_czk));
            if !self.foreign && !i.discount_pct.is_zero() {
                self.x.leaf(
                    "LineExtensionAmountBeforeDiscount",
                    m(i.quantity * i.unit_price),
                );
            }
            if self.foreign {
                self.x
                    .leaf("LineExtensionAmountTaxInclusiveCurr", m(base + vat));
            }
            let unit_czk = (i.unit_price * self.fx).round_dp(4);
            let unit_gross = if self.s.vat_mode.charges_vat() {
                (unit_czk * (Decimal::ONE_HUNDRED + i.vat_rate) / Decimal::ONE_HUNDRED).round_dp(4)
            } else {
                unit_czk
            };
            self.x
                .leaf("LineExtensionAmountTaxInclusive", m(base_czk + vat_czk))
                .leaf("LineExtensionTaxAmount", m(vat_czk))
                .leaf("UnitPrice", n(unit_czk))
                .leaf("UnitPriceTaxInclusive", n(unit_gross));
            self.tax_category("ClassifiedTaxCategory", i.vat_rate);
            self.x
                .open("Item", &[])
                .leaf("Description", description)
                .close()
                .close();
        }
        if id == 0 {
            // The schema needs at least one line.
            self.x.open("InvoiceLine", &[]).leaf("ID", "1");
            for e in [
                "LineExtensionAmount",
                "LineExtensionAmountTaxInclusive",
                "LineExtensionTaxAmount",
                "UnitPrice",
                "UnitPriceTaxInclusive",
            ] {
                self.x.leaf(e, "0");
            }
            self.tax_category("ClassifiedTaxCategory", Decimal::ZERO);
            self.x.close();
        }
        self.x.close();
    }

    fn adv(&self, r: &AdvanceRow) -> Amounts {
        Amounts {
            base: r.base,
            vat: r.vat,
            base_czk: r.base_czk.unwrap_or_else(|| self.czk(r.base)),
            vat_czk: r.vat_czk.unwrap_or_else(|| self.czk(r.vat)),
        }
    }

    pub(super) fn deposits(&mut self) {
        for taxed in [false, true] {
            let deps: Vec<&Deposit> = self
                .s
                .deposits
                .iter()
                .filter(|d| d.taxed == taxed)
                .collect();
            if deps.is_empty() {
                continue;
            }
            let (outer, inner) = if taxed {
                ("TaxedDeposits", "TaxedDeposit")
            } else {
                ("NonTaxedDeposits", "NonTaxedDeposit")
            };
            self.x.open(outer, &[]);
            for d in deps {
                let rows: Vec<(Decimal, Amounts)> =
                    d.rows.iter().map(|r| (r.vat_rate, self.adv(r))).collect();
                if !taxed {
                    let mut sum = Amounts::default();
                    rows.iter().for_each(|(_, a)| sum.add(*a));
                    self.x
                        .open(inner, &[])
                        .leaf("ID", &d.number)
                        .leaf("VariableSymbol", &d.variable_symbol);
                    self.money(
                        "DepositAmount",
                        sum.base + sum.vat,
                        sum.base_czk + sum.vat_czk,
                        false,
                    );
                    self.x.close();
                    continue;
                }
                for (rate, a) in rows {
                    self.x
                        .open(inner, &[])
                        .leaf("ID", &d.number)
                        .leaf("VariableSymbol", &d.variable_symbol);
                    self.money("TaxableDepositAmount", a.base, a.base_czk, false);
                    self.money(
                        "TaxInclusiveDepositAmount",
                        a.base + a.vat,
                        a.base_czk + a.vat_czk,
                        false,
                    );
                    self.tax_category("ClassifiedTaxCategory", rate);
                    self.x.close();
                }
            }
            self.x.close();
        }
    }

    fn rates(&self) -> Vec<Rate> {
        let mut rates: Vec<Rate> = Vec::new();
        fn at(rates: &mut Vec<Rate>, rate: Decimal) -> &mut Rate {
            let i = match rates.iter().position(|r| r.rate == rate) {
                Some(i) => i,
                None => {
                    rates.push(Rate {
                        rate,
                        ..Default::default()
                    });
                    rates.len() - 1
                }
            };
            &mut rates[i]
        }
        for r in &self.s.recap {
            at(&mut rates, r.vat_rate.normalize()).net.add(Amounts {
                base: r.base,
                vat: r.vat,
                base_czk: r.base_czk.unwrap_or_else(|| self.czk(r.base)),
                vat_czk: r.vat_czk.unwrap_or_else(|| self.czk(r.vat)),
            });
        }
        let charges = self.s.vat_mode.charges_vat();
        for d in &self.s.deposits {
            for row in &d.rows {
                let mut a = self.adv(row);
                if !charges {
                    a.vat = Decimal::ZERO;
                    a.vat_czk = Decimal::ZERO;
                }
                let r = at(&mut rates, row.vat_rate.normalize());
                if d.taxed {
                    r.taxed.add(a)
                } else {
                    r.nontaxed.add(a)
                }
            }
        }
        rates.sort_by_key(|r| std::cmp::Reverse(r.rate));
        rates
    }

    pub(super) fn totals(&mut self) {
        let rates = self.rates();
        let mut full = Amounts::default();
        let mut claimed = Amounts::default();
        let mut diff = Amounts::default();
        let mut paid = Amounts::default();
        self.x.open("TaxTotal", &[]);
        for r in &rates {
            let mut d = r.net;
            d.add(r.nontaxed);
            let mut f = d;
            f.add(r.taxed);
            full.add(f);
            claimed.add(r.taxed);
            diff.add(d);
            paid.add(r.nontaxed);
            self.x.open("TaxSubTotal", &[]);
            for (prefix, a) in [("", f), ("AlreadyClaimed", r.taxed), ("Difference", d)] {
                self.money(&format!("{prefix}TaxableAmount"), a.base, a.base_czk, false);
                self.money(&format!("{prefix}TaxAmount"), a.vat, a.vat_czk, false);
                self.money(
                    &format!("{prefix}TaxInclusiveAmount"),
                    a.base + a.vat,
                    a.base_czk + a.vat_czk,
                    false,
                );
            }
            self.tax_category("TaxCategory", r.rate);
            self.x.close();
        }
        self.money("TaxAmount", full.vat, full.vat_czk, false);
        self.x.close().open("LegalMonetaryTotal", &[]);
        for (prefix, a) in [
            ("", full),
            ("AlreadyClaimed", claimed),
            ("Difference", diff),
        ] {
            self.money(
                &format!("{prefix}TaxExclusiveAmount"),
                a.base,
                a.base_czk,
                true,
            );
            self.money(
                &format!("{prefix}TaxInclusiveAmount"),
                a.base + a.vat,
                a.base_czk + a.vat_czk,
                true,
            );
        }
        let s = self.s;
        self.money(
            "PayableRoundingAmount",
            s.rounding,
            self.czk(s.rounding),
            true,
        );
        self.money(
            "PaidDepositsAmount",
            paid.base + paid.vat,
            paid.base_czk + paid.vat_czk,
            true,
        );
        let payable_czk = s.total_czk.unwrap_or_else(|| self.czk(s.payable));
        self.money("PayableAmount", s.payable, payable_czk, true);
        self.x.close();
    }
}
