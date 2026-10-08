//! The design file set: the default design embedded from `design/`,
//! overridden file by file by `INVOICE__DESIGN_DIR` (re-read on every render).

use std::borrow::Cow;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context as _;
use bytes::Bytes;
use rust_embed::Embed;
use serde::Serialize;
use utoipa::ToSchema;

use crate::error::AppError;

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

/// A file of the effective set: embedded bytes or a path in the design dir.
enum Origin {
    Embedded,
    Dir(PathBuf),
}

/// Relative path (`/`-separated) of `path` under `root`; `None` for hidden
/// segments or non-UTF-8 names.
fn relative(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    let mut parts = Vec::new();
    for c in rel.components() {
        let s = c.as_os_str().to_str()?;
        if s.starts_with('.') {
            return None;
        }
        parts.push(s);
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// Regular files under `dir` (recursively, symlinks followed), hidden ones
/// skipped. Each real directory is entered once, so a symlink back into the
/// tree (`shared -> .`) cannot loop.
fn walk(dir: &Path) -> anyhow::Result<Vec<(String, PathBuf, u64)>> {
    let mut out = Vec::new();
    let mut visited = HashSet::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let real =
            fs::canonicalize(&current).with_context(|| format!("resolve {}", current.display()))?;
        if !visited.insert(real) {
            continue;
        }
        let entries =
            fs::read_dir(&current).with_context(|| format!("read {}", current.display()))?;
        for entry in entries {
            let path = entry
                .with_context(|| format!("read {}", current.display()))?
                .path();
            let Some(rel) = relative(dir, &path) else {
                continue;
            };
            let meta = fs::metadata(&path).with_context(|| format!("stat {}", path.display()))?;
            if meta.is_dir() {
                stack.push(path);
            } else if meta.is_file() {
                out.push((rel, path, meta.len()));
            }
        }
    }
    Ok(out)
}

fn effective(dir: Option<&Path>) -> anyhow::Result<BTreeMap<String, (Origin, u64)>> {
    let mut files: BTreeMap<String, (Origin, u64)> = DefaultDesign::iter()
        .filter(|p| !p.split('/').any(|s| s.starts_with('.')))
        .filter_map(|p| {
            let size = DefaultDesign::get(&p)?.data.len() as u64;
            Some((p.into_owned(), (Origin::Embedded, size)))
        })
        .collect();
    if let Some(dir) = dir {
        for (rel, path, size) in walk(dir)? {
            files.insert(rel, (Origin::Dir(path), size));
        }
    }
    Ok(files)
}

/// The effective set for `GET /api/pdf/design`, sorted by path.
pub fn list(dir: Option<&Path>) -> anyhow::Result<Vec<DesignFile>> {
    Ok(effective(dir)?
        .into_iter()
        .map(|(path, (origin, size))| DesignFile {
            path,
            source: match origin {
                Origin::Embedded => Source::Default,
                Origin::Dir(_) => Source::Custom,
            },
            size,
        })
        .collect())
}

fn is_font(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.starts_with("fonts/") && (lower.ends_with(".ttf") || lower.ends_with(".otf"))
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

/// Read the effective set. A file over 10 MB fails the render
/// (`pdf_render_failed`); unreadable files are internal errors.
pub fn load(dir: Option<&Path>) -> Result<Design, AppError> {
    let mut template = None;
    let mut files = BTreeMap::new();
    for (path, (origin, size)) in effective(dir)? {
        if path == QR_IMAGE {
            continue;
        }
        if size > MAX_FILE_BYTES {
            return Err(AppError::PdfRenderFailed(format!(
                "design file {path} is larger than 10 MB"
            )));
        }
        let bytes = match origin {
            Origin::Embedded => {
                match DefaultDesign::get(&path)
                    .with_context(|| format!("embedded design file {path} vanished"))?
                    .data
                {
                    Cow::Borrowed(b) => Bytes::from_static(b),
                    // Debug builds read the embedded folder from disk.
                    Cow::Owned(v) => Bytes::from(v),
                }
            }
            Origin::Dir(p) => {
                Bytes::from(fs::read(&p).with_context(|| format!("read {}", p.display()))?)
            }
        };
        if path == TEMPLATE {
            template = Some(String::from_utf8(bytes.to_vec()).map_err(|_| {
                AppError::PdfRenderFailed(format!("{TEMPLATE} is not valid UTF-8"))
            })?);
        } else {
            files.insert(path, bytes);
        }
    }
    let template = template.context("the design has no invoice.typ")?;
    Ok(Design { template, files })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_design_is_embedded() {
        let design = load(None).expect("default design");
        assert!(design.template.contains("/data.json"));
        let fonts = design.fonts();
        assert!(fonts.len() >= 3, "{fonts:?}");
        assert!(fonts.iter().all(|f| f.starts_with("fonts/Inter-")));
        let mut sorted = fonts.clone();
        sorted.sort();
        assert_eq!(fonts, sorted);
        assert!(design.files.contains_key("fonts/OFL.txt"));
        assert_eq!(design.logo(), None);
        assert_eq!(design.signature(), None);
        let listed = list(None).expect("list");
        assert!(listed.iter().all(|f| f.source == Source::Default));
        assert!(listed.iter().any(|f| f.path == TEMPLATE));
    }

    #[test]
    fn dir_overrides_file_by_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path();
        fs::write(root.join("invoice.typ"), "custom /data.json").expect("write");
        fs::write(root.join("logo.png"), b"png").expect("write");
        fs::write(root.join("logo.svg"), b"<svg/>").expect("write");
        fs::write(root.join("signature.png"), b"sig").expect("write");
        fs::write(root.join("qr.svg"), b"reserved").expect("write");
        fs::write(root.join(".hidden"), b"x").expect("write");
        fs::create_dir_all(root.join(".git")).expect("mkdir");
        fs::write(root.join(".git/config"), b"x").expect("write");
        fs::create_dir_all(root.join("fonts/extra")).expect("mkdir");
        fs::write(root.join("fonts/extra/Brand.OTF"), b"otf").expect("write");
        fs::write(root.join("parts.typ"), b"#let x = 1").expect("write");

        let design = load(Some(root)).expect("design");
        assert_eq!(design.template, "custom /data.json");
        assert_eq!(design.logo().as_deref(), Some("logo.svg"));
        assert_eq!(design.signature().as_deref(), Some("signature.png"));
        assert!(!design.files.contains_key("qr.svg"));
        assert!(
            !design
                .files
                .keys()
                .any(|k| k.contains(".git") || k == ".hidden")
        );
        assert!(
            design
                .fonts()
                .contains(&"fonts/extra/Brand.OTF".to_string())
        );
        assert!(design.files.contains_key("parts.typ"));

        let listed = list(Some(root)).expect("list");
        let find = |p: &str| listed.iter().find(|f| f.path == p).map(|f| f.source);
        assert_eq!(find("invoice.typ"), Some(Source::Custom));
        assert_eq!(find("fonts/OFL.txt"), Some(Source::Default));
        assert_eq!(find(".hidden"), None);
        let paths: Vec<&str> = listed.iter().map(|f| f.path.as_str()).collect();
        let mut sorted = paths.clone();
        sorted.sort();
        assert_eq!(paths, sorted);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_cycles_are_entered_once() {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path();
        fs::write(root.join("logo.png"), b"png").expect("write");
        fs::create_dir_all(root.join("fonts")).expect("mkdir");
        fs::write(root.join("fonts/Brand.ttf"), b"ttf").expect("write");
        std::os::unix::fs::symlink(".", root.join("shared")).expect("symlink");
        std::os::unix::fs::symlink(root.join("fonts/Brand.ttf"), root.join("fonts/Alias.ttf"))
            .expect("file symlink");
        let listed = list(Some(root)).expect("list terminates");
        let custom: Vec<&str> = listed
            .iter()
            .filter(|f| f.source == Source::Custom)
            .map(|f| f.path.as_str())
            .collect();
        assert!(custom.contains(&"logo.png"), "{custom:?}");
        assert!(
            custom.contains(&"fonts/Alias.ttf"),
            "file symlinks are followed"
        );
        let logos = custom.iter().filter(|p| p.ends_with("logo.png")).count();
        assert_eq!(logos, 1, "{custom:?}");
    }

    #[test]
    fn oversized_file_fails_the_render() {
        let dir = tempfile::tempdir().expect("temp dir");
        let big = fs::File::create(dir.path().join("logo.png")).expect("create");
        big.set_len(MAX_FILE_BYTES + 1).expect("grow");
        assert!(matches!(
            load(Some(dir.path())),
            Err(AppError::PdfRenderFailed(m)) if m.contains("logo.png")
        ));
    }
}
