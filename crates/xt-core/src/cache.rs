//! ESP/ESM 解析结果缓存
//!
//! 将 ESP 解析得到的 `Vec<SkyString>` 缓存到本地二进制文件，
//! 避免每次启动应用时重新解析大型 ESM 文件。
//!
//! 缓存策略：
//! - 密钥：ESP 文件的 SHA-256 哈希（内容寻址）
//! - 数据：bincode 序列化的 [`CachePayload`]
//! - 失效：ESP 文件内容变化 → 哈希不匹配 → 自动重新解析
//! - 清理：`prune()` 移除超过 max_entries 的旧缓存

use crate::types::sky_string::SkyString;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

const CACHE_VERSION: u32 = 1;
const FILE_EXT: &str = "cache";

/// 缓存文件载荷
#[derive(Serialize, Deserialize)]
pub struct CachePayload {
    pub version: u32,
    pub strings: Vec<SkyString>,
    pub compressed_records: u32,
    pub strings_loaded: u8,
}

/// ESP 解析结果缓存管理器
pub struct EsmCache {
    cache_dir: PathBuf,
    max_entries: usize,
}

impl EsmCache {
    /// 创建新的缓存管理器
    ///
    /// `cache_dir` 不存在时会自动创建。
    /// `max_entries` 控制 `prune()` 行为。
    pub fn new(cache_dir: PathBuf, max_entries: usize) -> Self {
        Self {
            cache_dir,
            max_entries,
        }
    }

    /// 根据 ESP 文件路径查找缓存
    ///
    /// 返回 `None` 表示缓存未命中（文件不存在 / 版本不匹配 / 反序列化失败）。
    /// 缓存命中时返回完整的解析结果载荷。
    pub fn lookup(&self, esp_path: &Path) -> Option<CachePayload> {
        let hash = hash_file(esp_path).ok()?;
        self.lookup_by_hash(&hash)
    }

    /// 使用预计算哈希查找缓存（避免重复 SHA-256 计算）
    pub fn lookup_by_hash(&self, hash: &str) -> Option<CachePayload> {
        let cache_path = self.cache_path(hash);

        if !cache_path.exists() {
            return None;
        }

        // 尝试触摸缓存文件（更新访问时间为解析替用时间）
        let _ = std::fs::File::open(&cache_path)
            .and_then(|f| f.set_modified(std::time::SystemTime::now()));

        let data = std::fs::read(&cache_path).ok()?;
        let payload: CachePayload = bincode::deserialize(&data).ok()?;

        if payload.version != CACHE_VERSION {
            // 版本不匹配 → 删除旧缓存
            let _ = std::fs::remove_file(&cache_path);
            return None;
        }

        Some(payload)
    }

    /// 存储 ESP 解析结果到缓存
    ///
    /// 若已存在同名缓存文件则覆盖。
    pub fn store(&self, esp_path: &Path, payload: &CachePayload) -> std::io::Result<()> {
        let hash = hash_file(esp_path)?;
        self.store_with_hash(&hash, payload)
    }

    /// 使用预计算哈希存储 ESP 解析结果（跳过重复 SHA-256 计算）
    pub fn store_with_hash(&self, hash: &str, payload: &CachePayload) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.cache_dir)?;

        let cache_path = self.cache_path(hash);

        let data = bincode::serialize(payload)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        std::fs::write(&cache_path, &data)?;

        // 存储后触发清理
        let _ = self.prune();

        Ok(())
    }

    /// 根据缓存字符串计算 record_counts
    pub fn compute_record_counts(strings: &[SkyString]) -> HashMap<String, usize> {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for sk in strings {
            let sig = String::from_utf8_lossy(&sk.esp_ptr.record_sig).to_string();
            *counts.entry(sig).or_insert(0) += 1;
        }
        counts
    }

    /// 移除超出最大容量的最旧缓存文件
    pub fn prune(&self) -> std::io::Result<()> {
        if !self.cache_dir.exists() {
            return Ok(());
        }

        let mut entries: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
        for entry in std::fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some(FILE_EXT) {
                continue;
            }
            let modified = entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .unwrap_or(std::time::UNIX_EPOCH);
            entries.push((modified, path));
        }

        if entries.len() <= self.max_entries {
            return Ok(());
        }

        // 按修改时间升序排列（最旧的在前）
        entries.sort_by_key(|(t, _)| *t);

        // 删除超出上限的旧文件
        let to_remove = entries.len() - self.max_entries;
        for (_, path) in entries.iter().take(to_remove) {
            let _ = std::fs::remove_file(path);
        }

        Ok(())
    }

    fn cache_path(&self, hash: &str) -> PathBuf {
        self.cache_dir.join(format!("{}.{}", hash, FILE_EXT))
    }
}

/// 计算文件的 SHA-256 哈希（流式读取，避免全量加载）
pub fn hash_file(path: &Path) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::esp_pointer::EspPointer;
    use crate::types::params::SkyStringParams;
    use std::io::Write;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("xt_cache_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).ok();
        dir
    }

    fn temp_file(name: &str, content: &[u8]) -> PathBuf {
        let dir = temp_dir();
        let path = dir.join(name);
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content).unwrap();
        path
    }

    fn make_sk(id: u32, source: &str, rec: [u8; 4], field: [u8; 4]) -> SkyString {
        let mut sk = SkyString::new(id, source.to_string(), String::new(), rec, field);
        sk.esp_ptr = EspPointer {
            str_id: id as i32,
            form_id: 0,
            record_sig: rec,
            field_sig: field,
            index: 0,
            index_max: 0,
            edid_hash: 0,
        };
        sk.params.set(SkyStringParams::INCOMPLETE_TRANS, true);
        sk
    }

    #[test]
    fn test_hash_file_consistent() {
        let path = temp_file("hash_test.bin", b"hello world");
        let hash1 = hash_file(&path).unwrap();
        let hash2 = hash_file(&path).unwrap();
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64);
    }

    #[test]
    fn test_cache_store_and_lookup() {
        let cache_dir = temp_dir().join("cache");
        let cache = EsmCache::new(cache_dir.clone(), 10);

        let esp_path = temp_file("test.esp", b"dummy esp content");
        let strings = vec![
            make_sk(0, "Hello", *b"LCTN", *b"FULL"),
            make_sk(1, "World", *b"QUST", *b"NNAM"),
        ];

        let payload = CachePayload {
            version: CACHE_VERSION,
            strings: strings.clone(),
            compressed_records: 0,
            strings_loaded: 3,
        };

        // Store
        cache.store(&esp_path, &payload).unwrap();
        assert!(cache_dir.exists());

        // Lookup
        let cached = cache.lookup(&esp_path).unwrap();
        assert_eq!(cached.version, CACHE_VERSION);
        assert_eq!(cached.strings.len(), 2);
        assert_eq!(cached.strings[0].source, "Hello");
        assert_eq!(cached.strings[1].source, "World");
        assert_eq!(cached.compressed_records, 0);
        assert_eq!(cached.strings_loaded, 3);
    }

    #[test]
    fn test_cache_miss_different_file() {
        let cache_dir = temp_dir().join("cache2");
        let cache = EsmCache::new(cache_dir.clone(), 10);

        let esp_path1 = temp_file("a.esp", b"content A");
        let esp_path2 = temp_file("b.esp", b"content B"); // different content

        let payload = CachePayload {
            version: CACHE_VERSION,
            strings: vec![make_sk(0, "Test", *b"LCTN", *b"FULL")],
            compressed_records: 0,
            strings_loaded: 1,
        };

        cache.store(&esp_path1, &payload).unwrap();

        // Different file → cache miss
        assert!(cache.lookup(&esp_path2).is_none());
    }

    #[test]
    fn test_cache_miss_modified_file() {
        let cache_dir = temp_dir().join("cache3");
        let cache = EsmCache::new(cache_dir.clone(), 10);

        let esp_path = temp_file("mod.esp", b"original content");
        let payload = CachePayload {
            version: CACHE_VERSION,
            strings: vec![make_sk(0, "Test", *b"LCTN", *b"FULL")],
            compressed_records: 0,
            strings_loaded: 1,
        };

        cache.store(&esp_path, &payload).unwrap();
        assert!(cache.lookup(&esp_path).is_some());

        // Modify file content
        let mut f = std::fs::File::create(&esp_path).unwrap();
        f.write_all(b"modified content").unwrap();

        // Modified file → cache miss
        assert!(cache.lookup(&esp_path).is_none());
    }

    #[test]
    fn test_prune_removes_oldest() {
        let cache_dir = temp_dir().join("cache_prune");
        let cache = EsmCache::new(cache_dir.clone(), 2); // max 2 entries

        // Create 3 different ESP files to generate 3 cache entries
        for i in 0..3 {
            let esp_path = temp_file(
                &format!("prune{}.esp", i),
                format!("content {}", i).as_bytes(),
            );
            let payload = CachePayload {
                version: CACHE_VERSION,
                strings: vec![make_sk(0, &format!("S{}", i), *b"LCTN", *b"FULL")],
                compressed_records: 0,
                strings_loaded: 1,
            };
            cache.store(&esp_path, &payload).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // Should have at most 2 cache files
        let count = std::fs::read_dir(&cache_dir)
            .unwrap()
            .filter(|e| {
                e.as_ref()
                    .ok()
                    .and_then(|e| e.path().extension().map(|x| x == "cache"))
                    .unwrap_or(false)
            })
            .count();
        assert!(count <= 2, "Expected ≤ 2 cache files, got {}", count);
    }

    #[test]
    fn test_compute_record_counts() {
        let strings = vec![
            make_sk(0, "A", *b"LCTN", *b"FULL"),
            make_sk(1, "B", *b"LCTN", *b"FULL"),
            make_sk(2, "C", *b"QUST", *b"NNAM"),
        ];
        let counts = EsmCache::compute_record_counts(&strings);
        assert_eq!(counts.get("LCTN"), Some(&2));
        assert_eq!(counts.get("QUST"), Some(&1));
    }
}
