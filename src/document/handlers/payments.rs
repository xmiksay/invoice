use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use rust_decimal::Decimal;
use uuid::Uuid;

use super::dto::{Payment, PaymentInput};
use super::input::exchange_rate;
use super::line_input::decimal;
use crate::app::AppState;
use crate::document::line::MAX_AMOUNT;
use crate::document::repo::payments::{self as repo, NewPayment, WithAdvance};
use crate::error::{AppError, ErrorBody, FieldErrors};
use crate::extract::{ApiJson, ApiPath};
use crate::pdf::archive;
use crate::time::today;
use crate::validation as v;

impl From<WithAdvance> for Payment {
    fn from((m, advance): WithAdvance) -> Self {
        Self {
            id: m.id,
            date: m.date,
            amount: crate::document::compute::round2(m.amount),
            note: m.note,
            advance_document_id: advance,
            created_at: m.created_at,
        }
    }
}

impl PaymentInput {
    pub fn validate(self) -> Result<NewPayment, AppError> {
        let mut e = FieldErrors::new();
        let date = e.check("date", self.date.ok_or("required"));
        let amount = e.check(
            "amount",
            decimal(&self.amount, 2, MAX_AMOUNT).and_then(|a| {
                if a > Decimal::ZERO {
                    Ok(a)
                } else {
                    Err("invalid")
                }
            }),
        );
        let note = e
            .check("note", v::opt_text(self.note.as_deref(), 500))
            .flatten();
        let exchange_rate = e
            .check("exchangeRate", exchange_rate(self.exchange_rate.as_deref()))
            .flatten();
        e.into_result()?;
        Ok(NewPayment {
            date: date.unwrap_or_default(),
            amount: amount.unwrap_or_default(),
            note,
            exchange_rate,
        })
    }
}

#[utoipa::path(
    get,
    path = "/api/documents/{id}/payments",
    tag = "documents",
    security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    responses((status = 200, body = Vec<Payment>), (status = 404, body = ErrorBody))
)]
pub async fn list(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> Result<Json<Vec<Payment>>, AppError> {
    let rows = repo::list(&state.db, id).await?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

#[utoipa::path(
    post,
    path = "/api/documents/{id}/payments",
    tag = "documents",
    security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    request_body = PaymentInput,
    responses(
        (status = 201, body = Payment),
        (status = 404, body = ErrorBody),
        (status = 409, description = "`invalid_state` (not issued, or a DDPP)", body = ErrorBody),
        (status = 422, description = "Validation failed; `exchangeRate: required` when a DDPP needs a rate ČNB cannot give", body = ErrorBody),
    )
)]
pub async fn create(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiJson(input): ApiJson<PaymentInput>,
) -> Result<(StatusCode, Json<Payment>), AppError> {
    let row = repo::create(&state.db, &state.cnb, id, input.validate()?, today()).await?;
    if let Some(ddpp) = row.1 {
        archive::spawn_archive_ddpp(state.db.clone(), state.pdf.clone(), ddpp);
    }
    Ok((StatusCode::CREATED, Json(row.into())))
}

#[utoipa::path(
    delete,
    path = "/api/documents/{id}/payments/{paymentId}",
    tag = "documents",
    security(("bearer" = [])),
    params(("id" = Uuid, Path), ("paymentId" = Uuid, Path)),
    responses(
        (status = 204),
        (status = 404, body = ErrorBody),
        (status = 409, description = "`invalid_state` (not issued) / `advance_settled` (its DDPP is deducted by an issued invoice)", body = ErrorBody),
    )
)]
pub async fn delete(
    State(state): State<AppState>,
    ApiPath((id, payment_id)): ApiPath<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    repo::delete(&state.db, id, payment_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
