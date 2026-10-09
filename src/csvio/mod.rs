//! CSV / XLSX interchange ([api/csv.md](../../docs/api/csv.md)): the shared
//! format (also for the 2c export) and the import (preview → confirm) on the
//! pipeline of [`crate::import`].

pub mod amounts;
pub mod analyze;
pub mod cell;
pub mod columns;
pub mod format;
pub mod handlers;
pub mod read;
pub mod row;
pub mod write;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};

use crate::app::AppState;
use crate::import::form::BODY_LIMIT;

/// Routes relative to `/api/import/csv`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/preview", post(handlers::preview))
        .route("/confirm", post(handlers::confirm))
        .route("/sample", get(handlers::sample))
        .layer(DefaultBodyLimit::max(BODY_LIMIT))
}
