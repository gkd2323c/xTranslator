//! 缓存索引 — 通过修改时间 mtime 和大小 size 将 ESP 文件路径映射到其 SHA-256 哈希值。
//!
//! 避免在每次加载时读取整个文件来计算 SHA-256。
//! 作为一个小型 JSON 文件与缓存数据库一起存储。
//!
//! 流程：
//! 1. 读取 cache_index.json（微秒级）
//! 2. 检查路径的 (mtime, size) → 获取 SHA-256
//! 3. 使用 SHA-256 查询 SQLite 缓存
//! 4. 未命中时：通过 HashingReader 同时进行解析与哈希计算，然后存储在索引中。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read as _;
use std::path::Path;

/// 将 `path_key` 映射到 `CacheIndexEntry`
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
    /// 从磁盘加载索引（如果文件不存在，则返回空索引）。
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

    /// 将索引保存到磁盘。
    pub fn save(&self, dir: &Path) {
        let _ = std::fs::create_dir_all(dir);
        if let Ok(json) = serde_json::to_string(self) {
            let path = dir.join("cache_index.json");
            let _ = std::fs::write(&path, json);
        }
    }

    /// 根据路径 + mtime + size 查找文件的 SHA-256 值。
    /// 如果文件的元数据不匹配，则返回 None。
    pub fn lookup(&self, file_path: &Path) -> Option<String> {
        let meta = file_path.metadata().ok()?;
        let key = file_path.to_string_lossy().to_string().to_lowercase();
        let entry = self.entries.get(&key)?;
        let mtime = meta
            .modified()
            .ok()?
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs();
        let size = meta.len();
        if entry.mtime == mtime && entry.size == size {
            Some(entry.sha256.clone())
        } else {
            None
        }
    }

    /// 存储文件的 SHA-256 哈希值，以路径 + mtime + size 作为键。
    pub fn store(&mut self, file_path: &Path, sha256: &str) {
        if let Ok(meta) = file_path.metadata() {
            let key = file_path.to_string_lossy().to_string().to_lowercase();
            let mtime = meta
                .modified()
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

        // 应该能找到它（mtime + size 匹配）
        let found = index.lookup(&test_file);
        assert_eq!(found, Some("abc123".to_string()));

        // 保存并重新加载
        index.save(&dir);
        let loaded = CacheIndex::load(&dir);
        let found2 = loaded.lookup(&test_file);
        assert_eq!(found2, Some("abc123".to_string()));

        // 不存在的文件 → None
        let missing = loaded.lookup(&dir.join("nonexistent.esp"));
        assert_eq!(missing, None);

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }
}
