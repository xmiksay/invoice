use axum::Json;
use axum::body::Body;
use axum::extract::State;
use axum::http::header;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use super::archive::{self, PdfBody};
use super::design::{self, DesignFile};
use super::format::Locale;
use super::payload;
use super::preview::Sample;
use crate::app::AppState;
use crate::document::line::Status;
use crate::error::{AppError, ErrorBody};
use crate::extract::{ApiPath, ApiQuery};
use crate::settings::repo::company;

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct DownloadQuery {
    /// `1` → `Content-Disposition: attachment`.
    pub download: Option<String>,
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct PreviewQuery {
    /// `cs` | `en`; default: the company's `defaultLocale`.
    pub locale: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DesignListing {
    /// The storage backend the custom files come from: `fs` | `s3`.
    pub storage: String,
    pub files: Vec<DesignFile>,
}

/// Keeps the header value ASCII and free of quotes / path separators.
pub(crate) fn safe_filename(stem: &str) -> String {
    stem.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn pdf_response(body: PdfBody, stem: &str, download: bool) -> Response {
    let disposition = format!(
        "{}; filename=\"{}.pdf\"",
        if download { "attachment" } else { "inline" },
        safe_filename(stem)
    );
    let (length, body) = match body {
        PdfBody::Rendered(bytes) => (bytes.len() as u64, Body::from(bytes)),
        PdfBody::Stored(d) => (d.size, Body::from_stream(d.stream)),
    };
    (
        [
            (header::CONTENT_TYPE, "application/pdf".to_string()),
            (header::CONTENT_DISPOSITION, disposition),
            (header::CACHE_CONTROL, "no-store".to_string()),
            (header::CONTENT_LENGTH, length.to_string()),
        ],
        body,
    )
        .into_response()
}

#[utoipa::path(
    get,
    path = "/api/documents/{id}/pdf",
    tag = "pdf",
    security(("bearer" = [])),
    params(("id" = Uuid, Path), DownloadQuery),
    responses(
        (status = 200, description = "The PDF: received / imported → the uploaded original; a draft rendered live (watermark, no QR, never stored); otherwise the archive", content_type = "application/pdf"),
        (status = 404, description = "`not_found`, or `pdf_missing` (received / imported without an original)", body = ErrorBody),
        (status = 502, description = "`pdf_render_failed` (+ `detail`)", body = ErrorBody),
        (status = 503, description = "`pdf_unavailable`, `storage_unavailable`", body = ErrorBody),
    )
)]
pub async fn document_pdf(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiQuery(q): ApiQuery<DownloadQuery>,
) -> Result<Response, AppError> {
    let (row, body) = archive::document_pdf(&state.db, &state.pdf, id).await?;
    // An imported draft already has its own number.
    let unnumbered = row.status == Status::Draft.as_str() && !row.imported;
    let stem = match (&row.number, unnumbered) {
        (Some(number), false) => number.clone(),
        _ => format!("draft-{}", &id.simple().to_string()[..8]),
    };
    let download = matches!(q.download.as_deref(), Some("1" | "true"));
    Ok(pdf_response(body, &stem, download))
}

#[utoipa::path(
    get,
    path = "/api/pdf/preview",
    tag = "pdf",
    security(("bearer" = [])),
    params(PreviewQuery),
    responses(
        (status = 200, description = "A sample invoice in the current design", content_type = "application/pdf"),
        (status = 422, description = "`locale`: `invalid`", body = ErrorBody),
        (status = 502, description = "`pdf_render_failed` (+ `detail`)", body = ErrorBody),
        (status = 503, description = "`pdf_unavailable`, `storage_unavailable`", body = ErrorBody),
    )
)]
pub async fn preview(
    State(state): State<AppState>,
    ApiQuery(q): ApiQuery<PreviewQuery>,
) -> Result<Response, AppError> {
    let company = company::get(&state.db).await?;
    let locale = match q.locale.as_deref() {
        Some(l) => Locale::parse(l).ok_or(AppError::field("locale", "invalid"))?,
        None => Locale::parse(&company.default_locale).unwrap_or(Locale::Cs),
    };
    let sample = Sample::load(&state.db, &company, locale).await?;
    let bytes = state.pdf.render(payload::build(&sample.input())?).await?;
    Ok(pdf_response(
        PdfBody::Rendered(bytes.into()),
        "preview",
        false,
    ))
}

#[utoipa::path(
    get,
    path = "/api/pdf/design",
    tag = "pdf",
    security(("bearer" = [])),
    responses(
        (status = 200, body = DesignListing),
        (status = 503, description = "`storage_unavailable`", body = ErrorBody),
    )
)]
pub async fn design(State(state): State<AppState>) -> Result<Json<DesignListing>, AppError> {
    let storage = state.pdf.storage();
    let files = design::list(storage).await?;
    Ok(Json(DesignListing {
        storage: storage.kind().to_string(),
        files,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filenames_are_header_safe() {
        assert_eq!(safe_filename("20260001"), "20260001");
        assert_eq!(safe_filename("FV-2026/01 \"x\""), "FV-2026_01__x_");
        assert_eq!(safe_filename("Č-1"), "_-1");
    }
}
