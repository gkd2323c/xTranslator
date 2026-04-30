//! ESP/ESM 解析结果的 SQLite 缓存
//!
//! 比 bincode 缓存的优势：
//! - 索引查询（按 record_sig / form_id / status 过滤）
//! - 单行更新（编辑翻译时不必重写整个缓存）
//! - 增量加载（未来可支持分页查询）

use crate::types::esp_pointer::EspPointer;
use crate::types::params::{SkyStringInternalParams, SkyStringParams};
use crate::types::sky_string::SkyString;
use rusqlite::{params, Connection};
use std::path::PathBuf;

const CACHE_VERSION: u32 = 2;

/// SQLite 缓存管理器
pub struct SqliteCache {
    cache_dir: PathBuf,
}

impl SqliteCache {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    fn db_path(&self, esp_hash: &str) -> PathBuf {
        self.cache_dir.join(format!("{}.sqlite", esp_hash))
    }

    /// 查找缓存：返回 None 表示未命中
    pub fn lookup(&self, esp_hash: &str) -> Option<CachePayload> {
        let path = self.db_path(esp_hash);
        if !path.exists() {
            return None;
        }
        let conn = Connection::open(&path).ok()?;
        self.read_payload(&conn, esp_hash).ok()
    }

    /// 存储解析结果
    pub fn store(&self, esp_hash: &str, payload: &CachePayload) -> rusqlite::Result<()> {
        std::fs::create_dir_all(&self.cache_dir)
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

        let path = self.db_path(esp_hash);
        // 删除旧文件以确保干净写入
        let _ = std::fs::remove_file(&path);

        let conn = Connection::open(&path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;

        self.create_schema(&conn)?;
        self.write_payload(&conn, esp_hash, payload)?;
        self.prune()?;

        // Touch file so prune keeps recently used
        let _ = std::fs::File::open(&path).and_then(|f| {
            f.set_modified(std::time::SystemTime::now())
        });

        Ok(())
    }

    /// 更新单条翻译（不重写整个缓存）
    pub fn update_translation(&self, esp_hash: &str, id: u32, translation: &str) -> rusqlite::Result<()> {
        let path = self.db_path(esp_hash);
        let conn = Connection::open(&path)?;
        conn.execute(
            "UPDATE strings SET translation = ?1 WHERE id = ?2 AND esp_hash = ?3",
            params![translation, id, esp_hash],
        )?;
        Ok(())
    }

    /// 按 record_sig 查询字符串
    pub fn query_by_record_sig(&self, esp_hash: &str, record_sig: &str) -> rusqlite::Result<Vec<SkyString>> {
        let path = self.db_path(esp_hash);
        let conn = Connection::open(&path)?;
        let mut stmt = conn.prepare(
            "SELECT * FROM strings WHERE esp_hash = ?1 AND record_sig = ?2 ORDER BY id",
        )?;
        let rows = stmt.query_map(params![esp_hash, record_sig], |row| row_to_sky_string(row))?;
        rows.collect()
    }

    /// 统计各 record_sig 的字符串数量
    pub fn compute_record_counts(&self, esp_hash: &str) -> rusqlite::Result<std::collections::HashMap<String, usize>> {
        let path = self.db_path(esp_hash);
        let conn = Connection::open(&path)?;
        let mut stmt = conn.prepare(
            "SELECT record_sig, COUNT(*) FROM strings WHERE esp_hash = ?1 GROUP BY record_sig",
        )?;
        let mut counts = std::collections::HashMap::new();
        let rows = stmt.query_map(params![esp_hash], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, usize>(1)?))
        })?;
        for row in rows {
            let (sig, count) = row?;
            counts.insert(sig, count);
        }
        Ok(counts)
    }

    fn create_schema(&self, conn: &Connection) -> rusqlite::Result<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS cache_meta (
                esp_hash TEXT PRIMARY KEY,
                version INTEGER NOT NULL,
                compressed_records INTEGER NOT NULL DEFAULT 0,
                strings_loaded INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS strings (
                id INTEGER NOT NULL,
                esp_hash TEXT NOT NULL,
                source TEXT NOT NULL,
                translation TEXT NOT NULL DEFAULT '',
                record_sig TEXT NOT NULL,
                field_sig TEXT NOT NULL,
                str_id INTEGER NOT NULL,
                form_id INTEGER NOT NULL DEFAULT 0,
                idx INTEGER NOT NULL DEFAULT 0,
                idx_max INTEGER NOT NULL DEFAULT 0,
                edid_hash INTEGER NOT NULL DEFAULT 0,
                params INTEGER NOT NULL DEFAULT 0,
                internal_params INTEGER NOT NULL DEFAULT 0,
                list_index INTEGER NOT NULL DEFAULT 0,
                parent_form_id INTEGER NOT NULL DEFAULT 0,
                normalized_hash INTEGER,
                hash INTEGER NOT NULL DEFAULT 0,
                hash_trans INTEGER NOT NULL DEFAULT 0,
                colab_id INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (esp_hash, id)
            );
            CREATE INDEX IF NOT EXISTS idx_strings_record_sig ON strings(esp_hash, record_sig);
            CREATE INDEX IF NOT EXISTS idx_strings_form_id ON strings(esp_hash, form_id);
            CREATE INDEX IF NOT EXISTS idx_strings_params ON strings(esp_hash, params);",
        )?;
        Ok(())
    }

    fn write_payload(&self, conn: &Connection, esp_hash: &str, payload: &CachePayload) -> rusqlite::Result<()> {
        conn.execute(
            "INSERT INTO cache_meta (esp_hash, version, compressed_records, strings_loaded) VALUES (?1, ?2, ?3, ?4)",
            params![esp_hash, payload.version, payload.compressed_records, payload.strings_loaded],
        )?;

        let mut stmt = conn.prepare(
            "INSERT INTO strings (
                id, esp_hash, source, translation, record_sig, field_sig,
                str_id, form_id, idx, idx_max, edid_hash,
                params, internal_params, list_index, parent_form_id,
                normalized_hash, hash, hash_trans, colab_id
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6,
                ?7, ?8, ?9, ?10, ?11,
                ?12, ?13, ?14, ?15,
                ?16, ?17, ?18, ?19
            )",
        )?;

        for sk in &payload.strings {
            let rec_sig = String::from_utf8_lossy(&sk.record_sig).to_string();
            let fld_sig = String::from_utf8_lossy(&sk.field_sig).to_string();
            stmt.execute(params![
                sk.id,
                esp_hash,
                sk.source,
                sk.translation,
                rec_sig,
                fld_sig,
                sk.esp_ptr.str_id,
                sk.esp_ptr.form_id,
                sk.esp_ptr.index,
                sk.esp_ptr.index_max,
                sk.esp_ptr.edid_hash,
                sk.params.0,
                sk.internal_params.0,
                sk.list_index,
                sk.parent_form_id,
                sk.normalized_hash,
                sk.hash,
                sk.hash_trans,
                sk.colab_id,
            ])?;
        }

        Ok(())
    }

    fn read_payload(&self, conn: &Connection, esp_hash: &str) -> rusqlite::Result<CachePayload> {
        let meta: (u32, u32, u8) = conn.query_row(
            "SELECT version, compressed_records, strings_loaded FROM cache_meta WHERE esp_hash = ?1",
            params![esp_hash],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;

        if meta.0 != CACHE_VERSION {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "Cache version mismatch: expected {}, got {}",
                CACHE_VERSION, meta.0
            )));
        }

        let mut stmt = conn.prepare("SELECT * FROM strings WHERE esp_hash = ?1 ORDER BY id")?;
        let strings: Vec<SkyString> = stmt
            .query_map(params![esp_hash], |row| row_to_sky_string(row))?
            .collect::<rusqlite::Result<_>>()?;

        Ok(CachePayload {
            version: meta.0,
            strings,
            compressed_records: meta.1,
            strings_loaded: meta.2,
        })
    }

    fn prune(&self) -> rusqlite::Result<()> {
        const MAX_ENTRIES: usize = 50;
        if !self.cache_dir.exists() {
            return Ok(());
        }

        let mut entries: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
        for entry in std::fs::read_dir(&self.cache_dir).map_err(|e| {
            rusqlite::Error::InvalidParameterName(e.to_string())
        })? {
            let entry = entry.map_err(|e| {
                rusqlite::Error::InvalidParameterName(e.to_string())
            })?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("sqlite") {
                continue;
            }
            let modified = entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .unwrap_or(std::time::UNIX_EPOCH);
            entries.push((modified, path));
        }

        if entries.len() <= MAX_ENTRIES {
            return Ok(());
        }

        entries.sort_by_key(|(t, _)| *t);
        let to_remove = entries.len() - MAX_ENTRIES;
        for (_, path) in entries.iter().take(to_remove) {
            let _ = std::fs::remove_file(path);
        }

        Ok(())
    }
}

/// 缓存载荷（与 bincode cache 的 CachePayload 兼容）
pub struct CachePayload {
    pub version: u32,
    pub strings: Vec<SkyString>,
    pub compressed_records: u32,
    pub strings_loaded: u8,
}

fn sig_from_str(s: &str) -> [u8; 4] {
    let bytes = s.as_bytes();
    let mut sig = [0u8; 4];
    let len = bytes.len().min(4);
    sig[..len].copy_from_slice(&bytes[..len]);
    sig
}

fn row_to_sky_string(row: &rusqlite::Row) -> rusqlite::Result<SkyString> {
    let rec_sig_str: String = row.get("record_sig")?;
    let fld_sig_str: String = row.get("field_sig")?;

    let mut sk = SkyString::new_without_search_index(
        row.get("id")?,
        row.get("source")?,
        row.get("translation")?,
        sig_from_str(&rec_sig_str),
        sig_from_str(&fld_sig_str),
    );

    sk.esp_ptr = EspPointer {
        str_id: row.get("str_id")?,
        form_id: row.get("form_id")?,
        record_sig: sig_from_str(&rec_sig_str),
        field_sig: sig_from_str(&fld_sig_str),
        index: row.get("idx")?,
        index_max: row.get("idx_max")?,
        edid_hash: row.get("edid_hash")?,
    };

    let params_val: u8 = row.get("params")?;
    sk.params = SkyStringParams(params_val);
    let internal_val: u64 = row.get("internal_params")?;
    sk.internal_params = SkyStringInternalParams(internal_val);
    sk.list_index = row.get("list_index")?;
    sk.parent_form_id = row.get("parent_form_id")?;
    sk.normalized_hash = row.get("normalized_hash")?;
    sk.hash = row.get("hash")?;
    sk.hash_trans = row.get("hash_trans")?;
    sk.colab_id = row.get("colab_id")?;

    Ok(sk)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::esp_pointer::EspPointer;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("xt_sqlite_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).ok();
        dir
    }

    fn make_sk(id: u32, source: &str, rec: [u8; 4], field: [u8; 4]) -> SkyString {
        let mut sk = SkyString::new(id, source.to_string(), String::new(), rec, field);
        sk.esp_ptr = EspPointer {
            str_id: id as i32,
            form_id: 0x1234,
            record_sig: rec,
            field_sig: field,
            index: 0,
            index_max: 0,
            edid_hash: 0xABCD,
        };
        sk.params.set(SkyStringParams::INCOMPLETE_TRANS, true);
        sk
    }

    #[test]
    fn test_store_and_lookup() {
        let dir = temp_dir().join("sqlite1");
        let cache = SqliteCache::new(dir);

        let strings = vec![
            make_sk(0, "Hello", *b"LCTN", *b"FULL"),
            make_sk(1, "World", *b"QUST", *b"NNAM"),
        ];

        let payload = CachePayload {
            version: CACHE_VERSION,
            strings,
            compressed_records: 42,
            strings_loaded: 3,
        };

        cache.store("abc123", &payload).unwrap();

        let cached = cache.lookup("abc123").unwrap();
        assert_eq!(cached.strings.len(), 2);
        assert_eq!(cached.strings[0].source, "Hello");
        assert_eq!(cached.strings[1].source, "World");
        assert_eq!(cached.compressed_records, 42);
        assert_eq!(cached.strings_loaded, 3);
    }

    #[test]
    fn test_cache_miss() {
        let dir = temp_dir().join("sqlite2");
        let cache = SqliteCache::new(dir);
        assert!(cache.lookup("nonexistent").is_none());
    }

    #[test]
    fn test_update_translation() {
        let dir = temp_dir().join("sqlite3");
        let cache = SqliteCache::new(dir);

        let strings = vec![make_sk(0, "Hello", *b"LCTN", *b"FULL")];
        let payload = CachePayload {
            version: CACHE_VERSION,
            strings,
            compressed_records: 0,
            strings_loaded: 1,
        };

        cache.store("hash1", &payload).unwrap();
        cache.update_translation("hash1", 0, "你好").unwrap();

        let cached = cache.lookup("hash1").unwrap();
        assert_eq!(cached.strings[0].translation, "你好");
    }

    #[test]
    fn test_query_by_record_sig() {
        let dir = temp_dir().join("sqlite4");
        let cache = SqliteCache::new(dir);

        let strings = vec![
            make_sk(0, "A", *b"LCTN", *b"FULL"),
            make_sk(1, "B", *b"LCTN", *b"FULL"),
            make_sk(2, "C", *b"QUST", *b"NNAM"),
        ];
        let payload = CachePayload {
            version: CACHE_VERSION,
            strings,
            compressed_records: 0,
            strings_loaded: 3,
        };

        cache.store("hash2", &payload).unwrap();

        let lctn = cache.query_by_record_sig("hash2", "LCTN").unwrap();
        assert_eq!(lctn.len(), 2);

        let qust = cache.query_by_record_sig("hash2", "QUST").unwrap();
        assert_eq!(qust.len(), 1);
    }

    #[test]
    fn test_compute_record_counts() {
        let dir = temp_dir().join("sqlite5");
        let cache = SqliteCache::new(dir);

        let strings = vec![
            make_sk(0, "A", *b"LCTN", *b"FULL"),
            make_sk(1, "B", *b"LCTN", *b"FULL"),
            make_sk(2, "C", *b"QUST", *b"NNAM"),
        ];
        let payload = CachePayload {
            version: CACHE_VERSION,
            strings,
            compressed_records: 0,
            strings_loaded: 3,
        };

        cache.store("hash3", &payload).unwrap();

        let counts = cache.compute_record_counts("hash3").unwrap();
        assert_eq!(counts.get("LCTN"), Some(&2));
        assert_eq!(counts.get("QUST"), Some(&1));
    }
}
