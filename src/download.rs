//! File download responses.

use axum::body::Body;
use axum::http::header;
use axum::response::{IntoResponse, Response};

/// `content_type` + `Content-Disposition: attachment` + `no-store`.
/// `filename` must already be header-safe.
pub fn attachment(content_type: &str, filename: &str, body: impl Into<Body>) -> Response {
    (
        [
            (header::CONTENT_TYPE, content_type.to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
            (header::CACHE_CONTROL, "no-store".to_string()),
        ],
        body.into(),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headers() {
        let r = attachment("text/csv; charset=utf-8", "a.csv", "x");
        let h = r.headers();
        assert_eq!(h[header::CONTENT_TYPE], "text/csv; charset=utf-8");
        assert_eq!(
            h[header::CONTENT_DISPOSITION],
            "attachment; filename=\"a.csv\""
        );
        assert_eq!(h[header::CACHE_CONTROL], "no-store");
    }
}
