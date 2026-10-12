//! File integrity: hashing, folder scans and baseline comparison.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use chrono::{DateTime, Utc};
use globset::{Glob, GlobSet, GlobSetBuilder};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{CoreError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum HashAlgorithm {
    #[default]
    Sha256,
    Blake3,
}

impl HashAlgorithm {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sha256 => "sha256",
            Self::Blake3 => "blake3",
        }
    }
}

impl std::str::FromStr for HashAlgorithm {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "sha256" | "sha-256" => Ok(Self::Sha256),
            "blake3" => Ok(Self::Blake3),
            _ => Err(CoreError::Invalid(format!("unknown hash algorithm: {s}"))),
        }
    }
}

enum Hasher {
    Sha256(Box<Sha256>),
    Blake3(Box<blake3::Hasher>),
}

impl Hasher {
    fn new(algorithm: HashAlgorithm) -> Self {
        match algorithm {
            HashAlgorithm::Sha256 => Self::Sha256(Box::default()),
            HashAlgorithm::Blake3 => Self::Blake3(Box::default()),
        }
    }

    fn update(&mut self, data: &[u8]) {
        match self {
            Self::Sha256(h) => h.update(data),
            Self::Blake3(h) => {
                h.update(data);
            }
        }
    }

    fn finish(self) -> String {
        match self {
            Self::Sha256(h) => hex::encode(h.finalize()),
            Self::Blake3(h) => h.finalize().to_hex().to_string(),
        }
    }
}

/// Hashes a file, streaming it in 1 MiB chunks. Returns the lowercase hex digest.
pub fn hash_file(path: impl AsRef<Path>, algorithm: HashAlgorithm) -> Result<String> {
    let mut reader = BufReader::with_capacity(1 << 20, File::open(path)?);
    let mut hasher = Hasher::new(algorithm);
    let mut buffer = vec![0u8; 1 << 20];
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(hasher.finish())
}

/// Compares a computed digest with a user supplied one (case/space insensitive).
pub fn hashes_match(actual: &str, expected: &str) -> bool {
    let normalize = |s: &str| s.trim().to_ascii_lowercase().replace([' ', ':'], "");
    !expected.trim().is_empty() && normalize(actual) == normalize(expected)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    /// Path relative to the scanned root, `/` separated.
    pub path: String,
    pub size: u64,
    pub modified_at: Option<DateTime<Utc>>,
    pub hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanOptions {
    pub algorithm: HashAlgorithm,
    /// Glob patterns (relative to the root) to skip, e.g. `**/node_modules/**`, `*.log`.
    pub excludes: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanReport {
    pub entries: Vec<FileEntry>,
    /// Files that could not be read, with the reason.
    pub errors: Vec<(String, String)>,
    pub cancelled: bool,
}

/// Builds the matcher for exclude patterns (paths relative to the root).
pub fn exclude_matcher(patterns: &[String]) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns.iter().map(|p| p.trim()).filter(|p| !p.is_empty()) {
        let glob = Glob::new(pattern)
            .map_err(|err| CoreError::Invalid(format!("exclude {pattern:?}: {err}")))?;
        builder.add(glob);
        // `node_modules` style patterns also match anywhere in the tree.
        if !pattern.contains('/') {
            let nested = format!("**/{pattern}");
            builder.add(Glob::new(&nested).map_err(|err| CoreError::Invalid(err.to_string()))?);
            builder.add(
                Glob::new(&format!("{nested}/**"))
                    .map_err(|err| CoreError::Invalid(err.to_string()))?,
            );
        }
    }
    builder
        .build()
        .map_err(|err| CoreError::Invalid(err.to_string()))
}

/// `path` relative to `root`, `/` separated.
pub fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// Lists the files under `root` (or `root` itself if it is a file), honoring excludes.
pub fn list_files(root: &Path, excludes: &[String]) -> Result<Vec<PathBuf>> {
    if !root.exists() {
        return Err(CoreError::NotFound(root.display().to_string()));
    }
    if root.is_file() {
        return Ok(vec![root.to_path_buf()]);
    }
    let excludes = exclude_matcher(excludes)?;
    let mut files: Vec<PathBuf> = walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            entry.depth() == 0 || !excludes.is_match(relative(root, entry.path()))
        })
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .collect();
    files.sort();
    Ok(files)
}

/// Hashes every file under `root` in parallel.
///
/// `on_progress(done, total)` is called from worker threads; `cancel` stops
/// the scan early (the report is then marked `cancelled`).
pub fn scan(
    root: &Path,
    options: &ScanOptions,
    cancel: &AtomicBool,
    on_progress: impl Fn(usize, usize) + Sync,
) -> Result<ScanReport> {
    let files = list_files(root, &options.excludes)?;
    let base = if root.is_file() {
        root.parent().unwrap_or(root)
    } else {
        root
    };
    let total = files.len();
    let done = AtomicUsize::new(0);
    on_progress(0, total);

    let results: Vec<std::result::Result<FileEntry, (String, String)>> = files
        .par_iter()
        .filter_map(|path| {
            if cancel.load(Ordering::Relaxed) {
                return None;
            }
            let rel = relative(base, path);
            let result = std::fs::metadata(path)
                .and_then(|meta| {
                    let hash = hash_file(path, options.algorithm).map_err(std::io::Error::other)?;
                    Ok(FileEntry {
                        path: rel.clone(),
                        size: meta.len(),
                        modified_at: meta.modified().ok().map(DateTime::<Utc>::from),
                        hash,
                    })
                })
                .map_err(|err| (rel, err.to_string()));
            on_progress(done.fetch_add(1, Ordering::Relaxed) + 1, total);
            Some(result)
        })
        .collect();

    let mut report = ScanReport {
        cancelled: cancel.load(Ordering::Relaxed),
        ..Default::default()
    };
    for result in results {
        match result {
            Ok(entry) => report.entries.push(entry),
            Err(error) => report.errors.push(error),
        }
    }
    report.entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(report)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModifiedFile {
    pub path: String,
    pub old_hash: String,
    pub new_hash: String,
    pub old_size: u64,
    pub new_size: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Diff {
    pub added: Vec<FileEntry>,
    pub removed: Vec<FileEntry>,
    pub modified: Vec<ModifiedFile>,
    pub unchanged: usize,
}

impl Diff {
    pub fn is_clean(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.modified.is_empty()
    }
}

/// Compares a baseline with a fresh scan.
pub fn compare(baseline: &[FileEntry], current: &[FileEntry]) -> Diff {
    let old: BTreeMap<&str, &FileEntry> = baseline.iter().map(|e| (e.path.as_str(), e)).collect();
    let new: BTreeMap<&str, &FileEntry> = current.iter().map(|e| (e.path.as_str(), e)).collect();
    let mut diff = Diff::default();
    for (path, entry) in &new {
        match old.get(path) {
            None => diff.added.push((*entry).clone()),
            Some(previous) if previous.hash != entry.hash => diff.modified.push(ModifiedFile {
                path: path.to_string(),
                old_hash: previous.hash.clone(),
                new_hash: entry.hash.clone(),
                old_size: previous.size,
                new_size: entry.size,
            }),
            Some(_) => diff.unchanged += 1,
        }
    }
    diff.removed = old
        .iter()
        .filter(|(path, _)| !new.contains_key(*path))
        .map(|(_, entry)| (*entry).clone())
        .collect();
    diff
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_digests() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("abc.txt");
        std::fs::write(&path, "abc").unwrap();
        assert_eq!(
            hash_file(&path, HashAlgorithm::Sha256).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            hash_file(&path, HashAlgorithm::Blake3).unwrap(),
            "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85"
        );
        assert!(hashes_match("BA78 16bf", " ba7816BF "));
        assert!(!hashes_match("ba78", ""));
    }

    fn write(root: &Path, rel: &str, content: &str) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    #[test]
    fn scans_with_excludes_and_compares() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "a.txt", "one");
        write(root, "sub/b.txt", "two");
        write(root, "sub/debug.log", "log");
        write(root, "node_modules/x/index.js", "js");

        let options = ScanOptions {
            algorithm: HashAlgorithm::Blake3,
            excludes: vec!["*.log".into(), "node_modules".into()],
        };
        let calls = AtomicUsize::new(0);
        let before = scan(root, &options, &AtomicBool::new(false), |_, total| {
            assert_eq!(total, 2);
            calls.fetch_add(1, Ordering::SeqCst);
        })
        .unwrap();
        let paths: Vec<_> = before.entries.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(paths, ["a.txt", "sub/b.txt"]);
        assert_eq!(calls.load(Ordering::SeqCst), 3);

        write(root, "a.txt", "changed");
        std::fs::remove_file(root.join("sub/b.txt")).unwrap();
        write(root, "c.txt", "new");
        let after = scan(root, &options, &AtomicBool::new(false), |_, _| {}).unwrap();

        let diff = compare(&before.entries, &after.entries);
        assert_eq!(
            diff.added.iter().map(|e| &e.path).collect::<Vec<_>>(),
            ["c.txt"]
        );
        assert_eq!(
            diff.removed.iter().map(|e| &e.path).collect::<Vec<_>>(),
            ["sub/b.txt"]
        );
        assert_eq!(diff.modified.len(), 1);
        assert_eq!(diff.modified[0].new_size, 7);
        assert_eq!(diff.unchanged, 0);
        assert!(!diff.is_clean());
        assert!(compare(&after.entries, &after.entries).is_clean());
    }

    #[test]
    fn single_file_root_and_cancel() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "only.bin", "x");
        let report = scan(
            &dir.path().join("only.bin"),
            &ScanOptions::default(),
            &AtomicBool::new(false),
            |_, _| {},
        )
        .unwrap();
        assert_eq!(report.entries[0].path, "only.bin");

        let cancelled = scan(
            dir.path(),
            &ScanOptions::default(),
            &AtomicBool::new(true),
            |_, _| {},
        )
        .unwrap();
        assert!(cancelled.cancelled);
        assert!(cancelled.entries.is_empty());

        assert!(matches!(
            scan(
                &dir.path().join("missing"),
                &ScanOptions::default(),
                &AtomicBool::new(false),
                |_, _| {}
            ),
            Err(CoreError::NotFound(_))
        ));
    }
}
