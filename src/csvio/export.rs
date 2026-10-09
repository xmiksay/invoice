//! `GET /api/export/csv` (the list filter) and `GET /api/export/accountant`
//! (a tax-date period): issued documents as CSV in the import format.

use axum::extract::State;
use axum::response::Response;
use chrono::NaiveDate;
use sea_orm::sea_query::{Expr, Order};
use sea_orm::{
    AccessMode, ColumnTrait, ConnectionTrait, EntityTrait, IsolationLevel, QueryFilter, QueryOrder,
    QuerySelect, Select, TransactionTrait,
};
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

use super::export_load;
use crate::app::AppState;
use crate::document::entity::document::{Column, Entity};
use crate::document::handlers::dto::ListQuery;
use crate::document::line::Status;
use crate::document::repo::query;
use crate::download::attachment;
use crate::error::{AppError, ErrorBody, FieldErrors};
use crate::extract::ApiQuery;
use crate::settings::doc_type::{DocType, ISSUED, RECEIVED};
use crate::time::today;

/// Most documents one export may hold.
pub const MAX_ROWS: u64 = 10_000;

/// Longest accountant period, `to − from` in days.
pub const MAX_PERIOD_DAYS: i64 = 366;

/// The tax date of a document: DUZP, for a received one without it the
/// received date (issued documents have none).
const TAX_DATE: &str = "COALESCE(tax_point_date, received_date)";

/// The ids of the matching issued (non-draft, non-cancelled) documents in
/// export order; more than [`MAX_ROWS`] → `filter: too_many`.
async fn ids<C: ConnectionTrait>(db: &C, select: Select<Entity>) -> Result<Vec<Uuid>, AppError> {
    let ids: Vec<Uuid> = select
        .filter(Column::Status.eq(Status::Issued.as_str()))
        .select_only()
        .column(Column::Id)
        .order_by_asc(Column::Direction)
        .order_by(
            Expr::cust(format!("COALESCE({TAX_DATE}, issue_date)")),
            Order::Asc,
        )
        // Allocated numbers sort by their series position (text order
        // breaks at a new digit count); imported ones by the text.
        .order_by_asc(Column::NumberYear)
        .order_by_asc(Column::NumberSeq)
        .order_by_asc(Column::Number)
        .order_by_asc(Column::Id)
        .limit(MAX_ROWS + 1)
        .into_tuple()
        .all(db)
        .await?;
    if ids.len() as u64 > MAX_ROWS {
        return Err(AppError::field("filter", "too_many"));
    }
    Ok(ids)
}

/// The whole export reads one snapshot: the transaction lives in the
/// streamed body, so the header's rate columns, the id list and every chunk
/// agree even while documents change.
async fn export(
    state: &AppState,
    select: Select<Entity>,
    filename: &str,
) -> Result<Response, AppError> {
    let txn = state
        .db
        .begin_with_config(
            Some(IsolationLevel::RepeatableRead),
            Some(AccessMode::ReadOnly),
        )
        .await?;
    let ids = match ids(&txn, select).await {
        Ok(ids) => ids,
        Err(e) => {
            if let Err(r) = txn.rollback().await {
                tracing::warn!(error = %r, "CSV export rollback failed");
            }
            return Err(e);
        }
    };
    let body = export_load::body(txn, ids).await?;
    Ok(attachment("text/csv; charset=utf-8", filename, body))
}

/// `issued` | `received`; absent → `required`, anything else → `invalid`.
fn list_direction(d: Option<&str>) -> Result<&'static str, AppError> {
    match d {
        Some(ISSUED) => Ok(ISSUED),
        Some(RECEIVED) => Ok(RECEIVED),
        None => Err(AppError::field("direction", "required")),
        Some(_) => Err(AppError::field("direction", "invalid")),
    }
}

#[utoipa::path(
    get,
    path = "/api/export/csv",
    tag = "export",
    security(("bearer" = [])),
    params(ListQuery),
    responses(
        (status = 200, description = "`doklady-{direction}-{yyyy-mm-dd}.csv`: the documents of the list filter without drafts and cancelled ones (`limit` / `offset` ignored), streamed", content_type = "text/csv"),
        (status = 422, description = "`direction`: `required` / `invalid`; `filter`: `too_many` (more than 10 000)", body = ErrorBody),
    )
)]
pub async fn list_csv(
    State(state): State<AppState>,
    ApiQuery(mut q): ApiQuery<ListQuery>,
) -> Result<Response, AppError> {
    let direction = list_direction(q.direction.as_deref().map(str::trim))?;
    q.direction = Some(direction.into());
    let today = today();
    let select = query::export_select(&q, today);
    export(&state, select, &format!("doklady-{direction}-{today}.csv")).await
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct AccountantQuery {
    /// `YYYY-MM-DD`, inclusive.
    pub from: Option<String>,
    /// `YYYY-MM-DD`, inclusive; `from ≤ to ≤ from + 366 days`.
    pub to: Option<String>,
    /// `issued` | `received` | `both` (default).
    pub direction: Option<String>,
}

/// A checked accountant request: the period and the direction (`None` =
/// both).
#[derive(Debug, PartialEq, Eq)]
pub struct Accountant {
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub direction: Option<&'static str>,
}

impl AccountantQuery {
    pub fn check(&self) -> Result<Accountant, AppError> {
        let mut e = FieldErrors::new();
        let date = |s: &Option<String>| {
            s.as_deref()
                .and_then(|s| NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").ok())
                .ok_or("invalid")
        };
        let from = e.check("from", date(&self.from));
        let to = e.check("to", date(&self.to));
        let direction = e.check(
            "direction",
            match self.direction.as_deref().map(str::trim) {
                None | Some("both") => Ok(None),
                Some(ISSUED) => Ok(Some(ISSUED)),
                Some(RECEIVED) => Ok(Some(RECEIVED)),
                Some(_) => Err("invalid"),
            },
        );
        if let (Some(f), Some(t)) = (from, to)
            && (t < f || (t - f).num_days() > MAX_PERIOD_DAYS)
        {
            e.add("from", "invalid");
            e.add("to", "invalid");
        }
        e.into_result()?;
        match (from, to, direction) {
            (Some(from), Some(to), Some(direction)) => Ok(Accountant {
                from,
                to,
                direction,
            }),
            _ => Err(anyhow::anyhow!("checked accountant query incomplete").into()),
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/export/accountant",
    tag = "export",
    security(("bearer" = [])),
    params(AccountantQuery),
    responses(
        (status = 200, description = "`ucetni-{from}-{to}.csv`: every non-proforma, non-draft, non-cancelled document whose tax date (received: else the received date) is in the period, streamed", content_type = "text/csv"),
        (status = 422, description = "`from` / `to` / `direction`: `invalid`; `filter`: `too_many` (more than 10 000)", body = ErrorBody),
    )
)]
pub async fn accountant(
    State(state): State<AppState>,
    ApiQuery(q): ApiQuery<AccountantQuery>,
) -> Result<Response, AppError> {
    let a = q.check()?;
    let mut select = Entity::find()
        .filter(Column::DocType.ne(DocType::Proforma.as_str()))
        .filter(Expr::cust_with_values(
            format!("{TAX_DATE} BETWEEN $1 AND $2"),
            [a.from, a.to],
        ));
    if let Some(d) = a.direction {
        select = select.filter(Column::Direction.eq(d));
    }
    export(&state, select, &format!("ucetni-{}-{}.csv", a.from, a.to)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(from: Option<&str>, to: Option<&str>, direction: Option<&str>) -> AccountantQuery {
        AccountantQuery {
            from: from.map(str::to_string),
            to: to.map(str::to_string),
            direction: direction.map(str::to_string),
        }
    }

    fn fields(r: Result<Accountant, AppError>) -> Vec<(String, &'static str)> {
        match r {
            Err(AppError::Validation(e)) => {
                let mut v: Vec<_> = ["from", "to", "direction"]
                    .into_iter()
                    .filter_map(|f| e.get(f).map(|r| (f.to_string(), r)))
                    .collect();
                v.sort();
                v
            }
            other => panic!("expected validation, got {other:?}"),
        }
    }

    #[test]
    fn accountant_period() {
        let date = |m, d| NaiveDate::from_ymd_opt(2026, m, d).expect("date");
        assert_eq!(
            q(Some("2026-01-01"), Some("2026-01-31"), None)
                .check()
                .expect("ok"),
            Accountant {
                from: date(1, 1),
                to: date(1, 31),
                direction: None
            }
        );
        let one_day = q(Some("2026-03-05"), Some("2026-03-05"), Some("received"));
        assert_eq!(one_day.check().expect("ok").direction, Some(RECEIVED));
        assert!(
            q(Some("2026-01-01"), Some("2027-01-02"), Some("both"))
                .check()
                .is_ok()
        );
        let inv = |f: &str| (f.to_string(), "invalid");
        assert_eq!(
            fields(q(Some("2026-01-01"), Some("2027-01-03"), None).check()),
            [inv("from"), inv("to")],
            "367 days"
        );
        assert_eq!(
            fields(q(Some("2026-02-01"), Some("2026-01-31"), None).check()),
            [inv("from"), inv("to")]
        );
        assert_eq!(
            fields(q(None, Some("31.01.2026"), Some("x")).check()),
            [inv("direction"), inv("from"), inv("to")]
        );
    }

    #[test]
    fn list_directions() {
        assert_eq!(list_direction(Some("issued")).ok(), Some(ISSUED));
        assert_eq!(list_direction(Some("received")).ok(), Some(RECEIVED));
        assert!(list_direction(None).is_err());
        assert!(list_direction(Some("both")).is_err());
    }
}
