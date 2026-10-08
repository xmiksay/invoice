//! Contacts: one address book for customers and suppliers.

pub mod entity;
pub mod handlers;
pub mod repo;

use axum::Router;
use axum::routing::get;

use crate::app::AppState;
use handlers::contacts;

/// Routes relative to `/api/contacts`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(contacts::list).post(contacts::create))
        .route(
            "/{id}",
            get(contacts::get)
                .put(contacts::update)
                .delete(contacts::delete),
        )
}
