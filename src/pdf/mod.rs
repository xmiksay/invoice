//! PDF rendering through the mdcast service: design files, the `/data.json`
//! payload, the SPAYD QR code, and the archive of issued documents.

pub mod amounts;
pub mod archive;
pub mod client;
pub mod design;
pub mod format;
pub mod handlers;
pub mod labels;
pub mod payload;
pub mod preview;
pub mod reference;
pub mod source;
pub mod spayd;
#[cfg(test)]
mod test_fixtures;

use std::time::Duration;

use anyhow::Context as _;
use axum::Router;
use axum::routing::get;
use mdcast_client::{AssetBundle, Client, request};

use crate::app::AppState;
use crate::error::AppError;
use crate::storage::Storage;
use payload::Payload;

pub const DEFAULT_MDCAST_URL: &str = "https://mdcast.nexial.cz";
const MDCAST_TIMEOUT: Duration = Duration::from_secs(60);

/// The mdcast client plus the storage designs come from and archives go to.
#[derive(Clone)]
pub struct PdfService {
    client: Client,
    storage: Storage,
    design_cache: design::Cache,
}

impl PdfService {
    /// Builds the client (no network).
    pub fn new(url: &str, token: Option<&str>, storage: Storage) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(MDCAST_TIMEOUT)
            .build()
            .context("build mdcast HTTP client")?;
        // mdcast-client insists on a token; a server without a bearer gate
        // ignores the header, so a placeholder is safe.
        let token = token
            .filter(|t| !t.trim().is_empty())
            .unwrap_or("unauthenticated");
        let client = Client::builder(url)
            .token(token)
            .http_client(http)
            .build()
            .context("INVOICE__MDCAST_URL must start with http:// or https://")?;
        Ok(Self {
            client,
            storage,
            design_cache: design::Cache::default(),
        })
    }

    /// Archives, originals and the design overrides.
    pub fn storage(&self) -> &Storage {
        &self.storage
    }

    pub fn design_cache(&self) -> &design::Cache {
        &self.design_cache
    }

    /// Render `payload` with the current design (listed every time).
    pub async fn render(&self, mut payload: Payload) -> Result<Vec<u8>, AppError> {
        let design = design::load(&self.storage, &self.design_cache).await?;
        payload.assets = payload::Assets {
            logo: design.logo(),
            signature: design.signature(),
        };
        let fonts = design.fonts();
        let mut bundle = AssetBundle::new();
        for (key, bytes) in design.files {
            bundle.insert(key, bytes);
        }
        if let Some(s) = &payload.spayd {
            bundle.insert(design::QR_IMAGE, spayd::qr_svg(s)?.into_bytes());
        }
        let data = serde_json::to_string(&payload).context("serialize PDF payload")?;
        let mut req = request::template(design.template, data);
        req.fonts = fonts;
        let artifact = self
            .client
            .render_template(req, &bundle)
            .await
            .map_err(client::map_error)?;
        Ok(artifact.bytes.to_vec())
    }
}

/// Routes relative to `/api/pdf`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/preview", get(handlers::preview))
        .route("/design", get(handlers::design))
}
