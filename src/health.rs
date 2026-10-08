//! Public liveness + DB readiness probe.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::Serialize;
use utoipa::ToSchema;

use crate::app::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    /// `ok` or `degraded` (database unreachable).
    pub status: &'static str,
    /// Crate version; omitted when degraded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<&'static str>,
}

#[utoipa::path(
    get,
    path = "/api/health",
    tag = "health",
    responses(
        (status = 200, description = "Service and database are up", body = HealthResponse),
        (status = 503, description = "Database unreachable", body = HealthResponse),
    )
)]
pub async fn health(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    match state.db.ping().await {
        Ok(()) => (
            StatusCode::OK,
            Json(HealthResponse {
                status: "ok",
                version: Some(env!("CARGO_PKG_VERSION")),
            }),
        ),
        Err(e) => {
            tracing::warn!(error = %e, "health check: database ping failed");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(HealthResponse {
                    status: "degraded",
                    version: None,
                }),
            )
        }
    }
}
