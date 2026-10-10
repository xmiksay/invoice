//! The design file set: the default design embedded from `design/`,
//! overridden file by file by the space's storage keys `design/…` (listed on every
//! render; file contents cached by key + version, so unchanged files are not
//! downloaded again).

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use anyhow::Context as _;
use bytes::Bytes;
use rust_embed::Embed;
use serde::Serialize;
use utoipa::ToSchema;

use crate::error::AppError;
use crate::storage::local::{is_hidden, rel_key};
use crate::storage::{DESIGN_PREFIX, Object, Storage, Version};

#[derive(Embed)]
#[folder = "design/"]
struct DefaultDesign;

pub const TEMPLATE: &str = "invoice.typ";
/// Generated per render; a design file of this name is ignored.
pub const QR_IMAGE: &str = "qr.svg";
pub const MAX_FILE_BYTES: u64 = 10 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Custom,
    Default,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct DesignFile {
    pub path: String,
    pub source: Source,
    pub size: u64,
}

/// The design path of a storage key: `design/fonts/a.ttf` → `fonts/a.ttf`;
/// `None` outside the prefix or for hidden segments.
pub fn design_path(key: &str) -> Option<&str> {
    rel_key(key, DESIGN_PREFIX)
}

pub fn key(path: &str) -> String {
    format!("{DESIGN_PREFIX}/{path}")
}

/// Remove the custom file `path` (`invoice design rm`); `false` when none is
/// stored (a built-in default cannot be removed).
pub async fn remove(storage: &Storage, path: &str) -> Result<bool, crate::storage::Error> {
    let key = key(path);
    if storage.head(&key).await?.is_none() {
        return Ok(false);
    }
    storage.delete(&key).await?;
    Ok(true)
}

fn embedded() -> BTreeMap<String, u64> {
    DefaultDesign::iter()
        .filter(|p| !p.split('/').any(is_hidden))
        .filter_map(|p| {
            let size = DefaultDesign::get(&p)?.data.len() as u64;
            Some((p.into_owned(), size))
        })
        .collect()
}

fn embedded_bytes(path: &str) -> anyhow::Result<Bytes> {
    Ok(
        match DefaultDesign::get(path)
            .with_context(|| format!("embedded design file {path} vanished"))?
            .data
        {
            Cow::Borrowed(b) => Bytes::from_static(b),
            // Debug builds read the embedded folder from disk.
            Cow::Owned(v) => Bytes::from(v),
        },
    )
}

/// The stored overrides by design path.
async fn custom(storage: &Storage) -> Result<BTreeMap<String, Object>, AppError> {
    Ok(storage
        .list(DESIGN_PREFIX)
        .await?
        .into_iter()
        .filter_map(|o| Some((design_path(&o.key)?.to_string(), o)))
        .collect())
}

/// The effective set for `GET /api/pdf/design`, sorted by path.
pub async fn list(storage: &Storage) -> Result<Vec<DesignFile>, AppError> {
    let mut files: BTreeMap<String, DesignFile> = embedded()
        .into_iter()
        .map(|(path, size)| {
            let file = DesignFile {
                path: path.clone(),
                source: Source::Default,
                size,
            };
            (path, file)
        })
        .collect();
    for (path, o) in custom(storage).await? {
        let file = DesignFile {
            path: path.clone(),
            source: Source::Custom,
            size: o.version.size,
        };
        files.insert(path, file);
    }
    Ok(files.into_values().collect())
}

fn is_font(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.starts_with("fonts/") && (lower.ends_with(".ttf") || lower.ends_with(".otf"))
}

/// Downloaded overrides by storage key. Shared by every clone of the PDF
/// service; entries whose key disappeared from the listing are dropped.
#[derive(Clone, Default)]
pub struct Cache(Arc<Mutex<HashMap<String, (Version, Bytes)>>>);

impl Cache {
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn hit(&self, key: &str, version: &Version) -> Option<Bytes> {
        let map = self.0.lock().unwrap_or_else(|e| e.into_inner());
        map.get(key)
            .filter(|(v, _)| v == version)
            .map(|(_, b)| b.clone())
    }

    fn store(&self, key: String, version: Version, bytes: Bytes) {
        let mut map = self.0.lock().unwrap_or_else(|e| e.into_inner());
        map.insert(key, (version, bytes));
    }

    /// Drop the entries of `scope` whose key is not in `keys` (other
    /// scopes untouched).
    fn retain(&self, scope: &str, keys: &[String]) {
        let mut map = self.0.lock().unwrap_or_else(|e| e.into_inner());
        map.retain(|k, _| !k.starts_with(&format!("{scope}|")) || keys.contains(k));
    }

    /// Cached files (tests check what was downloaded).
    pub fn len(&self) -> usize {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    async fn fetch(&self, storage: &Storage, scope: &str, o: &Object) -> Result<Bytes, AppError> {
        let cache_key = scoped_key(scope, &o.key);
        if let Some(bytes) = self.hit(&cache_key, &o.version) {
            return Ok(bytes);
        }
        // Cached under the listed version (a GET reports e.g. a coarser
        // Last-Modified): a file replaced between the listing and this read
        // no longer matches the next listing and is fetched again.
        let bytes = storage.get(&o.key).await?;
        self.store(cache_key, o.version.clone(), bytes.clone());
        Ok(bytes)
    }
}

fn scoped_key(scope: &str, key: &str) -> String {
    format!("{scope}|{key}")
}

/// A loaded design, ready to go into a render request.
pub struct Design {
    pub template: String,
    /// Every other file, keyed by relative path (the template and `qr.svg`
    /// excluded). Embedded files borrow the binary's static data.
    pub files: BTreeMap<String, Bytes>,
}

impl Design {
    /// Font keys for the request, sorted by path.
    pub fn fonts(&self) -> Vec<String> {
        self.files.keys().filter(|k| is_font(k)).cloned().collect()
    }

    /// `logo.svg`, else `logo.png`.
    pub fn logo(&self) -> Option<String> {
        ["logo.svg", "logo.png"]
            .into_iter()
            .find(|k| self.files.contains_key(*k))
            .map(str::to_string)
    }

    pub fn signature(&self) -> Option<String> {
        self.files
            .contains_key("signature.png")
            .then(|| "signature.png".to_string())
    }
}

fn too_large(path: &str, size: u64) -> Result<(), AppError> {
    if size > MAX_FILE_BYTES {
        return Err(AppError::PdfRenderFailed(format!(
            "design file {path} is larger than 10 MB"
        )));
    }
    Ok(())
}

/// Read the effective set. A file over 10 MB fails the render
/// (`pdf_render_failed`, checked before any download); an unreachable
/// storage is `storage_unavailable`.
pub async fn load(storage: &Storage, cache: &Cache, scope: &str) -> Result<Design, AppError> {
    let custom = custom(storage).await?;
    let keys: Vec<String> = custom.values().map(|o| scoped_key(scope, &o.key)).collect();
    cache.retain(scope, &keys);
    let mut files = BTreeMap::new();
    for (path, size) in embedded() {
        if path == QR_IMAGE || custom.contains_key(&path) {
            continue;
        }
        too_large(&path, size)?;
        let bytes = embedded_bytes(&path)?;
        files.insert(path, bytes);
    }
    for (path, o) in &custom {
        if path == QR_IMAGE {
            continue;
        }
        too_large(path, o.version.size)?;
        files.insert(path.clone(), cache.fetch(storage, scope, o).await?);
    }
    let template = files
        .remove(TEMPLATE)
        .context("the design has no invoice.typ")?;
    let template = String::from_utf8(template.to_vec())
        .map_err(|_| AppError::PdfRenderFailed(format!("{TEMPLATE} is not valid UTF-8")))?;
    Ok(Design { template, files })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn design_paths() {
        assert_eq!(design_path("design/logo.png"), Some("logo.png"));
        assert_eq!(design_path("design/fonts/a.ttf"), Some("fonts/a.ttf"));
        assert_eq!(design_path("design/.git/config"), None);
        assert_eq!(design_path("design/fonts/.DS_Store"), None);
        assert_eq!(design_path("designer/x"), None);
        assert_eq!(design_path("documents/2026/x.pdf"), None);
        assert_eq!(key("logo.svg"), "design/logo.svg");
    }

    #[test]
    fn default_design_is_embedded() {
        let files = embedded();
        assert!(files.contains_key(TEMPLATE));
        assert!(files.contains_key("fonts/OFL.txt"));
        let fonts: Vec<_> = files.keys().filter(|k| is_font(k)).collect();
        assert!(fonts.len() >= 3, "{fonts:?}");
        assert!(fonts.iter().all(|f| f.starts_with("fonts/Inter-")));
    }

    #[test]
    fn logo_prefers_svg() {
        let mut design = Design {
            template: String::new(),
            files: BTreeMap::new(),
        };
        assert_eq!((design.logo(), design.signature()), (None, None));
        design.files.insert("logo.png".into(), Bytes::new());
        assert_eq!(design.logo().as_deref(), Some("logo.png"));
        design.files.insert("logo.svg".into(), Bytes::new());
        design.files.insert("signature.png".into(), Bytes::new());
        assert_eq!(design.logo().as_deref(), Some("logo.svg"));
        assert_eq!(design.signature().as_deref(), Some("signature.png"));
    }

    #[test]
    fn size_limit() {
        assert!(too_large("a", MAX_FILE_BYTES).is_ok());
        assert!(matches!(
            too_large("logo.png", MAX_FILE_BYTES + 1),
            Err(AppError::PdfRenderFailed(m)) if m.contains("logo.png")
        ));
    }
}
