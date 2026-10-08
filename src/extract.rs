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

/// An optional JSON body: empty (or whitespace) → `T::default()`, malformed →
/// 400 `bad_request`. For action routes whose body is all-optional.
pub fn optional_json<T: serde::de::DeserializeOwned + Default>(body: &[u8]) -> Result<T, AppError> {
    if body.iter().all(u8::is_ascii_whitespace) {
        return Ok(T::default());
    }
    serde_json::from_slice(body).map_err(|e| AppError::BadRequest(e.to_string()))
}

/// An already-parsed JSON body as `T`; a wrong shape → 400 `bad_request`.
/// For routes that pick the body type from the body itself.
pub fn from_value<T: serde::de::DeserializeOwned>(body: serde_json::Value) -> Result<T, AppError> {
    serde_json::from_value(body).map_err(|e| AppError::BadRequest(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default, PartialEq, serde::Deserialize)]
    struct Body {
        a: Option<i32>,
    }

    #[test]
    fn optional_json_accepts_empty_and_rejects_garbage() {
        assert_eq!(optional_json::<Body>(b"").expect("empty"), Body::default());
        assert_eq!(
            optional_json::<Body>(b" \n").expect("blank"),
            Body::default()
        );
        assert_eq!(
            optional_json::<Body>(b"{\"a\":1}").expect("json"),
            Body { a: Some(1) }
        );
        assert!(matches!(
            optional_json::<Body>(b"{"),
            Err(AppError::BadRequest(_))
        ));
    }
}
