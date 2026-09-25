//! Content-addressed cache.
//!
//! The point of the layers: the diff is the only thing that must be fresh.
//! Project facts almost never change, the commit-style profile only changes
//! when the log window moves, and per-file summaries are keyed by content — so
//! a second run over the same staged tree costs nothing.
//!
//! Keys are always namespaced per repo, so one cache directory serves every
//! repository on the machine.

use std::cell::Cell;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::{Error, Result};
use crate::paths;

#[cfg(test)]
#[path = "cache_tests.rs"]
mod cache_tests;

pub struct Cache {
    root: PathBuf,
    repo_key: String,
    enabled: bool,
    hits: Cell<usize>,
    misses: Cell<usize>,
}

impl Cache {
    pub fn new(repo_key: &str, enabled: bool) -> Self {
        Self {
            root: paths::cache_dir(),
            repo_key: paths::short_hash(repo_key),
            enabled,
            hits: Cell::new(0),
            misses: Cell::new(0),
        }
    }

    pub fn repo_dir(&self) -> PathBuf {
        self.root.join(&self.repo_key)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn hits(&self) -> usize {
        self.hits.get()
    }

    pub fn misses(&self) -> usize {
        self.misses.get()
    }

    /// `ns/key` -> a path inside the repo's cache directory.
    fn entry_path(&self, namespace: &str, key: &str) -> PathBuf {
        self.repo_dir()
            .join(namespace)
            .join(format!("{}.json", paths::short_hash(key)))
    }

    pub fn load<T: DeserializeOwned>(&self, namespace: &str, key: &str) -> Option<T> {
        if !self.enabled {
            return None;
        }
        let path = self.entry_path(namespace, key);
        let text = std::fs::read_to_string(&path).ok()?;
        match serde_json::from_str::<T>(&text) {
            Ok(value) => {
                self.hits.set(self.hits.get() + 1);
                Some(value)
            }
            Err(_) => {
                // Corrupt, or written by an older shape: drop and recompute.
                let _ = std::fs::remove_file(&path);
                None
            }
        }
    }

    pub fn store<T: Serialize>(&self, namespace: &str, key: &str, value: &T) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }
        let path = self.entry_path(namespace, key);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
        let text = serde_json::to_string(value)?;
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, text).map_err(|e| Error::io(&tmp, e))?;
        std::fs::rename(&tmp, &path).map_err(|e| Error::io(&path, e))?;
        Ok(())
    }

    /// Cached read, or compute-and-store.
    pub fn get_or_compute<T, F>(&self, namespace: &str, key: &str, compute: F) -> Result<T>
    where
        T: Serialize + DeserializeOwned,
        F: FnOnce() -> Result<T>,
    {
        if let Some(hit) = self.load(namespace, key) {
            return Ok(hit);
        }
        self.misses.set(self.misses.get() + 1);
        let value = compute()?;
        self.store(namespace, key, &value)?;
        Ok(value)
    }

    /// (namespace, entries, bytes) for `wyckoff cache stat`.
    pub fn stats(&self) -> Vec<(String, usize, u64)> {
        let mut out = Vec::new();
        let Ok(entries) = std::fs::read_dir(self.repo_dir()) else {
            return out;
        };
        for entry in entries.flatten() {
            if !entry.path().is_dir() {
                continue;
            }
            let mut count = 0;
            let mut bytes = 0;
            if let Ok(files) = std::fs::read_dir(entry.path()) {
                for file in files.flatten() {
                    if let Ok(meta) = file.metadata() {
                        count += 1;
                        bytes += meta.len();
                    }
                }
            }
            out.push((
                entry.file_name().to_string_lossy().to_string(),
                count,
                bytes,
            ));
        }
        out.sort();
        out
    }

    /// Remove this repo's entries. Returns (entries, bytes).
    pub fn clean(&self) -> Result<(usize, u64)> {
        let dir = self.repo_dir();
        if !dir.exists() {
            return Ok((0, 0));
        }
        let mut entries = 0;
        let mut bytes = 0;
        let namespaces = std::fs::read_dir(&dir).map_err(|e| Error::io(&dir, e))?;
        for namespace in namespaces.flatten() {
            if let Ok(files) = std::fs::read_dir(namespace.path()) {
                for file in files.flatten() {
                    if let Ok(meta) = file.metadata() {
                        entries += 1;
                        bytes += meta.len();
                    }
                }
            }
            std::fs::remove_dir_all(namespace.path())
                .map_err(|e| Error::io(namespace.path(), e))?;
        }
        Ok((entries, bytes))
    }
}
