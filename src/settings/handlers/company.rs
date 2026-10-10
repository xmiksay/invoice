use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::app::AppState;
use crate::auth::{Manage, Read};
use crate::error::{AppError, ErrorBody, FieldErrors};
use crate::extract::ApiJson;
use crate::settings::entity::company;
use crate::settings::repo;
use crate::validation as v;

/// Own company profile (singleton). Request and response share the shape.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Company {
    #[serde(default)]
    pub name: String,
    pub ico: Option<String>,
    pub dic: Option<String>,
    pub vat_payer: bool,
    #[serde(default)]
    pub street: String,
    #[serde(default)]
    pub city: String,
    #[serde(default)]
    pub zip: String,
    #[serde(default)]
    pub country: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub web: Option<String>,
    pub registration: Option<String>,
    pub default_due_days: i32,
    /// `cs` or `en`.
    pub default_locale: String,
}

impl Company {
    /// Normalize every field or report all failures at once.
    pub fn validate(self) -> Result<Self, AppError> {
        let mut e = FieldErrors::new();
        let out = Self {
            name: e
                .check("name", v::required_text(&self.name, 200))
                .unwrap_or_default(),
            ico: e.check("ico", v::opt_ico(self.ico.as_deref())).flatten(),
            dic: e.check("dic", v::opt_dic(self.dic.as_deref())).flatten(),
            vat_payer: self.vat_payer,
            street: e
                .check("street", v::text(&self.street, 200))
                .unwrap_or_default(),
            city: e
                .check("city", v::text(&self.city, 100))
                .unwrap_or_default(),
            zip: e.check("zip", v::text(&self.zip, 20)).unwrap_or_default(),
            country: e
                .check("country", v::country(&self.country))
                .unwrap_or_default(),
            email: e
                .check("email", v::opt_email(self.email.as_deref()))
                .flatten(),
            phone: e
                .check("phone", v::opt_text(self.phone.as_deref(), 50))
                .flatten(),
            web: e
                .check("web", v::opt_text(self.web.as_deref(), 200))
                .flatten(),
            registration: e
                .check(
                    "registration",
                    v::opt_text(self.registration.as_deref(), 300),
                )
                .flatten(),
            default_due_days: e
                .check("defaultDueDays", v::due_days(self.default_due_days))
                .unwrap_or_default(),
            default_locale: e
                .check("defaultLocale", v::locale(&self.default_locale))
                .unwrap_or_default(),
        };
        e.into_result()?;
        Ok(out)
    }
}

impl From<company::Model> for Company {
    fn from(m: company::Model) -> Self {
        Self {
            name: m.name,
            ico: m.ico,
            dic: m.dic,
            vat_payer: m.vat_payer,
            street: m.street,
            city: m.city,
            zip: m.zip,
            country: m.country,
            email: m.email,
            phone: m.phone,
            web: m.web,
            registration: m.registration,
            default_due_days: m.default_due_days,
            default_locale: m.default_locale,
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/settings/company",
    tag = "settings",
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = Company))
)]
pub async fn get_company(
    State(state): State<AppState>,
    access: Read,
) -> Result<Json<Company>, AppError> {
    Ok(Json(
        repo::company::get(&state.db, access.space()).await?.into(),
    ))
}

#[utoipa::path(
    put,
    path = "/api/settings/company",
    tag = "settings",
    security(("cookie" = []), ("bearer" = [])),
    request_body = Company,
    responses(
        (status = 200, body = Company),
        (status = 422, description = "Validation failed", body = ErrorBody),
    )
)]
pub async fn put_company(
    State(state): State<AppState>,
    access: Manage,
    ApiJson(input): ApiJson<Company>,
) -> Result<Json<Company>, AppError> {
    let input = input.validate()?;
    Ok(Json(
        repo::company::update(&state.db, access.space(), input)
            .await?
            .into(),
    ))
}
