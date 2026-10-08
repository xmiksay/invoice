pub mod actions;
pub mod compute_input;
pub mod documents;
pub mod dto;
pub mod existing;
pub mod fields;
pub mod imported;
pub mod input;
pub mod line_input;
pub mod line_out;
pub mod meta;
pub mod original;
pub mod payments;
pub mod received;
pub mod received_input;

use uuid::Uuid;

use crate::app::AppState;
use crate::document::repo::{query, view};
use crate::error::AppError;
use crate::time::today;

/// Re-read a document and build its response.
pub async fn fetch(state: &AppState, id: Uuid) -> Result<dto::Document, AppError> {
    view::document(query::load(&state.db, id).await?, today())
}
