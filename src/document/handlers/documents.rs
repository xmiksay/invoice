use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use uuid::Uuid;

use super::compute_input::{ComputeCtx, ComputeInput};
use super::dto::{Computed, Document, DocumentList, ListQuery, lines_out};
use super::input::DocumentInput;
use super::{fetch, received};
use crate::app::AppState;
use crate::contact::handlers::dto::ListQuery as Paging;
use crate::document::line::Status;
use crate::document::repo::{
    advance_sources, context, ddpp_correction, query, received as received_repo, view, write,
};
use crate::error::{AppError, ErrorBody};
use crate::extract::{ApiJson, ApiPath, ApiQuery, from_value};
use crate::settings::doc_type::RECEIVED;
use crate::settings::repo::company;
use crate::time::today;

#[utoipa::path(
    get,
    path = "/api/documents",
    tag = "documents",
    security(("bearer" = [])),
    params(ListQuery),
    responses((status = 200, body = DocumentList))
)]
pub async fn list(
    State(state): State<AppState>,
    ApiQuery(query): ApiQuery<ListQuery>,
) -> Result<Json<DocumentList>, AppError> {
    let (q, limit, offset) = Paging {
        q: query.q.clone(),
        limit: query.limit,
        offset: query.offset,
    }
    .normalized();
    let today = today();
    let (docs, total) = query::list(&state.db, &query, q.as_deref(), limit, offset, today).await?;
    Ok(Json(DocumentList {
        items: view::summaries(&state.db, docs, today).await?,
        total,
    }))
}

#[utoipa::path(
    post,
    path = "/api/documents",
    tag = "documents",
    security(("bearer" = [])),
    request_body(content = DocumentInput, description = "An issued draft (`DocumentInput`), or with `direction: \"received\"` a received document (`ReceivedInput`)"),
    responses(
        (status = 201, body = Document),
        (status = 409, description = "`number_taken` (received: the allocated number is used)", body = ErrorBody),
        (status = 422, description = "Validation failed", body = ErrorBody),
    )
)]
pub async fn create(
    State(state): State<AppState>,
    ApiJson(body): ApiJson<serde_json::Value>,
) -> Result<(StatusCode, Json<Document>), AppError> {
    let direction = body
        .get("direction")
        .and_then(|d| d.as_str())
        .map(str::trim);
    let id = if direction == Some(RECEIVED) {
        received::create(&state, body).await?
    } else {
        let input: DocumentInput = from_value(body)?;
        let ctx = context::load(&state.db, &input, today(), None).await?;
        let (mut data, evaluated) = input.validate(&ctx)?;
        context::resolve_bank(&state.db, &mut data, true).await?;
        write::create(&state.db, data, evaluated.totals).await?
    };
    Ok((StatusCode::CREATED, Json(fetch(&state, id).await?)))
}

#[utoipa::path(
    get,
    path = "/api/documents/{id}",
    tag = "documents",
    security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    responses((status = 200, body = Document), (status = 404, body = ErrorBody))
)]
pub async fn get(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> Result<Json<Document>, AppError> {
    Ok(Json(fetch(&state, id).await?))
}

#[utoipa::path(
    put,
    path = "/api/documents/{id}",
    tag = "documents",
    security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    request_body(content = DocumentInput, description = "`DocumentInput` for an issued draft, `ReceivedInput` for a received document"),
    responses(
        (status = 200, body = Document),
        (status = 404, body = ErrorBody),
        (status = 409, description = "`document_locked` (an issued document that is not a draft)", body = ErrorBody),
        (status = 422, description = "Validation failed", body = ErrorBody),
    )
)]
pub async fn update(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiJson(body): ApiJson<serde_json::Value>,
) -> Result<Json<Document>, AppError> {
    let doc = query::find(&state.db, id).await?;
    if doc.direction == RECEIVED {
        received::update(&state, &doc, body).await?;
        return Ok(Json(fetch(&state, id).await?));
    }
    if view::status(&doc)? != Status::Draft {
        return Err(AppError::DocumentLocked);
    }
    let input: DocumentInput = from_value(body)?;
    let ctx = context::load(&state.db, &input, today(), Some(&doc)).await?;
    let (mut data, evaluated) = input.validate(&ctx)?;
    context::resolve_bank(&state.db, &mut data, false).await?;
    write::update(&state.db, id, data, evaluated.totals).await?;
    Ok(Json(fetch(&state, id).await?))
}

#[utoipa::path(
    delete,
    path = "/api/documents/{id}",
    tag = "documents",
    security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    responses(
        (status = 204),
        (status = 404, body = ErrorBody),
        (status = 409, description = "`document_locked` (an issued document that is not a draft; received documents can always be deleted)", body = ErrorBody),
    )
)]
pub async fn delete(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> Result<StatusCode, AppError> {
    let doc = query::find(&state.db, id).await?;
    let original = if doc.direction == RECEIVED {
        received_repo::delete(&state.db, id).await?
    } else {
        write::delete(&state.db, id).await?
    };
    // After the commit: a failed delete never loses the file.
    if let Some(rel) = original {
        state.pdf.remove(&rel).await;
    }
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/documents/compute",
    tag = "documents",
    security(("bearer" = [])),
    request_body = ComputeInput,
    responses(
        (status = 200, body = Computed),
        (status = 422, description = "Same rules as save", body = ErrorBody),
    )
)]
pub async fn compute(
    State(state): State<AppState>,
    ApiJson(input): ApiJson<ComputeInput>,
) -> Result<Json<Computed>, AppError> {
    let company = company::get(&state.db).await?;
    let existing = match input.document_id {
        Some(id) => match query::find(&state.db, id).await {
            Ok(doc) => Some(context::existing(&doc)?),
            Err(AppError::NotFound) => None,
            Err(e) => return Err(e),
        },
        None => None,
    };
    let exact = ddpp_correction::basis_for(&state.db, existing.as_ref()).await?;
    let ctx = ComputeCtx {
        vat_payer: company.vat_payer,
        default_rate: context::default_vat_rate(&state.db).await?,
        default_locale: company.default_locale,
        existing,
        advances: advance_sources::load(&state.db, &input.advance_ids()).await?,
        exact,
    };
    let (lines, evaluated) = input.validate(&ctx)?;
    Ok(Json(Computed {
        lines: lines_out(&lines, &evaluated.lines),
        totals: evaluated.totals.into(),
    }))
}
