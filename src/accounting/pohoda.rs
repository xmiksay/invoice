//! Stored documents → Stormware POHODA data exchange XML (`dataPack` 2.0,
//! Windows-1250). Pure: the export pipeline ([`crate::csvio::export_load`])
//! loads the [`Source`]s and streams what this writes.

use anyhow::Context as _;
use chrono::NaiveDate;
use rust_decimal::Decimal;

use super::pohoda_summary::{self as summary, Foreign, Ns};
use super::settings::{AccountingSettings, CodeRow};
use crate::csvio::export_row::{Kind, Source, czk_recap, party};
use crate::document::handlers::dto::BankSnapshot;
use crate::document::line::{PaymentMethod, VatMode};
use crate::isdoc::xml::Xml;
use crate::settings::doc_type::{DocType, ISSUED};

const NS_DAT: &str = "http://www.stormware.cz/schema/version_2/data.xsd";
const NS_INV: &str = "http://www.stormware.cz/schema/version_2/invoice.xsd";
const NS_INT: &str = "http://www.stormware.cz/schema/version_2/intDoc.xsd";
const NS_TYP: &str = "http://www.stormware.cz/schema/version_2/type.xsd";

/// The end of the file, after the last item.
pub const TAIL: &str = "</dat:dataPack>\n";

/// What every item of one export shares.
pub struct Ctx {
    /// `dataPack/@ico`: the settings override, else the company IČO.
    pub ico: Option<String>,
    pub settings: AccountingSettings,
    /// The `price3` rate: the first active Settings rate other than 0 / 12 / 21.
    pub third: Option<Decimal>,
    pub from: NaiveDate,
    pub to: NaiveDate,
}

/// The text in Windows-1250; a character outside it becomes a numeric
/// character reference (`&#8364;`), which is valid in XML text and
/// attribute values.
pub fn encode(s: &str) -> Vec<u8> {
    encoding_rs::WINDOWS_1250.encode(s).0.into_owned()
}

/// The XML declaration and the open `dat:dataPack`.
pub fn head(ctx: &Ctx) -> String {
    let id = format!("invoice-{}-{}", ctx.from, ctx.to);
    let note = format!("Export z Invoice {}–{}", ctx.from, ctx.to);
    let mut attrs = vec![
        ("xmlns:dat", NS_DAT),
        ("xmlns:inv", NS_INV),
        ("xmlns:int", NS_INT),
        ("xmlns:typ", NS_TYP),
        ("id", id.as_str()),
    ];
    if let Some(ico) = &ctx.ico {
        attrs.push(("ico", ico));
    }
    attrs.extend([
        ("application", "invoice"),
        ("version", "2.0"),
        ("note", note.as_str()),
    ]);
    let mut x = Xml::fragment();
    x.open("dat:dataPack", &attrs);
    format!(
        "<?xml version=\"1.0\" encoding=\"Windows-1250\"?>\n{}\n",
        x.unclosed()
    )
}

/// The agenda and `invoiceType` of a document; DDPPs and their corrections
/// go to Interní doklady (Pohoda keeps tax documents to received payments
/// there, its advance invoices are no tax documents).
fn agenda(doc_type: DocType, issued: bool) -> anyhow::Result<(Ns, Option<&'static str>)> {
    let t = match (doc_type, issued) {
        (DocType::Invoice | DocType::Simplified, true) => "issuedInvoice",
        (DocType::Invoice | DocType::Simplified, false) => "receivedInvoice",
        (DocType::CreditNote, true) => "issuedCreditNotice",
        (DocType::CreditNote, false) => "receivedCreditNotice",
        (DocType::DebitNote, true) => "issuedDebitNote",
        (DocType::DebitNote, false) => "receivedDebitNote",
        (DocType::AdvanceTaxDoc | DocType::AdvanceCreditNote, _) => return Ok((Ns::Int, None)),
        (other, _) => anyhow::bail!("{} is not exported to Pohoda", other.as_str()),
    };
    Ok((Ns::Inv, Some(t)))
}

/// The label in `text` (Czech, as the UI names the types).
pub fn label(doc_type: DocType, issued: bool) -> &'static str {
    match (doc_type, issued) {
        (DocType::Invoice, true) => "Faktura",
        (DocType::Invoice, false) => "Přijatá faktura",
        (DocType::CreditNote, true) => "Dobropis",
        (DocType::CreditNote, false) => "Přijatý dobropis",
        (DocType::DebitNote, true) => "Vrubopis",
        (DocType::DebitNote, false) => "Přijatý vrubopis",
        (DocType::AdvanceTaxDoc, true) => "Daňový doklad k platbě",
        (DocType::AdvanceTaxDoc, false) => "Přijatý daňový doklad k platbě",
        (DocType::AdvanceCreditNote, true) => "Opravný daňový doklad k platbě",
        (DocType::AdvanceCreditNote, false) => "Přijatý opravný doklad k platbě",
        (DocType::Simplified, true) => "Zjednodušený daňový doklad",
        (DocType::Simplified, false) => "Přijatý zjednodušený daňový doklad",
        _ => "Doklad",
    }
}

/// `s` when it fits `max` characters (an XSD `maxLength`), else an error:
/// a number is never cut.
fn fits<'a>(what: &str, s: &'a str, max: usize) -> anyhow::Result<&'a str> {
    anyhow::ensure!(s.chars().count() <= max, "{what} longer than {max}: {s}");
    Ok(s)
}

/// The first `max` characters (address lines Pohoda keeps shorter).
fn cut(s: &str, max: usize) -> String {
    s.trim().chars().take(max).collect()
}

fn ids(x: &mut Xml, name: &str, code: Option<&String>) {
    if let Some(c) = code {
        x.open(name, &[]).leaf("typ:ids", c).close();
    }
}

fn partner(x: &mut Xml, ns: Ns, s: &Source, issued: bool) -> anyhow::Result<()> {
    let snapshot = match issued {
        true => &s.doc.customer_snapshot,
        false => &s.doc.supplier_snapshot,
    };
    let Some(p) = party(snapshot)? else {
        return Ok(());
    };
    let some = |v: String| Some(v).filter(|v| !v.is_empty());
    x.open(&ns.name("partnerIdentity"), &[])
        .open("typ:address", &[])
        .opt("typ:company", some(cut(&p.name, 255)))
        .opt("typ:city", some(cut(&p.city, 45)))
        .opt("typ:street", some(cut(&p.street, 64)))
        .opt("typ:zip", some(cut(&p.zip, 15)))
        .opt("typ:ico", p.ico.as_deref().map(|v| cut(v, 15)))
        .opt("typ:dic", p.dic.as_deref().map(|v| cut(v, 18)));
    if let Some(c) = some(cut(&p.country, 19)) {
        x.open("typ:country", &[]).leaf("typ:ids", c).close();
    }
    x.close().close();
    Ok(())
}

/// `paymentType` + (issued) `account` of an invoice-agenda document.
fn payment(x: &mut Xml, s: &Source, issued: bool) -> anyhow::Result<()> {
    let method = PaymentMethod::parse(&s.doc.payment_method).context("stored payment method")?;
    let pt = match method {
        PaymentMethod::BankTransfer => Some("draft"),
        PaymentMethod::Cash => Some("cash"),
        PaymentMethod::Card => Some("creditcard"),
        PaymentMethod::Other => None,
    };
    if let Some(pt) = pt {
        x.open("inv:paymentType", &[])
            .leaf("typ:paymentType", pt)
            .close();
    }
    let Some(bank) = s.doc.bank_snapshot.as_ref().filter(|_| issued) else {
        return Ok(());
    };
    let bank: BankSnapshot =
        serde_json::from_value(bank.clone()).context("decode bank snapshot")?;
    let split = bank
        .account_number
        .as_deref()
        .and_then(|a| a.trim().split_once('/'))
        .filter(|(no, code)| no.chars().count() <= 34 && code.chars().count() <= 11);
    if let Some((no, code)) = split {
        x.open("inv:account", &[])
            .leaf("typ:accountNo", no.trim())
            .leaf("typ:bankCode", code.trim())
            .close();
    }
    Ok(())
}

/// `classificationVAT`: a received document without the VAT deduction
/// takes the non-deductible code, else `nonSubsume` ("nezahrnovat do
/// DPH") — never the deductible code, and never nothing, which Pohoda
/// reads as `inland` (deductible). Every other document: the code or
/// nothing.
fn classification(x: &mut Xml, ns: Ns, codes: Option<&CodeRow>, non_deductible: bool) {
    let name = ns.name("classificationVAT");
    let code = codes.and_then(|c| match non_deductible {
        true => c.classification_vat_non_deductible.as_ref(),
        false => c.classification_vat.as_ref(),
    });
    match (code, non_deductible) {
        (Some(c), _) => ids(x, &name, Some(c)),
        (None, true) => {
            x.open(&name, &[])
                .leaf("typ:classificationVATType", "nonSubsume")
                .close();
        }
        (None, false) => {}
    }
}

/// The number the file names a document by (issued: ours, received: the
/// supplier's), for messages.
pub fn shown_number(doc: &crate::document::entity::document::Model) -> Option<&str> {
    match doc.direction.as_str() {
        ISSUED => doc.number.as_deref(),
        _ => doc.supplier_number.as_deref().or(doc.number.as_deref()),
    }
}

/// One `dat:dataPackItem` (UTF-8 text, [`encode`]d by the caller). An
/// error makes the document unexportable (an unmappable rate, a number too
/// long for Pohoda, an undecodable snapshot…).
pub fn item(s: &Source, ctx: &Ctx) -> anyhow::Result<String> {
    let doc = s.doc;
    let Kind {
        issued,
        doc_type,
        vat_mode,
        ..
    } = Kind::of(doc)?;
    let (ns, invoice_type) = agenda(doc_type, issued)?;
    let vat_free = matches!(vat_mode, VatMode::Exempt | VatMode::NonPayer);
    let slots = summary::slots(&czk_recap(doc, s.recap)?, vat_free, ctx.third)?;
    let codes: Option<&CodeRow> = ctx.settings.pohoda_codes(&doc.direction, doc_type);
    let code = |f: fn(&CodeRow) -> &Option<String>| codes.and_then(|c| f(c).as_ref());
    let tax_date = doc.tax_point_date.or(doc.received_date.filter(|_| !issued));
    let accounting_date = match issued {
        true => tax_date,
        false => doc.received_date.or(tax_date),
    };
    let shown = shown_number(doc);
    let original = s.parent.and_then(|p| match issued {
        true => p.number.as_deref(),
        false => p.supplier_number.as_deref().or(p.number.as_deref()),
    });
    let mut text = label(doc_type, issued).to_string();
    for (sep, n) in [(" ", shown), (" k ", original)] {
        if let Some(n) = n {
            text.push_str(sep);
            text.push_str(n);
        }
    }

    let id = doc.id.to_string();
    let mut x = Xml::fragment();
    x.open("dat:dataPackItem", &[("id", &id), ("version", "2.0")]);
    match ns {
        Ns::Inv => x.open("inv:invoice", &[("version", "2.0")]),
        Ns::Int => x.open("int:intDoc", &[("version", "2.0")]),
    };
    x.open(
        &ns.name(match ns {
            Ns::Inv => "invoiceHeader",
            Ns::Int => "intDocHeader",
        }),
        &[],
    );
    if let Some(t) = invoice_type {
        x.leaf("inv:invoiceType", t);
    }
    // Our number always (Pohoda keeps it and checks duplicates), within
    // the configured series when one is set.
    let series = code(|c| &c.number_series);
    if series.is_some() || doc.number.is_some() {
        x.open(&ns.name("number"), &[]).opt("typ:ids", series);
        if let Some(n) = &doc.number {
            x.leaf("typ:numberRequested", fits("number", n, 32)?);
        }
        x.close();
    }
    if let Some(vs) = &doc.variable_symbol {
        x.leaf(&ns.name("symVar"), fits("variable symbol", vs, 20)?);
    }
    if ns == Ns::Inv
        && let Some(o) = shown
    {
        x.leaf("inv:originalDocument", fits("original document", o, 32)?);
    }
    x.leaf(&ns.name("date"), doc.issue_date.to_string())
        .opt(&ns.name("dateTax"), tax_date.map(|d| d.to_string()))
        .opt(
            &ns.name("dateAccounting"),
            accounting_date.map(|d| d.to_string()),
        );
    if ns == Ns::Inv {
        x.opt("inv:dateDue", doc.due_date.map(|d| d.to_string()));
    }
    ids(&mut x, &ns.name("accounting"), code(|c| &c.accounting));
    classification(&mut x, ns, codes, !issued && !doc.vat_deductible);
    x.leaf(&ns.name("text"), cut(&text, 240));
    partner(&mut x, ns, s, issued)?;
    if ns == Ns::Inv {
        payment(&mut x, s, issued)?;
    }
    x.close();

    let is_czk = doc.currency == "CZK";
    let sign = Decimal::from(doc_type.sign());
    x.open(
        &ns.name(match ns {
            Ns::Inv => "invoiceSummary",
            Ns::Int => "intDocSummary",
        }),
        &[],
    );
    let foreign = (!is_czk).then(|| Foreign {
        currency: &doc.currency,
        rate: doc.exchange_rate,
        sum: (doc.total + doc.rounding) * sign,
    });
    let rounding = summary::Rounding {
        document: summary::rounding_document(doc.total, doc.rounding),
        price_round: is_czk.then_some(doc.rounding),
    };
    summary::write(&mut x, ns, &slots, sign, rounding, foreign);
    let mut out = x.finish();
    out.push('\n');
    Ok(out)
}

#[cfg(test)]
#[path = "pohoda_tests.rs"]
mod tests;
