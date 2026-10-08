//! Response shapes and small request bodies of the document routes.

use chrono::{DateTime, FixedOffset, NaiveDate};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::document::compute::{self, RecapRow};
use crate::document::line::{LineData, PaymentState, Status};
use crate::document::subtotals::Resolved;

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ItemLine {
    pub position: i32,
    pub description: String,
    pub quantity: Decimal,
    pub unit: Option<String>,
    pub unit_price: Decimal,
    pub discount_pct: Decimal,
    pub vat_rate: Decimal,
    pub base: Decimal,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TextLine {
    pub position: i32,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubtotalLine {
    pub position: i32,
    pub description: String,
    pub refs: Vec<i32>,
    pub collapse: bool,
    /// Sum of the referenced bases.
    pub base: Decimal,
    /// The shared rate of the members.
    pub vat_rate: Decimal,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Line {
    Item(ItemLine),
    Text(TextLine),
    Subtotal(SubtotalLine),
}

/// Response lines with positions and computed bases.
pub fn lines_out(lines: &[LineData], resolved: &Resolved) -> Vec<Line> {
    lines
        .iter()
        .zip(resolved)
        .zip(1..)
        .map(|((line, r), position)| {
            let (vat_rate, base) = r.unwrap_or_default();
            match line {
                LineData::Item(i) => Line::Item(ItemLine {
                    position,
                    description: i.description.clone(),
                    quantity: i.quantity.normalize(),
                    unit: i.unit.clone(),
                    unit_price: i.unit_price.normalize(),
                    discount_pct: i.discount_pct.normalize(),
                    vat_rate: i.vat_rate.normalize(),
                    base: compute::round2(base),
                }),
                LineData::Text { description } => Line::Text(TextLine {
                    position,
                    description: description.clone(),
                }),
                LineData::Subtotal {
                    description,
                    refs,
                    collapse,
                } => Line::Subtotal(SubtotalLine {
                    position,
                    description: description.clone(),
                    refs: refs.clone(),
                    collapse: *collapse,
                    base: compute::round2(base),
                    vat_rate: vat_rate.normalize(),
                }),
            }
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Recap {
    pub vat_rate: Decimal,
    pub base: Decimal,
    pub vat: Decimal,
    pub base_czk: Option<Decimal>,
    pub vat_czk: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    /// Sorted by rate, highest first.
    pub recap: Vec<Recap>,
    pub base: Decimal,
    pub vat: Decimal,
    pub total: Decimal,
    pub rounding: Decimal,
    pub payable: Decimal,
    pub total_czk: Option<Decimal>,
}

impl From<compute::Totals> for Totals {
    fn from(t: compute::Totals) -> Self {
        Self {
            recap: t
                .recap
                .into_iter()
                .map(|r: RecapRow| Recap {
                    vat_rate: r.vat_rate.normalize(),
                    base: r.base,
                    vat: r.vat,
                    base_czk: r.base_czk,
                    vat_czk: r.vat_czk,
                })
                .collect(),
            base: t.base,
            vat: t.vat,
            total: t.total,
            rounding: t.rounding,
            payable: t.payable,
            total_czk: t.total_czk,
        }
    }
}

/// Party data frozen at issue.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PartySnapshot {
    pub name: String,
    pub ico: Option<String>,
    pub dic: Option<String>,
    pub street: String,
    pub city: String,
    pub zip: String,
    pub country: String,
    pub registration: Option<String>,
    pub vat_payer: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BankSnapshot {
    pub account_number: Option<String>,
    pub iban: Option<String>,
    pub bic: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub id: Uuid,
    pub doc_type: String,
    pub direction: String,
    pub number: Option<String>,
    pub status: Status,
    pub payment_state: Option<PaymentState>,
    pub overdue: bool,
    pub contact_id: Option<Uuid>,
    pub issue_date: NaiveDate,
    pub tax_point_date: Option<NaiveDate>,
    pub due_date: NaiveDate,
    pub currency: String,
    pub exchange_rate: Option<Decimal>,
    pub exchange_rate_date: Option<NaiveDate>,
    /// `cnb` | `manual`.
    pub exchange_rate_source: Option<String>,
    pub locale: String,
    pub vat_mode: String,
    pub bank_account_id: Option<Uuid>,
    pub payment_method: String,
    pub variable_symbol: Option<String>,
    pub constant_symbol: Option<String>,
    pub order_ref: Option<String>,
    pub header_note: Option<String>,
    pub footer_note: Option<String>,
    pub internal_note: Option<String>,
    pub round_total: bool,
    pub sent_at: Option<DateTime<FixedOffset>>,
    pub cancelled_at: Option<DateTime<FixedOffset>>,
    pub cancel_reason: Option<String>,
    pub supplier: Option<PartySnapshot>,
    pub customer: Option<PartySnapshot>,
    pub bank_snapshot: Option<BankSnapshot>,
    pub lines: Vec<Line>,
    pub totals: Totals,
    /// Sum of payments.
    pub paid: Decimal,
    pub created_at: DateTime<FixedOffset>,
    pub updated_at: DateTime<FixedOffset>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSummary {
    pub id: Uuid,
    pub doc_type: String,
    pub direction: String,
    pub number: Option<String>,
    pub status: Status,
    pub payment_state: Option<PaymentState>,
    pub overdue: bool,
    pub contact_id: Option<Uuid>,
    /// Snapshot name once issued, the live contact name for drafts.
    pub customer_name: Option<String>,
    pub issue_date: NaiveDate,
    pub due_date: NaiveDate,
    pub currency: String,
    pub payable: Decimal,
    pub paid: Decimal,
    pub sent_at: Option<DateTime<FixedOffset>>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DocumentList {
    pub items: Vec<DocumentSummary>,
    pub total: u64,
}

#[derive(Debug, Clone, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
#[serde(rename_all = "camelCase")]
pub struct ListQuery {
    pub direction: Option<String>,
    pub doc_type: Option<String>,
    pub status: Option<Status>,
    pub payment_state: Option<PaymentState>,
    /// `true`: only overdue; `false`: only not overdue.
    pub overdue: Option<bool>,
    pub contact_id: Option<Uuid>,
    /// Case-insensitive substring of number, customer name or variable symbol.
    pub q: Option<String>,
    /// Inclusive `issueDate` range.
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    /// Default 50, clamped to 1..=200.
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Computed {
    pub lines: Vec<Line>,
    pub totals: Totals,
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct CancelInput {
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct MarkSentInput {
    /// Default now.
    pub sent_at: Option<DateTime<FixedOffset>>,
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct InternalNoteInput {
    pub internal_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Payment {
    pub id: Uuid,
    pub date: NaiveDate,
    pub amount: Decimal,
    pub note: Option<String>,
    pub created_at: DateTime<FixedOffset>,
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct PaymentInput {
    pub date: Option<NaiveDate>,
    /// Decimal string > 0, at most 2 dp, document currency.
    pub amount: String,
    pub note: Option<String>,
}
