use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::app::AppState;
use crate::auth::{Manage, Read};
use crate::error::{AppError, ErrorBody, FieldErrors};
use crate::extract::{ApiJson, ApiPath};
use crate::settings::entity::bank_account;
use crate::settings::repo;
use crate::validation as v;

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BankAccount {
    pub id: Uuid,
    pub label: Option<String>,
    pub currency: String,
    pub account_number: Option<String>,
    pub iban: Option<String>,
    pub bic: Option<String>,
    pub is_default: bool,
}

impl From<bank_account::Model> for BankAccount {
    fn from(m: bank_account::Model) -> Self {
        Self {
            id: m.id,
            label: m.label,
            currency: m.currency,
            account_number: m.account_number,
            iban: m.iban,
            bic: m.bic,
            is_default: m.is_default,
        }
    }
}

/// Create/update body. At least one of `accountNumber` / `iban` is required.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct BankAccountInput {
    pub label: Option<String>,
    pub currency: String,
    pub account_number: Option<String>,
    pub iban: Option<String>,
    pub bic: Option<String>,
    pub is_default: bool,
}

impl BankAccountInput {
    pub fn validate(self) -> Result<Self, AppError> {
        let mut e = FieldErrors::new();
        let out = Self {
            label: e
                .check("label", v::opt_text(self.label.as_deref(), 100))
                .flatten(),
            currency: e
                .check("currency", v::currency(&self.currency))
                .unwrap_or_default(),
            account_number: e
                .check(
                    "accountNumber",
                    v::opt_cz_account(self.account_number.as_deref()),
                )
                .flatten(),
            iban: e.check("iban", v::opt_iban(self.iban.as_deref())).flatten(),
            bic: e.check("bic", v::opt_bic(self.bic.as_deref())).flatten(),
            is_default: self.is_default,
        };
        let raw_blank = |s: &Option<String>| s.as_deref().is_none_or(|s| s.trim().is_empty());
        if raw_blank(&self.account_number) && raw_blank(&self.iban) {
            e.add("accountNumber", "required");
        }
        e.into_result()?;
        Ok(out)
    }
}

#[utoipa::path(
    get,
    path = "/api/settings/bank-accounts",
    tag = "settings",
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = Vec<BankAccount>))
)]
pub async fn list(
    State(state): State<AppState>,
    access: Read,
) -> Result<Json<Vec<BankAccount>>, AppError> {
    let rows = repo::bank_accounts::list(&state.db, access.space()).await?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

#[utoipa::path(
    post,
    path = "/api/settings/bank-accounts",
    tag = "settings",
    security(("cookie" = []), ("bearer" = [])),
    request_body = BankAccountInput,
    responses(
        (status = 201, body = BankAccount),
        (status = 422, description = "Validation failed", body = ErrorBody),
    )
)]
pub async fn create(
    State(state): State<AppState>,
    access: Manage,
    ApiJson(input): ApiJson<BankAccountInput>,
) -> Result<(StatusCode, Json<BankAccount>), AppError> {
    let row = repo::bank_accounts::create(&state.db, access.space(), input.validate()?).await?;
    Ok((StatusCode::CREATED, Json(row.into())))
}

#[utoipa::path(
    put,
    path = "/api/settings/bank-accounts/{id}",
    tag = "settings",
    security(("cookie" = []), ("bearer" = [])),
    params(("id" = Uuid, Path)),
    request_body = BankAccountInput,
    responses(
        (status = 200, body = BankAccount),
        (status = 404, body = ErrorBody),
        (status = 422, description = "Validation failed", body = ErrorBody),
    )
)]
pub async fn update(
    State(state): State<AppState>,
    access: Manage,
    ApiPath(id): ApiPath<Uuid>,
    ApiJson(input): ApiJson<BankAccountInput>,
) -> Result<Json<BankAccount>, AppError> {
    let row = repo::bank_accounts::update(&state.db, access.space(), id, input.validate()?).await?;
    Ok(Json(row.into()))
}

#[utoipa::path(
    delete,
    path = "/api/settings/bank-accounts/{id}",
    tag = "settings",
    security(("cookie" = []), ("bearer" = [])),
    params(("id" = Uuid, Path)),
    responses((status = 204), (status = 404, body = ErrorBody))
)]
pub async fn delete(
    State(state): State<AppState>,
    access: Manage,
    ApiPath(id): ApiPath<Uuid>,
) -> Result<StatusCode, AppError> {
    repo::bank_accounts::delete(&state.db, access.space(), id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields(err: AppError) -> String {
        format!("{err:?}")
    }

    #[test]
    fn requires_account_number_or_iban() {
        let input = BankAccountInput {
            currency: "CZK".into(),
            ..Default::default()
        };
        assert!(
            fields(input.validate().expect_err("must fail"))
                .contains("\"accountNumber\": \"required\"")
        );
    }

    #[test]
    fn normalizes_fields() {
        let input = BankAccountInput {
            label: Some("  ".into()),
            currency: "eur".into(),
            iban: Some("cz65 0800 0000 1920 0014 5399".into()),
            bic: Some("gibaczpx".into()),
            ..Default::default()
        }
        .validate()
        .expect("valid");
        assert_eq!(input.label, None);
        assert_eq!(input.currency, "EUR");
        assert_eq!(input.iban.as_deref(), Some("CZ6508000000192000145399"));
        assert_eq!(input.bic.as_deref(), Some("GIBACZPX"));
    }

    #[test]
    fn invalid_iban_is_not_reported_as_missing() {
        let err = BankAccountInput {
            currency: "CZK".into(),
            iban: Some("CZ00".into()),
            ..Default::default()
        }
        .validate()
        .expect_err("must fail");
        let f = fields(err);
        assert!(f.contains("\"iban\": \"invalid\""));
        assert!(!f.contains("accountNumber"));
    }
}
