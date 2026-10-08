//! Shared state and the top-level Axum router.

use axum::Router;
use axum::middleware;
use axum::routing::get;
use sea_orm::DatabaseConnection;
use tower_http::trace::TraceLayer;

use crate::ares::AresClient;
use crate::cnb::CnbClient;
use crate::error::AppError;
use crate::secret::Secret;
use crate::{ares, auth, catalog, cnb, contact, document, health, openapi, settings, spa};

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub api_token: Secret<String>,
    pub ares: AresClient,
    pub cnb: CnbClient,
}

/// `/api/health` and `/api/openapi.json` are public; every other `/api/*`
/// path — including unknown ones — requires the Bearer token. Non-API paths
/// fall through to the embedded SPA.
pub fn router(state: AppState) -> Router {
    let protected = Router::new()
        .route("/auth/check", get(auth::check))
        .nest("/settings", settings::router())
        .nest("/contacts", contact::router())
        .route("/ares/{ico}", get(ares::handlers::lookup))
        .nest("/documents", document::router())
        .nest("/catalog", catalog::router())
        .route("/exchange-rates/{currency}", get(cnb::handlers::get_rate))
        .fallback(api_not_found)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_token,
        ));

    let api = Router::new()
        .route("/health", get(health::health))
        .route("/openapi.json", get(openapi::openapi_json))
        .merge(protected);

    Router::new()
        .nest("/api", api)
        .fallback(spa::serve)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn api_not_found() -> AppError {
    AppError::NotFound
}
