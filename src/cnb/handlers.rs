use axum::Json;
use axum::extract::State;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use super::repo;
use crate::app::AppState;
use crate::error::{AppError, ErrorBody};
use crate::extract::{ApiPath, ApiQuery};
use crate::time::today;
use crate::validation as v;

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeRate {
    pub currency: String,
    /// The ČNB publication date of the rate used.
    pub date: NaiveDate,
    /// CZK per 1 unit, decimal string.
    pub rate: Decimal,
}

#[derive(Debug, Clone, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct RateQuery {
    /// `YYYY-MM-DD`, default today.
    pub date: Option<NaiveDate>,
}

#[utoipa::path(
    get,
    path = "/api/exchange-rates/{currency}",
    tag = "exchange-rates",
    security(("bearer" = [])),
    params(("currency" = String, Path, description = "ISO 4217 code"), RateQuery),
    responses(
        (status = 200, body = ExchangeRate),
        (status = 404, description = "unknown currency", body = ErrorBody),
        (status = 502, description = "`cnb_unavailable`", body = ErrorBody),
    )
)]
pub async fn get_rate(
    State(state): State<AppState>,
    ApiPath(currency): ApiPath<String>,
    ApiQuery(query): ApiQuery<RateQuery>,
) -> Result<Json<ExchangeRate>, AppError> {
    let currency = v::currency(&currency).map_err(|_| AppError::NotFound)?;
    let today = today();
    let r = repo::rate(
        &state.db,
        &state.cnb,
        &currency,
        query.date.unwrap_or(today),
        today,
    )
    .await?;
    Ok(Json(ExchangeRate {
        currency: r.currency,
        date: r.date,
        rate: r.rate,
    }))
}
