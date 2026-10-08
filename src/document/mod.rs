//! Documents: issued invoices (1b) — lines, VAT recap, issue, cancel, payments.

pub mod compute;
pub mod entity;
pub mod handlers;
pub mod line;
pub mod repo;
pub mod state;
pub mod subtotals;

use axum::Router;
use axum::routing::{delete, get, post, put};

use crate::app::AppState;
use handlers::{actions, documents, payments};

/// Routes relative to `/api/documents`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(documents::list).post(documents::create))
        .route("/compute", post(documents::compute))
        .route(
            "/{id}",
            get(documents::get)
                .put(documents::update)
                .delete(documents::delete),
        )
        .route("/{id}/issue", post(actions::issue))
        .route("/{id}/cancel", post(actions::cancel))
        .route("/{id}/mark-sent", post(actions::mark_sent))
        .route("/{id}/internal-note", put(actions::internal_note))
        .route("/{id}/payments", get(payments::list).post(payments::create))
        .route("/{id}/payments/{payment_id}", delete(payments::delete))
}
