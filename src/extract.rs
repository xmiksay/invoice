//! Axum extractors whose rejections use the API error format instead of
//! axum's plain-text bodies.

use axum::extract::{FromRequest, FromRequestParts};

use crate::error::AppError;

/// JSON body; malformed JSON or a wrong field type → 400 `bad_request`.
#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(AppError))]
pub struct ApiJson<T>(pub T);

/// Query string; unparsable parameters → 400 `bad_request`.
#[derive(FromRequestParts)]
#[from_request(via(axum::extract::Query), rejection(AppError))]
pub struct ApiQuery<T>(pub T);

/// Path parameters; an unparsable segment (e.g. a non-UUID id) → 404 `not_found`.
#[derive(FromRequestParts)]
#[from_request(via(axum::extract::Path), rejection(AppError))]
pub struct ApiPath<T>(pub T);
