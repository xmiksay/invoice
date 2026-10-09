//! Walking a local directory into storage keys — the CLI's source side
//! (`storage migrate`, `design push`). Blocking; run it off the runtime.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};

/// A regular file found under the walked root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalFile {
    /// `/`-separated path relative to the root.
    pub rel: String,
    pub path: PathBuf,
    pub size: u64,
}

/// Relative path (`/`-separated) of `path` under `root`; `None` for hidden
/// segments or non-UTF-8 names.
fn relative(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    let mut parts = Vec::new();
    for c in rel.components() {
        let s = c.as_os_str().to_str()?;
        if is_hidden(s) {
            return None;
        }
        parts.push(s);
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// A key's path below the directory `prefix` (`design/fonts/a.ttf` →
/// `fonts/a.ttf`): `None` outside it, or with an empty, `..` or hidden
/// segment — so the result is safe to join onto a local directory.
pub fn rel_key<'a>(key: &'a str, prefix: &str) -> Option<&'a str> {
    let rel = key.strip_prefix(prefix)?.strip_prefix('/')?;
    let safe = rel
        .split('/')
        .all(|s| !s.is_empty() && s != ".." && !is_hidden(s));
    safe.then_some(rel)
}

/// Dot-files and dot-directories (`.git`, editor swap files, the old archive's
/// `.{uuid}.tmp`) are never copied.
pub fn is_hidden(segment: &str) -> bool {
    segment.starts_with('.')
}

/// What a walk found: the files, and entries it could not read (an
/// unreadable directory such as `lost+found`, a dangling symlink) with why.
#[derive(Debug, Default)]
pub struct Walk {
    pub files: Vec<LocalFile>,
    pub skipped: Vec<String>,
}

/// Regular files under `dir` (recursively, symlinks followed), hidden ones
/// skipped, sorted by relative path. Each real directory is entered once,
/// so a symlink back into the tree (`shared -> .`) cannot loop. Only an
/// unreadable `dir` itself is an error; anything below it that cannot be
/// read is reported in [`Walk::skipped`] and the walk carries on.
pub fn walk(dir: &Path) -> Result<Walk> {
    let mut out = Walk::default();
    let mut visited = HashSet::new();
    fs::read_dir(dir).with_context(|| format!("read {}", dir.display()))?;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let mut skip = |path: &Path, e: &dyn std::fmt::Display| {
            out.skipped.push(format!("{}: {e}", path.display()));
        };
        let real = match fs::canonicalize(&current) {
            Ok(real) => real,
            Err(e) => {
                skip(&current, &e);
                continue;
            }
        };
        if !visited.insert(real) {
            continue;
        }
        let entries = match fs::read_dir(&current) {
            Ok(entries) => entries,
            Err(e) => {
                skip(&current, &e);
                continue;
            }
        };
        for entry in entries {
            let path = match entry {
                Ok(entry) => entry.path(),
                Err(e) => {
                    skip(&current, &e);
                    continue;
                }
            };
            let Some(rel) = relative(dir, &path) else {
                continue;
            };
            let meta = match fs::metadata(&path) {
                Ok(meta) => meta,
                Err(e) => {
                    skip(&path, &e);
                    continue;
                }
            };
            if meta.is_dir() {
                stack.push(path);
            } else if meta.is_file() {
                out.files.push(LocalFile {
                    rel,
                    path,
                    size: meta.len(),
                });
            }
        }
    }
    out.files.sort_by(|a, b| a.rel.cmp(&b.rel));
    out.skipped.sort();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_entries_are_skipped_and_output_sorted() {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path();
        fs::write(root.join("b.txt"), b"b").expect("write");
        fs::write(root.join(".hidden"), b"x").expect("write");
        fs::create_dir_all(root.join(".git")).expect("mkdir");
        fs::write(root.join(".git/config"), b"x").expect("write");
        fs::create_dir_all(root.join("a/deep")).expect("mkdir");
        fs::write(root.join("a/deep/c.bin"), b"ccc").expect("write");
        let rels: Vec<(String, u64)> = walk(root)
            .expect("walk")
            .files
            .into_iter()
            .map(|f| (f.rel, f.size))
            .collect();
        assert_eq!(rels, [("a/deep/c.bin".into(), 3), ("b.txt".into(), 1)]);
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
        let rels: Vec<String> = walk(root)
            .expect("walk terminates")
            .files
            .into_iter()
            .map(|f| f.rel)
            .collect();
        assert!(rels.contains(&"fonts/Alias.ttf".to_string()), "{rels:?}");
        let logos = rels.iter().filter(|p| p.ends_with("logo.png")).count();
        assert_eq!(logos, 1, "{rels:?}");
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_entries_are_skipped_and_reported() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path();
        fs::write(root.join("a.txt"), b"a").expect("write");
        std::os::unix::fs::symlink(root.join("gone"), root.join("dangling")).expect("symlink");
        let locked = root.join("lost+found");
        fs::create_dir(&locked).expect("mkdir");
        fs::write(locked.join("x"), b"x").expect("write");
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).expect("chmod");
        // root ignores permissions; then only the dangling link is skipped.
        let readable = fs::read_dir(&locked).is_ok();
        let walked = walk(root);
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).expect("chmod back");
        let walked = walked.expect("the walk carries on");
        let rels: Vec<&str> = walked.files.iter().map(|f| f.rel.as_str()).collect();
        assert!(rels.contains(&"a.txt"), "{rels:?}");
        assert!(
            walked.skipped.iter().any(|s| s.contains("dangling")),
            "{:?}",
            walked.skipped
        );
        if !readable {
            assert!(
                walked.skipped.iter().any(|s| s.contains("lost+found")),
                "{:?}",
                walked.skipped
            );
        }
        assert!(walk(&root.join("missing")).is_err());
    }

    #[test]
    fn rel_keys_stay_inside_the_directory() {
        assert_eq!(rel_key("design/fonts/a.ttf", "design"), Some("fonts/a.ttf"));
        for bad in [
            "design/../etc/passwd",
            "design/.git/config",
            "design/fonts/.DS_Store",
            "design//x",
            "designer/x",
            "design",
            "design/",
        ] {
            assert_eq!(rel_key(bad, "design"), None, "{bad}");
        }
    }
}
