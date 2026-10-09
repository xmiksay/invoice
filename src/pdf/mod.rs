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
pub mod storage;
#[cfg(test)]
mod test_fixtures;

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Context as _;
use axum::Router;
use axum::routing::get;
use mdcast_client::{AssetBundle, Client, request};

use crate::app::AppState;
use crate::error::AppError;
use payload::Payload;

pub const DEFAULT_MDCAST_URL: &str = "https://mdcast.nexial.cz";
const MDCAST_TIMEOUT: Duration = Duration::from_secs(60);

/// The mdcast client plus where designs come from and archives go.
#[derive(Clone)]
pub struct PdfService {
    client: Client,
    design_dir: Option<PathBuf>,
    storage_dir: PathBuf,
}

impl PdfService {
    /// Builds the client (no network) and creates the storage directory.
    pub fn new(
        url: &str,
        token: Option<&str>,
        design_dir: Option<PathBuf>,
        storage_dir: PathBuf,
    ) -> anyhow::Result<Self> {
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
        std::fs::create_dir_all(&storage_dir).with_context(|| {
            format!(
                "create INVOICE__STORAGE_DIR {}",
                storage_dir.to_string_lossy()
            )
        })?;
        Ok(Self {
            client,
            design_dir,
            storage_dir,
        })
    }

    pub fn design_dir(&self) -> Option<&Path> {
        self.design_dir.as_deref()
    }

    /// Render `payload` with the current design (re-read every time).
    pub async fn render(&self, mut payload: Payload) -> Result<Vec<u8>, AppError> {
        let dir = self.design_dir.clone();
        let design = tokio::task::spawn_blocking(move || design::load(dir.as_deref()))
            .await
            .context("design loader panicked")??;
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

    async fn blocking<T: Send + 'static>(
        &self,
        f: impl FnOnce(&Path) -> anyhow::Result<T> + Send + 'static,
    ) -> anyhow::Result<T> {
        let root = self.storage_dir.clone();
        tokio::task::spawn_blocking(move || f(&root))
            .await
            .context("storage task panicked")?
    }

    pub async fn write(&self, rel: &str, bytes: Vec<u8>) -> anyhow::Result<()> {
        let rel = rel.to_string();
        self.blocking(move |root| storage::write_atomic(root, &rel, &bytes))
            .await
    }

    pub async fn read(&self, rel: &str) -> anyhow::Result<Vec<u8>> {
        let rel = rel.to_string();
        self.blocking(move |root| storage::read(root, &rel)).await
    }

    pub async fn remove(&self, rel: &str) {
        let rel = rel.to_string();
        let removed = self
            .blocking(move |root| {
                storage::remove(root, &rel);
                Ok(())
            })
            .await;
        if let Err(e) = removed {
            tracing::warn!(error = %e, "remove orphaned PDF");
        }
    }
}

/// Routes relative to `/api/pdf`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/preview", get(handlers::preview))
        .route("/design", get(handlers::design))
}
