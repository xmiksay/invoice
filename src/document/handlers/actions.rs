use axum::Json;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use uuid::Uuid;

use super::dto::{CancelInput, CreditNoteInput, Document, InternalNoteInput, MarkSentInput};
use super::fetch;
use super::meta::MetadataInput;
use crate::app::AppState;
use crate::document::repo::{
    credit as credit_repo, issue as issue_repo, lifecycle, meta as meta_repo, query,
    settle as settle_repo,
};
use crate::error::{AppError, ErrorBody, FieldErrors};
use crate::extract::{ApiJson, ApiPath, optional_json};
use crate::time::today;
use crate::validation as v;

#[utoipa::path(
    post,
    path = "/api/documents/{id}/issue",
    tag = "documents",
    security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, body = Document),
        (status = 404, body = ErrorBody),
        (status = 409, description = "`invalid_state` (not a draft)", body = ErrorBody),
        (status = 422, description = "Not issuable (contactId, lines, dueDate, taxPointDate, bankAccountId, exchangeRate, correctionReason, lines.N.advanceDocumentId, lines: exceeds_original)", body = ErrorBody),
        (status = 502, description = "`pdf_render_failed` (+ `detail`); nothing issued", body = ErrorBody),
        (status = 503, description = "`pdf_unavailable`; nothing issued", body = ErrorBody),
    )
)]
pub async fn issue(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> Result<Json<Document>, AppError> {
    issue_repo::issue(&state.db, &state.cnb, &state.pdf, id, today()).await?;
    Ok(Json(fetch(&state, id).await?))
}

#[utoipa::path(
    post,
    path = "/api/documents/{id}/cancel",
    tag = "documents",
    security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    request_body = CancelInput,
    responses(
        (status = 200, body = Document),
        (status = 404, body = ErrorBody),
        (status = 409, description = "`invalid_state` (not issued, a native DDPP, or a proforma with payments); `exceeds_original` (a debit note whose credit notes would exceed the lowered cap); DDPP correction whose DDPP is deducted: `advance_settled` / `advance_in_use`; imported DDPP with a live correction: `advance_in_use`", body = ErrorBody),
    )
)]
pub async fn cancel(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    body: Bytes,
) -> Result<Json<Document>, AppError> {
    let input: CancelInput = optional_json(&body)?;
    let mut e = FieldErrors::new();
    let reason = e
        .check("reason", v::opt_text(input.reason.as_deref(), 2000))
        .flatten();
    e.into_result()?;
    lifecycle::cancel(&state.db, id, reason).await?;
    Ok(Json(fetch(&state, id).await?))
}

#[utoipa::path(
    post,
    path = "/api/documents/{id}/mark-sent",
    tag = "documents",
    security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    request_body = MarkSentInput,
    responses(
        (status = 200, body = Document),
        (status = 404, body = ErrorBody),
        (status = 409, description = "`invalid_state` (not issued, or a received document)", body = ErrorBody),
    )
)]
pub async fn mark_sent(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    body: Bytes,
) -> Result<Json<Document>, AppError> {
    let input: MarkSentInput = optional_json(&body)?;
    let sent_at = input
        .sent_at
        .unwrap_or_else(|| chrono::Utc::now().fixed_offset());
    lifecycle::mark_sent(&state.db, id, sent_at).await?;
    Ok(Json(fetch(&state, id).await?))
}

#[utoipa::path(
    put,
    path = "/api/documents/{id}/internal-note",
    tag = "documents",
    security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    request_body = InternalNoteInput,
    responses(
        (status = 200, body = Document),
        (status = 404, body = ErrorBody),
        (status = 422, description = "`internalNote`: `too_long`", body = ErrorBody),
    )
)]
pub async fn internal_note(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiJson(input): ApiJson<InternalNoteInput>,
) -> Result<Json<Document>, AppError> {
    let note = v::opt_text(input.internal_note.as_deref(), 2000)
        .map_err(|r| AppError::field("internalNote", r))?;
    lifecycle::set_internal_note(&state.db, id, note).await?;
    Ok(Json(fetch(&state, id).await?))
}

#[utoipa::path(
    put,
    path = "/api/documents/{id}/metadata",
    tag = "documents",
    security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    request_body = MetadataInput,
    responses(
        (status = 200, body = Document),
        (status = 404, body = ErrorBody),
        (status = 422, description = "`categoryId` (`invalid` / `inactive`), `customFields.<key>`, `internalNote`", body = ErrorBody),
    )
)]
pub async fn metadata(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiJson(input): ApiJson<MetadataInput>,
) -> Result<Json<Document>, AppError> {
    let doc = query::find(&state.db, id).await?;
    let ctx = meta_repo::load(&state.db, &doc.direction, input.category_id, Some(&doc)).await?;
    let (meta, note) = input.validate(&doc.direction, &ctx)?;
    meta_repo::set(&state.db, id, meta, note).await?;
    Ok(Json(fetch(&state, id).await?))
}

#[utoipa::path(
    post,
    path = "/api/documents/{id}/settle",
    tag = "documents",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "The proforma")),
    responses(
        (status = 201, description = "Draft final invoice", body = Document),
        (status = 404, body = ErrorBody),
        (status = 409, description = "`invalid_state` (not an issued proforma, or already settled)", body = ErrorBody),
    )
)]
pub async fn settle(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> Result<(StatusCode, Json<Document>), AppError> {
    let new_id = settle_repo::settle(&state.db, id, today()).await?;
    Ok((StatusCode::CREATED, Json(fetch(&state, new_id).await?)))
}

#[utoipa::path(
    post,
    path = "/api/documents/{id}/credit-note",
    tag = "documents",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "The invoice / simplified document, or the DDPP")),
    request_body = CreditNoteInput,
    responses(
        (status = 201, description = "Draft credit note (`credit_note`), or on a DDPP its correction (`advance_credit_note`)", body = Document),
        (status = 404, body = ErrorBody),
        (status = 409, description = "`invalid_state` (not an issued invoice / simplified document / DDPP); DDPP deducted by an invoice: `advance_settled` (issued) / `advance_in_use` (draft)", body = ErrorBody),
        (status = 422, description = "`correctionReason`: `too_long`", body = ErrorBody),
    )
)]
pub async fn credit_note(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    body: Bytes,
) -> Result<(StatusCode, Json<Document>), AppError> {
    let reason = correction_reason(&body)?;
    let new_id = credit_repo::create(&state.db, id, reason, today()).await?;
    Ok((StatusCode::CREATED, Json(fetch(&state, new_id).await?)))
}

#[utoipa::path(
    post,
    path = "/api/documents/{id}/debit-note",
    tag = "documents",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "The invoice / simplified document")),
    request_body = CreditNoteInput,
    responses(
        (status = 201, description = "Draft debit note without lines", body = Document),
        (status = 404, body = ErrorBody),
        (status = 409, description = "`invalid_state` (not an issued invoice / simplified document)", body = ErrorBody),
        (status = 422, description = "`correctionReason`: `too_long`", body = ErrorBody),
    )
)]
pub async fn debit_note(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    body: Bytes,
) -> Result<(StatusCode, Json<Document>), AppError> {
    let reason = correction_reason(&body)?;
    let new_id = credit_repo::create_debit(&state.db, id, reason, today()).await?;
    Ok((StatusCode::CREATED, Json(fetch(&state, new_id).await?)))
}

/// The optional `{ correctionReason }` body of the correction endpoints.
fn correction_reason(body: &Bytes) -> Result<Option<String>, AppError> {
    let input: CreditNoteInput = optional_json(body)?;
    v::opt_text(input.correction_reason.as_deref(), 500)
        .map_err(|r| AppError::field("correctionReason", r))
}
