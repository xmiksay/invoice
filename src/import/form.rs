//! The multipart upload of the import routes: file parts (≤ 50 MiB
//! together) plus an optional `options` text part holding JSON.

use axum::extract::Multipart;
use axum::extract::multipart::{MultipartError, MultipartRejection};
use axum::http::StatusCode;
use serde::de::DeserializeOwned;

use crate::error::AppError;

/// Largest accepted upload (all files together).
pub const MAX_UPLOAD: usize = 50 * 1024 * 1024;
/// Request body limit of the import routes: the files plus multipart overhead.
pub const BODY_LIMIT: usize = MAX_UPLOAD + 1024 * 1024;

pub struct File {
    pub name: String,
    pub bytes: Vec<u8>,
}

fn multipart_error(e: MultipartError) -> AppError {
    if e.status() == StatusCode::PAYLOAD_TOO_LARGE {
        AppError::TooLarge
    } else {
        AppError::BadRequest(e.body_text())
    }
}

/// The `part` file parts (> [`MAX_UPLOAD`] together → `too_large`; none →
/// 422 `{part}: required`) and the raw `options` part; other parts are
/// ignored.
pub async fn read(
    form: Result<Multipart, MultipartRejection>,
    part: &str,
) -> Result<(Vec<File>, Option<String>), AppError> {
    let mut form = form.map_err(|e| AppError::BadRequest(e.body_text()))?;
    let (mut files, mut options, mut total) = (Vec::new(), None, 0usize);
    while let Some(mut field) = form.next_field().await.map_err(multipart_error)? {
        match field.name() {
            Some(name) if name == part => {
                let name = field
                    .file_name()
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("file{}", files.len() + 1));
                let mut bytes = Vec::new();
                while let Some(chunk) = field.chunk().await.map_err(multipart_error)? {
                    total += chunk.len();
                    if total > MAX_UPLOAD {
                        return Err(AppError::TooLarge);
                    }
                    bytes.extend_from_slice(&chunk);
                }
                files.push(File { name, bytes });
            }
            Some("options") => options = Some(field.text().await.map_err(multipart_error)?),
            _ => {}
        }
    }
    if files.is_empty() {
        return Err(AppError::field(part, "required"));
    }
    Ok((files, options))
}

/// The `options` JSON; missing or malformed → 422 `options: invalid`.
pub fn options<T: DeserializeOwned>(raw: Option<String>) -> Result<T, AppError> {
    raw.and_then(|s| serde_json::from_str(&s).ok())
        .ok_or(AppError::field("options", "invalid"))
}
