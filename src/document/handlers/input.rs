//! Document request bodies and their validation into [`DocumentData`].

use chrono::{Days, NaiveDate};
use rust_decimal::Decimal;
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use super::line_input::{LineInput, Texts, decimal, validate_lines};
use crate::contact::entity::contact;
use crate::document::compute::{self, Evaluated, Params};
use crate::document::line::{LineData, MAX_RATE, PaymentMethod, VatMode};
use crate::error::{AppError, FieldErrors};
use crate::settings::entity::company;
use crate::validation::{self as v, Check};

/// Create (`POST`, omitted fields get defaults) and update (`PUT`, replaces
/// every field) body.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct DocumentInput {
    /// Only `invoice` in this phase.
    pub doc_type: Option<String>,
    /// Only `issued` in this phase.
    pub direction: Option<String>,
    pub contact_id: Option<Uuid>,
    pub issue_date: Option<NaiveDate>,
    pub tax_point_date: Option<NaiveDate>,
    pub due_date: Option<NaiveDate>,
    pub currency: Option<String>,
    /// CZK per 1 unit; manual override (null = ČNB at issue).
    pub exchange_rate: Option<String>,
    pub locale: Option<String>,
    /// `standard` | `reverse_charge` | `exempt` | `non_payer`.
    pub vat_mode: Option<String>,
    pub bank_account_id: Option<Uuid>,
    /// `bank_transfer` | `cash` | `card` | `other`.
    pub payment_method: Option<String>,
    pub variable_symbol: Option<String>,
    pub constant_symbol: Option<String>,
    pub order_ref: Option<String>,
    pub header_note: Option<String>,
    pub footer_note: Option<String>,
    pub internal_note: Option<String>,
    pub round_total: Option<bool>,
    pub lines: Vec<LineInput>,
}

/// Validated [`DocumentInput`].
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentData {
    pub contact_id: Option<Uuid>,
    pub issue_date: NaiveDate,
    pub tax_point_date: Option<NaiveDate>,
    pub due_date: NaiveDate,
    pub currency: String,
    pub exchange_rate: Option<Decimal>,
    pub locale: String,
    pub vat_mode: VatMode,
    pub bank_account_id: Option<Uuid>,
    pub payment_method: PaymentMethod,
    pub variable_symbol: Option<String>,
    pub constant_symbol: Option<String>,
    pub order_ref: Option<String>,
    pub header_note: Option<String>,
    pub footer_note: Option<String>,
    pub internal_note: Option<String>,
    pub round_total: bool,
    pub lines: Vec<LineData>,
}

impl DocumentData {
    pub fn params(&self) -> Params {
        Params {
            vat_mode: self.vat_mode,
            is_czk: self.currency == "CZK",
            exchange_rate: self.exchange_rate,
            round_total: self.round_total,
        }
    }
}

/// What validation needs from the database.
pub struct Context {
    pub today: NaiveDate,
    /// `POST`: fill omitted fields from contact/company; `PUT`: they are required.
    pub apply_defaults: bool,
    pub company: company::Model,
    /// The contact named by `contactId`, if it exists.
    pub contact: Option<contact::Model>,
    pub default_vat_rate: Option<Decimal>,
}

fn digits(s: Option<&str>, max: usize) -> Check<Option<String>> {
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

/// `value`, else the default when allowed, else `required`.
fn or_default<T>(value: Option<T>, ctx: &Context, default: impl FnOnce() -> T) -> Check<T> {
    match value {
        Some(v) => Ok(v),
        None if ctx.apply_defaults => Ok(default()),
        None => Err("required"),
    }
}

pub fn parse_vat_mode(s: &str) -> Check<VatMode> {
    VatMode::parse(s.trim()).ok_or("invalid")
}

impl DocumentInput {
    /// Normalize and validate every field (all failures at once) and run the
    /// computation rules. The bank account is checked by the repo.
    pub fn validate(self, ctx: &Context) -> Result<(DocumentData, Evaluated), AppError> {
        let mut e = FieldErrors::new();
        let issued_invoice = self.doc_type.as_deref().is_none_or(|d| d == "invoice")
            && self.direction.as_deref().is_none_or(|d| d == "issued");
        if !issued_invoice {
            e.add("docType", "invalid");
        }
        if self.contact_id.is_some() && ctx.contact.is_none() {
            e.add("contactId", "invalid");
        }
        let c = ctx.contact.as_ref();
        let issue_date = e
            .check("issueDate", or_default(self.issue_date, ctx, || ctx.today))
            .unwrap_or(ctx.today);
        let tax_point_date = self
            .tax_point_date
            .or(ctx.apply_defaults.then_some(issue_date));
        let due_days = c
            .and_then(|c| c.default_due_days)
            .unwrap_or(ctx.company.default_due_days);
        let due_date = e
            .check(
                "dueDate",
                or_default(self.due_date, ctx, || {
                    issue_date
                        .checked_add_days(Days::new(u64::try_from(due_days).unwrap_or(0)))
                        .unwrap_or(issue_date)
                }),
            )
            .unwrap_or(issue_date);
        let currency = match self.currency.as_deref().map(v::currency) {
            Some(r) => e.check("currency", r),
            None => e.check(
                "currency",
                or_default(None, ctx, || {
                    c.and_then(|c| c.default_currency.clone())
                        .unwrap_or_else(|| "CZK".into())
                }),
            ),
        }
        .unwrap_or_else(|| "CZK".into());
        let rate = e
            .check("exchangeRate", exchange_rate(self.exchange_rate.as_deref()))
            .flatten()
            .filter(|_| currency != "CZK");
        let locale = match self.locale.as_deref().map(v::locale) {
            Some(r) => e.check("locale", r),
            None => e.check(
                "locale",
                or_default(None, ctx, || {
                    c.and_then(|c| c.default_locale.clone())
                        .unwrap_or_else(|| ctx.company.default_locale.clone())
                }),
            ),
        }
        .unwrap_or_default();
        let vat_mode = match self.vat_mode.as_deref().map(parse_vat_mode) {
            Some(r) => e.check("vatMode", r),
            None => e.check(
                "vatMode",
                or_default(None, ctx, || {
                    if ctx.company.vat_payer {
                        VatMode::Standard
                    } else {
                        VatMode::NonPayer
                    }
                }),
            ),
        };
        let payment_method = match self.payment_method.as_deref() {
            Some(s) => e.check(
                "paymentMethod",
                PaymentMethod::parse(s.trim()).ok_or("invalid"),
            ),
            None => e.check(
                "paymentMethod",
                or_default(None, ctx, || PaymentMethod::BankTransfer),
            ),
        };
        let lines = validate_lines(
            self.lines,
            vat_mode,
            ctx.default_vat_rate,
            Texts::Validate,
            &mut e,
        );
        let data = DocumentData {
            contact_id: self.contact_id,
            issue_date,
            tax_point_date,
            due_date,
            currency,
            exchange_rate: rate,
            locale,
            vat_mode: vat_mode.unwrap_or(VatMode::Standard),
            bank_account_id: self.bank_account_id,
            payment_method: payment_method.unwrap_or(PaymentMethod::BankTransfer),
            variable_symbol: e
                .check(
                    "variableSymbol",
                    digits(self.variable_symbol.as_deref(), 10),
                )
                .flatten(),
            constant_symbol: e
                .check("constantSymbol", digits(self.constant_symbol.as_deref(), 4))
                .flatten(),
            order_ref: e
                .check("orderRef", v::opt_text(self.order_ref.as_deref(), 100))
                .flatten(),
            header_note: e
                .check("headerNote", v::opt_text(self.header_note.as_deref(), 2000))
                .flatten(),
            footer_note: e
                .check("footerNote", v::opt_text(self.footer_note.as_deref(), 2000))
                .flatten(),
            internal_note: e
                .check(
                    "internalNote",
                    v::opt_text(self.internal_note.as_deref(), 2000),
                )
                .flatten(),
            round_total: self.round_total.unwrap_or(false),
            lines,
        };
        // Line-level rules only make sense on well-formed lines.
        if !e.is_empty() {
            return Err(AppError::Validation(e));
        }
        let evaluated =
            compute::evaluate(&data.lines, data.params()).map_err(AppError::Validation)?;
        Ok((data, evaluated))
    }
}

/// `POST /api/documents/compute` body.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct ComputeInput {
    pub lines: Vec<LineInput>,
    /// Default: from the company (`standard` / `non_payer`).
    pub vat_mode: Option<String>,
    /// Default `CZK`.
    pub currency: Option<String>,
    pub exchange_rate: Option<String>,
    pub round_total: bool,
}

impl ComputeInput {
    pub fn validate(
        self,
        vat_payer: bool,
        default_rate: Option<Decimal>,
    ) -> Result<(Vec<LineData>, Evaluated), AppError> {
        let mut e = FieldErrors::new();
        let vat_mode = match self.vat_mode.as_deref() {
            Some(s) => e.check("vatMode", parse_vat_mode(s)),
            None if vat_payer => Some(VatMode::Standard),
            None => Some(VatMode::NonPayer),
        };
        let currency = match self.currency.as_deref() {
            Some(s) => e.check("currency", v::currency(s)),
            None => Some("CZK".into()),
        }
        .unwrap_or_default();
        let rate = e
            .check("exchangeRate", exchange_rate(self.exchange_rate.as_deref()))
            .flatten();
        let lines = validate_lines(self.lines, vat_mode, default_rate, Texts::Skip, &mut e);
        e.into_result()?;
        let params = Params {
            vat_mode: vat_mode.unwrap_or(VatMode::Standard),
            is_czk: currency == "CZK",
            exchange_rate: rate,
            round_total: self.round_total,
        };
        let evaluated = compute::evaluate(&lines, params).map_err(AppError::Validation)?;
        Ok((lines, evaluated))
    }
}

#[cfg(test)]
#[path = "input_tests.rs"]
mod tests;
