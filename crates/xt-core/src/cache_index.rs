//! Cache index — maps ESP file paths to their SHA-256 hashes via mtime+size.
//!
//! Avoids reading the entire file to compute SHA-256 on every load.
//! Stored as a small JSON file alongside the cache databases.
//!
//! Flow:
//! 1. Read cache_index.json (microseconds)
//! 2. Check path's (mtime, size) → get SHA-256
//! 3. Look up SQLite cache with SHA-256
//! 4. On miss: parse + hash simultaneously via HashingReader, then store in index

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read as _;
use std::path::Path;

/// Maps `path_key` → `CacheIndexEntry`
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CacheIndex {
    entries: HashMap<String, CacheIndexEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheIndexEntry {
    mtime: u64,
    size: u64,
    sha256: String,
}

impl CacheIndex {
    /// Load the index from disk (returns empty if file doesn't exist).
    pub fn load(dir: &Path) -> Self {
        let path = dir.join("cache_index.json");
        match std::fs::File::open(&path) {
            Ok(mut f) => {
                let mut buf = String::new();
                f.read_to_string(&mut buf).ok();
                serde_json::from_str(&buf).unwrap_or_default()
            }
            Err(_) => Self::default(),
        }
    }

    /// Save the index to disk.
    pub fn save(&self, dir: &Path) {
        let _ = std::fs::create_dir_all(dir);
        if let Ok(json) = serde_json::to_string(self) {
            let path = dir.join("cache_index.json");
            let _ = std::fs::write(&path, json);
        }
    }

    /// Look up SHA-256 for a file by its path + mtime + size.
    /// Returns None if the file metadata doesn't match.
    pub fn lookup(&self, file_path: &Path) -> Option<String> {
        let meta = file_path.metadata().ok()?;
        let key = file_path.to_string_lossy().to_string().to_lowercase();
        let entry = self.entries.get(&key)?;
        let mtime = meta.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
        let size = meta.len();
        if entry.mtime == mtime && entry.size == size {
            Some(entry.sha256.clone())
        } else {
            None
        }
    }

    /// Store a file's sha256 hash, keyed by path + mtime + size.
    pub fn store(&mut self, file_path: &Path, sha256: &str) {
        if let Ok(meta) = file_path.metadata() {
            let key = file_path.to_string_lossy().to_string().to_lowercase();
            let mtime = meta.modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            self.entries.insert(
                key,
                CacheIndexEntry {
                    mtime,
                    size: meta.len(),
                    sha256: sha256.to_string(),
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_store_and_lookup() {
        let dir = std::env::temp_dir().join(format!("xt_cache_index_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).ok();

        let test_file = dir.join("test.esp");
        let mut f = std::fs::File::create(&test_file).unwrap();
        f.write_all(b"test content").unwrap();
        f.sync_all().unwrap();

        let mut index = CacheIndex::default();
        index.store(&test_file, "abc123");

        // Should find it (mtime + size match)
        let found = index.lookup(&test_file);
        assert_eq!(found, Some("abc123".to_string()));

        // Save and reload
        index.save(&dir);
        let loaded = CacheIndex::load(&dir);
        let found2 = loaded.lookup(&test_file);
        assert_eq!(found2, Some("abc123".to_string()));

        // Non-existent file → None
        let missing = loaded.lookup(&dir.join("nonexistent.esp"));
        assert_eq!(missing, None);

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }
}
