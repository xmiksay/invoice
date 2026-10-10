//! `GET` / `PUT /api/settings/accounting`.

use axum::Json;
use axum::extract::State;

use super::repo;
use super::settings::AccountingSettings;
use crate::app::AppState;
use crate::error::{AppError, ErrorBody};
use crate::extract::ApiJson;

#[utoipa::path(
    get,
    path = "/api/settings/accounting",
    tag = "settings",
    security(("bearer" = [])),
    responses((status = 200, body = AccountingSettings, description = "Every direction × exported type row present (missing ones all null)"))
)]
pub async fn get(State(state): State<AppState>) -> Result<Json<AccountingSettings>, AppError> {
    Ok(Json(repo::get(&state.db).await?))
}

#[utoipa::path(
    put,
    path = "/api/settings/accounting",
    tag = "settings",
    security(("bearer" = [])),
    request_body = AccountingSettings,
    responses(
        (status = 200, body = AccountingSettings),
        (status = 422, description = "`pohoda.ico`: `invalid_ico`; `pohoda.codes.N.direction` / `docType`: `invalid`, `docType`: `duplicate`; `pohoda.codes.N.{accounting|classificationVat|numberSeries}`: `too_long` (> 19)", body = ErrorBody),
    )
)]
pub async fn put(
    State(state): State<AppState>,
    ApiJson(input): ApiJson<AccountingSettings>,
) -> Result<Json<AccountingSettings>, AppError> {
    let s = input.validate()?;
    Ok(Json(repo::put(&state.db, &s).await?))
}
