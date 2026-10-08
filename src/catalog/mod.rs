//! Catalog: reusable items and groups of items (inserted into documents by
//! the client).

pub mod entity;
pub mod handlers;
pub mod repo;

use axum::Router;
use axum::routing::get;

use crate::app::AppState;
use handlers::{groups, items};

/// Routes relative to `/api/catalog`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/items", get(items::list).post(items::create))
        .route(
            "/items/{id}",
            get(items::get).put(items::update).delete(items::delete),
        )
        .route("/groups", get(groups::list).post(groups::create))
        .route(
            "/groups/{id}",
            get(groups::get).put(groups::update).delete(groups::delete),
        )
}
