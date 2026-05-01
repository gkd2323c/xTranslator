//! 翻译结果独立缓存（append-only journal）
//!
//! 与 ESP 解析缓存（[`crate::cache::EsmCache`]）分离，专门存储批量翻译的中间结果。
//!
//! 设计：
//! - 格式：JSONL（每行一条 JSON），崩溃安全
//! - 文件：`{cache_dir}/xTranslator/translation_cache/{esp_hash}.journal`
//! - 写入：append + flush，每条翻译完成后立即持久化
//! - 恢复：启动时扫描 journal，未应用的翻译提示用户恢复

use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

const JOURNAL_EXT: &str = "journal";
const CACHE_SUBDIR: &str = "translation_cache";

/// 单条翻译缓存记录（JSONL 中的一行）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TranslationCacheRecord {
    pub str_id: i32,
    pub source_text: String,
    pub translated_text: String,
    /// UNIX epoch 秒数
    pub timestamp: u64,
}

/// 恢复检测结果（通过 IPC 传给前端）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecoveryDetection {
    pub esp_name: String,
    pub pending_count: u32,
    pub cache_file_path: String,
}

/// 翻译缓存管理器
pub struct TranslationCache {
    base_dir: PathBuf,
}

impl TranslationCache {
    /// 创建新的翻译缓存管理器
    ///
    /// `base_dir` 通常为 `%LOCALAPPDATA%/xTranslator` 或等效路径。
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    fn cache_dir(&self) -> PathBuf {
        self.base_dir.join(CACHE_SUBDIR)
    }

    fn journal_path(&self, esp_hash: &str) -> PathBuf {
        self.cache_dir().join(format!("{}.{}", esp_hash, JOURNAL_EXT))
    }

    /// 追加一条翻译记录到 journal 文件
    ///
    /// 自动创建目录和文件。每行 JSON 后立即 flush，确保崩溃安全。
    pub fn append_translation(
        &self,
        esp_hash: &str,
        str_id: i32,
        source_text: &str,
        translated_text: &str,
    ) -> Result<(), String> {
        let dir = self.cache_dir();
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("创建缓存目录失败: {}", e))?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let record = TranslationCacheRecord {
            str_id,
            source_text: source_text.to_string(),
            translated_text: translated_text.to_string(),
            timestamp,
        };

        let line = serde_json::to_string(&record)
            .map_err(|e| format!("序列化缓存记录失败: {}", e))?;

        let path = self.journal_path(esp_hash);
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| format!("打开 journal 文件失败: {}", e))?;

        writeln!(file, "{}", line).map_err(|e| format!("写入 journal 失败: {}", e))?;
        file.flush().map_err(|e| format!("flush journal 失败: {}", e))?;

        Ok(())
    }

    /// 读取 journal 文件中的所有有效记录
    ///
    /// 损坏行（反序列化失败）被跳过，不丢失其余数据。
    pub fn read_all(&self, esp_hash: &str) -> Result<Vec<TranslationCacheRecord>, String> {
        let path = self.journal_path(esp_hash);
        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = std::fs::File::open(&path)
            .map_err(|e| format!("打开 journal 文件失败: {}", e))?;
        let reader = BufReader::new(file);

        let mut records = Vec::new();
        for line_result in reader.lines() {
            let line = match line_result {
                Ok(l) => l,
                Err(_) => continue,
            };
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(record) = serde_json::from_str::<TranslationCacheRecord>(&line) {
                records.push(record);
            }
            // 损坏行：静默跳过
        }

        Ok(records)
    }

    /// 检测是否有未应用到 ESP 的翻译缓存
    ///
    /// `current_translations`: `(str_id, current_translation_text)` 列表。
    pub fn detect_pending(
        &self,
        esp_hash: &str,
        esp_name: &str,
        current_translations: &[(i32, Option<&str>)],
    ) -> Result<Option<RecoveryDetection>, String> {
        let records = self.read_all(esp_hash)?;
        if records.is_empty() {
            return Ok(None);
        }

        let current_map: std::collections::HashMap<i32, Option<String>> = current_translations
            .iter()
            .map(|(id, trans)| (*id, trans.map(|t| t.to_string())))
            .collect();

        let pending: Vec<_> = records
            .into_iter()
            .filter(|r| {
                if r.translated_text.is_empty() {
                    return false;
                }
                match current_map.get(&r.str_id) {
                    Some(Some(existing)) => existing != &r.translated_text,
                    _ => true,
                }
            })
            .collect();

        if pending.is_empty() {
            return Ok(None);
        }

        Ok(Some(RecoveryDetection {
            esp_name: esp_name.to_string(),
            pending_count: pending.len() as u32,
            cache_file_path: self.journal_path(esp_hash).to_string_lossy().to_string(),
        }))
    }

    /// 删除 journal 文件
    pub fn discard_cache(&self, esp_hash: &str) -> Result<(), String> {
        let path = self.journal_path(esp_hash);
        if path.exists() {
            std::fs::remove_file(&path)
                .map_err(|e| format!("删除 journal 文件失败: {}", e))?;
        }
        Ok(())
    }

    /// 读取缓存中所有非空翻译记录，返回 `(str_id, translated_text)`
    pub fn read_translations(&self, esp_hash: &str) -> Result<Vec<(i32, String)>, String> {
        let records = self.read_all(esp_hash)?;
        Ok(records
            .into_iter()
            .filter(|r| !r.translated_text.is_empty())
            .map(|r| (r.str_id, r.translated_text))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_cache() -> (TranslationCache, std::path::PathBuf) {
        let id: u64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        let tmp = std::env::temp_dir().join(format!("xt_cache_test_{}_{}", std::process::id(), id));
        let _ = std::fs::remove_dir_all(&tmp);
        let cache = TranslationCache::new(tmp.clone());
        (cache, tmp)
    }

    #[test]
    fn test_append_and_read() {
        let (cache, tmp) = setup_cache();
        let hash = "abc123def456";

        cache.append_translation(hash, 1, "Iron Sword", "铁剑").unwrap();
        cache.append_translation(hash, 2, "Steel Armor", "钢甲").unwrap();

        let records = cache.read_all(hash).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].str_id, 1);
        assert_eq!(records[0].translated_text, "铁剑");
        assert_eq!(records[1].str_id, 2);
        assert_eq!(records[1].translated_text, "钢甲");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_empty_read() {
        let (cache, tmp) = setup_cache();
        let records = cache.read_all("nonexistent").unwrap();
        assert!(records.is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_detect_pending() {
        let (cache, tmp) = setup_cache();
        let hash = "detect_test";

        cache.append_translation(hash, 1, "Hello", "你好").unwrap();
        cache.append_translation(hash, 2, "World", "世界").unwrap();

        // Case 1: all translations match current — no pending
        let current = vec![(1i32, Some("你好")), (2i32, Some("世界"))];
        let result = cache.detect_pending(hash, "test.esp", &current).unwrap();
        assert!(result.is_none());

        // Case 2: translation differs — pending
        let current = vec![(1i32, Some("你好")), (2i32, Some("地球"))];
        let result = cache.detect_pending(hash, "test.esp", &current).unwrap();
        assert!(result.is_some());
        assert_eq!(result.as_ref().unwrap().pending_count, 1);

        // Case 3: no existing translation — pending
        let current = vec![(1i32, Some("你好"))];
        let result = cache.detect_pending(hash, "test.esp", &current).unwrap();
        assert!(result.is_some());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_discard_cache() {
        let (cache, tmp) = setup_cache();
        let hash = "discard_test";

        cache.append_translation(hash, 1, "Test", "测试").unwrap();
        assert!(cache.journal_path(hash).exists());

        cache.discard_cache(hash).unwrap();
        assert!(!cache.journal_path(hash).exists());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_read_translations() {
        let (cache, tmp) = setup_cache();
        let hash = "read_test";

        cache.append_translation(hash, 1, "A", "甲").unwrap();
        cache.append_translation(hash, 2, "B", "").unwrap(); // empty translation
        cache.append_translation(hash, 3, "C", "丙").unwrap();

        let translations = cache.read_translations(hash).unwrap();
        assert_eq!(translations.len(), 2); // empty skipped
        assert_eq!(translations[0], (1, "甲".to_string()));
        assert_eq!(translations[1], (3, "丙".to_string()));

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
