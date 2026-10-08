//! Stored rows → response DTOs (derived state computed here, at read time).

use std::collections::HashMap;

use anyhow::Context as _;
use chrono::NaiveDate;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::de::DeserializeOwned;
use uuid::Uuid;

use super::query::Full;
use crate::contact::entity::contact;
use crate::document::compute::{self, Params, RecapRow, round2};
use crate::document::entity::document;
use crate::document::handlers::dto::{self, Document, DocumentSummary, RelatedDocument, lines_out};
use crate::document::line::{Status, VatMode};
use crate::document::state;
use crate::error::AppError;

pub fn status(doc: &document::Model) -> Result<Status, AppError> {
    Ok(Status::parse(&doc.status)
        .with_context(|| format!("document {} has unknown status {:?}", doc.id, doc.status))?)
}

fn snapshot<T: DeserializeOwned>(v: &Option<serde_json::Value>) -> Result<Option<T>, AppError> {
    Ok(v.clone()
        .map(serde_json::from_value)
        .transpose()
        .context("decode document snapshot")?)
}

fn derived(doc: &document::Model, today: NaiveDate) -> Result<(Status, bool), AppError> {
    let s = status(doc)?;
    Ok((
        s,
        state::is_overdue(&doc.doc_type, s, doc.paid, doc.payable, doc.due_date, today),
    ))
}

/// The stored totals (never recomputed on read).
fn stored_totals(full: &Full) -> compute::Totals {
    let d = &full.doc;
    compute::Totals {
        recap: full
            .recap
            .iter()
            .map(|r| RecapRow {
                vat_rate: r.vat_rate,
                base: round2(r.base),
                vat: round2(r.vat),
                base_czk: r.base_czk.map(round2),
                vat_czk: r.vat_czk.map(round2),
            })
            .collect(),
        base: round2(d.total_base),
        vat: round2(d.total_vat),
        total: round2(d.total),
        rounding: round2(d.rounding),
        payable: round2(d.payable),
        total_czk: d.total_czk.map(round2),
    }
}

fn related(r: &document::Model) -> Result<RelatedDocument, AppError> {
    Ok(RelatedDocument {
        id: r.id,
        doc_type: r.doc_type.clone(),
        number: r.number.clone(),
        status: status(r)?,
        payable: round2(r.payable),
    })
}

pub fn document(full: Full, today: NaiveDate) -> Result<Document, AppError> {
    let (status, overdue) = derived(&full.doc, today)?;
    let d = &full.doc;
    let vat_mode = VatMode::parse(&d.vat_mode)
        .with_context(|| format!("document {} has unknown vat mode", d.id))?;
    let params = Params {
        vat_mode,
        is_czk: d.currency == "CZK",
        exchange_rate: d.exchange_rate,
        round_total: d.round_total,
    };
    // Stored lines passed validation on save, so this only derives line bases.
    let evaluated = compute::evaluate(&full.lines, params)
        .map_err(|e| anyhow::anyhow!("stored document {} fails validation: {e:?}", d.id))?;
    let totals = stored_totals(&full).into();
    let related_documents = full
        .related
        .iter()
        .map(related)
        .collect::<Result<Vec<_>, AppError>>()?;
    let parent = full.parent.as_ref().map(related).transpose()?;
    let d = full.doc;
    let settled = (d.doc_type == "proforma").then(|| {
        related_documents
            .iter()
            .any(|r| r.doc_type == "invoice" && r.status != Status::Cancelled)
    });
    Ok(Document {
        payment_state: state::document_payment_state(&d.doc_type, status, d.paid, d.payable),
        sign: dto::sign(&d.doc_type),
        settled,
        related_documents,
        parent,
        related_document_id: d.related_document_id,
        payment_id: d.payment_id,
        correction_reason: d.correction_reason,
        overdue,
        status,
        supplier: snapshot(&d.supplier_snapshot)?,
        customer: snapshot(&d.customer_snapshot)?,
        bank_snapshot: snapshot::<dto::BankSnapshot>(&d.bank_snapshot)?,
        lines: lines_out(&full.lines, &evaluated.lines),
        totals,
        id: d.id,
        doc_type: d.doc_type,
        direction: d.direction,
        number: d.number,
        contact_id: d.contact_id,
        issue_date: d.issue_date,
        tax_point_date: d.tax_point_date,
        due_date: d.due_date,
        currency: d.currency,
        exchange_rate: d.exchange_rate.map(|r| r.normalize()),
        exchange_rate_date: d.exchange_rate_date,
        exchange_rate_source: d.exchange_rate_source,
        locale: d.locale,
        vat_mode: d.vat_mode,
        bank_account_id: d.bank_account_id,
        payment_method: d.payment_method,
        variable_symbol: d.variable_symbol,
        constant_symbol: d.constant_symbol,
        order_ref: d.order_ref,
        header_note: d.header_note,
        footer_note: d.footer_note,
        internal_note: d.internal_note,
        round_total: d.round_total,
        sent_at: d.sent_at,
        cancelled_at: d.cancelled_at,
        cancel_reason: d.cancel_reason,
        paid: round2(d.paid),
        created_at: d.created_at,
        updated_at: d.updated_at,
    })
}

/// Summaries; drafts (no snapshot yet) show the live contact name.
pub async fn summaries(
    db: &DatabaseConnection,
    docs: Vec<document::Model>,
    today: NaiveDate,
) -> Result<Vec<DocumentSummary>, AppError> {
    let ids: Vec<Uuid> = docs
        .iter()
        .filter(|d| d.customer_snapshot.is_none())
        .filter_map(|d| d.contact_id)
        .collect();
    let names: HashMap<Uuid, String> = if ids.is_empty() {
        HashMap::new()
    } else {
        contact::Entity::find()
            .filter(contact::Column::Id.is_in(ids))
            .all(db)
            .await?
            .into_iter()
            .map(|c| (c.id, c.name))
            .collect()
    };
    docs.into_iter()
        .map(|d| {
            let (status, overdue) = derived(&d, today)?;
            let snap: Option<dto::PartySnapshot> = snapshot(&d.customer_snapshot)?;
            let customer_name = match snap {
                Some(s) => Some(s.name),
                None => d.contact_id.and_then(|id| names.get(&id).cloned()),
            };
            Ok(DocumentSummary {
                payment_state: state::document_payment_state(
                    &d.doc_type,
                    status,
                    d.paid,
                    d.payable,
                ),
                sign: dto::sign(&d.doc_type),
                related_document_id: d.related_document_id,
                overdue,
                status,
                customer_name,
                id: d.id,
                doc_type: d.doc_type,
                direction: d.direction,
                number: d.number,
                contact_id: d.contact_id,
                issue_date: d.issue_date,
                due_date: d.due_date,
                currency: d.currency,
                payable: round2(d.payable),
                paid: round2(d.paid),
                sent_at: d.sent_at,
            })
        })
        .collect()
}
