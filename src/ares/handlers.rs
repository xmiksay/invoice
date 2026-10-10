use axum::Json;
use axum::extract::State;

use super::AresSubject;
use crate::app::AppState;
use crate::error::{AppError, ErrorBody};
use crate::extract::ApiPath;
use crate::validation::opt_ico;

#[utoipa::path(
    get,
    path = "/api/ares/{ico}",
    tag = "ares",
    security(("bearer" = [])),
    params(("ico" = String, Path, description = "8-digit IČO")),
    responses(
        (status = 200, body = AresSubject),
        (status = 404, description = "`ares_not_found`", body = ErrorBody),
        (status = 422, description = "`ico`: `invalid_ico`", body = ErrorBody),
        (status = 502, description = "`ares_unavailable`", body = ErrorBody),
    )
)]
pub async fn lookup(
    State(state): State<AppState>,
    ApiPath(ico): ApiPath<String>,
) -> Result<Json<AresSubject>, AppError> {
    Ok(Json(lookup_ico(&state, &ico).await?))
}

/// ARES lookup of a (normalized) IČO; a malformed one → `ico: invalid_ico`.
pub async fn lookup_ico(state: &AppState, ico: &str) -> Result<AresSubject, AppError> {
    let ico = opt_ico(Some(ico))
        .ok()
        .flatten()
        .ok_or_else(|| AppError::field("ico", "invalid_ico"))?;
    state.ares.lookup(&ico).await
}
