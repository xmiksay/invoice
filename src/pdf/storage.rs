//! Archived PDFs on disk under `INVOICE__STORAGE_DIR`. Blocking file I/O —
//! callers run these through `spawn_blocking`.

use std::fs::{self, File};
use std::io::Write as _;
use std::path::Path;

use anyhow::{Context as _, Result};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// `documents/{numberYear}/{id}.pdf`, relative to the storage root.
pub fn relative_path(number_year: i32, id: Uuid) -> String {
    format!("documents/{number_year}/{id}.pdf")
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Write `bytes` to `root/rel` atomically: a temp file in the same directory,
/// fsync, rename, fsync the directory. Readers never see a partial file.
pub fn write_atomic(root: &Path, rel: &str, bytes: &[u8]) -> Result<()> {
    let path = root.join(rel);
    let dir = path
        .parent()
        .with_context(|| format!("archive path {rel} has no directory"))?;
    fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    let tmp = dir.join(format!(".{}.tmp", Uuid::new_v4()));
    let written = (|| -> Result<()> {
        let mut f = File::create(&tmp).with_context(|| format!("create {}", tmp.display()))?;
        f.write_all(bytes)
            .with_context(|| format!("write {}", tmp.display()))?;
        f.sync_all()
            .with_context(|| format!("fsync {}", tmp.display()))?;
        fs::rename(&tmp, &path).with_context(|| format!("rename to {}", path.display()))
    })();
    if written.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    written?;
    // The rename is durable only once the directory entry is flushed.
    File::open(dir)
        .and_then(|d| d.sync_all())
        .with_context(|| format!("fsync {}", dir.display()))
}

pub fn read(root: &Path, rel: &str) -> Result<Vec<u8>> {
    let path = root.join(rel);
    fs::read(&path).with_context(|| format!("read archived PDF {}", path.display()))
}

/// Best effort: the file of a transaction that did not commit.
pub fn remove(root: &Path, rel: &str) {
    let path = root.join(rel);
    if let Err(e) = fs::remove_file(&path) {
        tracing::warn!(path = %path.display(), error = %e, "remove orphaned PDF");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_and_hash() {
        let id = Uuid::nil();
        assert_eq!(
            relative_path(2026, id),
            "documents/2026/00000000-0000-0000-0000-000000000000.pdf"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn write_read_overwrite_remove() {
        let root = tempfile::tempdir().expect("temp dir");
        let rel = relative_path(2026, Uuid::new_v4());
        write_atomic(root.path(), &rel, b"%PDF-1").expect("write");
        write_atomic(root.path(), &rel, b"%PDF-2").expect("overwrite");
        assert_eq!(read(root.path(), &rel).expect("read"), b"%PDF-2");
        let dir = root.path().join("documents/2026");
        let names: Vec<_> = fs::read_dir(&dir)
            .expect("list")
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(names.len(), 1, "no temp files left behind");
        remove(root.path(), &rel);
        assert!(read(root.path(), &rel).is_err());
    }
}
