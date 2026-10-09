//! Received documents behind the shared `/api/documents` routes: validate,
//! resolve the ČNB rate (before any transaction), compute totals, store.

use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use uuid::Uuid;

use super::received_input::{ReceivedCtx, ReceivedData, ReceivedInput};
use crate::app::AppState;
use crate::contact::entity::contact;
use crate::document::compute::{Overflow, RecapRow};
use crate::document::entity::{document, vat_recap};
use crate::document::received;
use crate::document::repo::issue::{self as issue_repo, Rate, customer};
use crate::document::repo::meta;
use crate::document::repo::received::{self as repo, Record};
use crate::error::AppError;
use crate::extract::from_value;
use crate::settings::doc_type::RECEIVED;
use crate::settings::repo::company;
use crate::time::today;

async fn context(
    db: &DatabaseConnection,
    input: &ReceivedInput,
    stored: Option<&document::Model>,
) -> Result<ReceivedCtx, AppError> {
    let contact = match input.contact_id {
        Some(id) => contact::Entity::find_by_id(id).one(db).await?,
        None => None,
    };
    let related = match input.related_document_id {
        Some(id) => document::Entity::find_by_id(id).one(db).await?,
        None => None,
    };
    let existing = match stored {
        Some(d) => Some((d.id, crate::document::repo::context::existing(d)?.doc_type)),
        None => None,
    };
    Ok(ReceivedCtx {
        existing,
        contact,
        related,
        meta: meta::load(db, RECEIVED, input.category_id, stored).await?,
    })
}

/// A manual rate wins. A ČNB rate already fixed for the same currency and
/// `receivedDate` is kept (also when the client echoes it back); otherwise
/// ČNB is asked for `receivedDate`.
async fn rate(
    state: &AppState,
    data: &ReceivedData,
    stored: Option<&document::Model>,
) -> Result<Rate, AppError> {
    let fixed = stored.filter(|d| {
        d.currency == data.currency
            && d.received_date == Some(data.received_date)
            && d.exchange_rate_source.as_deref() == Some("cnb")
            && d.exchange_rate.is_some()
            && data
                .exchange_rate
                .is_none_or(|r| Some(r) == d.exchange_rate)
    });
    if let Some(d) = fixed {
        return Ok(Rate {
            rate: d.exchange_rate,
            date: d.exchange_rate_date,
            source: Some("cnb"),
        });
    }
    issue_repo::fetch_rate(
        &state.db,
        &state.cnb,
        &data.currency,
        data.exchange_rate,
        data.received_date,
        today(),
    )
    .await
}

async fn record(
    state: &AppState,
    input: ReceivedInput,
    stored: Option<&document::Model>,
) -> Result<Record, AppError> {
    let ctx = context(&state.db, &input, stored).await?;
    let data = input.validate(&ctx)?;
    let supplier = ctx
        .contact
        .as_ref()
        .map(customer)
        .ok_or(AppError::field("contactId", "invalid"))?;
    let rate = rate(state, &data, stored).await?;
    let totals =
        received::totals(&data.recap, data.rounding, data.payable, rate.rate).map_err(|o| {
            let field = match o {
                Overflow::Lines => "vatRecap",
                Overflow::ExchangeRate => "exchangeRate",
            };
            AppError::field(field, "invalid")
        })?;
    Ok(Record {
        data,
        totals,
        rate,
        supplier: serde_json::to_value(supplier).map_err(anyhow::Error::from)?,
    })
}

pub async fn create(state: &AppState, body: serde_json::Value) -> Result<Uuid, AppError> {
    let r = record(state, from_value(body)?, None).await?;
    let locale = company::get(&state.db).await?.default_locale;
    repo::create(&state.db, r, locale).await
}

/// `PUT` of a stored received document; the body may not switch direction.
pub async fn update(
    state: &AppState,
    doc: &document::Model,
    body: serde_json::Value,
) -> Result<(), AppError> {
    let input: ReceivedInput = from_value(body)?;
    if input
        .direction
        .as_deref()
        .is_some_and(|d| d.trim() != RECEIVED)
    {
        return Err(AppError::field("direction", "invalid"));
    }
    let mut r = record(state, input, Some(doc)).await?;
    let stored: Vec<RecapRow> = vat_recap::Entity::find()
        .filter(vat_recap::Column::DocumentId.eq(doc.id))
        .order_by_desc(vat_recap::Column::VatRate)
        .all(&state.db)
        .await?
        .into_iter()
        .map(|v| RecapRow {
            vat_rate: v.vat_rate,
            base: v.base,
            vat: v.vat,
            base_czk: v.base_czk,
            vat_czk: v.vat_czk,
        })
        .collect();
    received::keep_stored_czk(
        &mut r.totals,
        r.rate.rate,
        received::Stored {
            recap: &stored,
            rounding: doc.rounding,
            payable: doc.payable,
            total_czk: doc.total_czk,
            rate: doc.exchange_rate,
        },
    );
    repo::update(&state.db, doc.id, r).await
}
