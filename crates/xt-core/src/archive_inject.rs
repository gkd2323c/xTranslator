//! 归档注入安全封装（DP-06）
//!
//! 统一 BSA/BA2 replacement injection 的磁盘安全流程：
//! 1. 打开并解析归档，确认存在（结构校验）
//! 2. 写入临时文件（同目录，确保同文件系统）
//! 3. 重新打开临时文件验证可读
//! 4. 备份原文件
//! 5. 原子替换（Windows 上先删后改，POSIX 用 rename）

use crate::ba2::Ba2Archive;
use crate::bsa::BsaArchive;
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

/// 注入结果汇总
#[derive(Debug, Clone)]
pub struct InjectResult {
    /// 注入的文件数
    pub injected: usize,
    /// 未在归档中找到的请求路径
    pub not_found: Vec<String>,
    /// 备份文件路径（原文件被替换前保留）
    pub backup_path: Option<PathBuf>,
    /// 最终归档字节数
    pub output_size: u64,
}

/// 判断归档类型并执行注入。
///
/// `archive_path`: .bsa 或 .ba2 文件路径。
/// `replacements`: 小写路径（`/` 或 `\`）→ 新数据。
/// `create_backup`: 替换前是否备份原文件。
pub fn inject_archive(
    archive_path: &Path,
    replacements: &HashMap<String, Vec<u8>>,
    create_backup: bool,
) -> Result<InjectResult, String> {
    let ext = archive_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "bsa" => inject_bsa_file(archive_path, replacements, create_backup),
        "ba2" => inject_ba2_file(archive_path, replacements, create_backup),
        other => Err(format!(
            "unsupported archive extension: {other}; expected .bsa or .ba2"
        )),
    }
}

fn inject_bsa_file(
    archive_path: &Path,
    replacements: &HashMap<String, Vec<u8>>,
    create_backup: bool,
) -> Result<InjectResult, String> {
    let archive = BsaArchive::open(archive_path)
        .map_err(|e| format!("failed to open BSA {}: {e}", archive_path.display()))?;

    // 结构校验：所有替换目标必须存在于归档中（fail-closed）
    let missing: Vec<String> = replacements
        .keys()
        .filter(|k| !archive.contains_file(k))
        .cloned()
        .collect();
    if !missing.is_empty() {
        return Err(format!(
            "replacements reference files not in archive: {}",
            missing.join(", ")
        ));
    }

    let tmp_path = temp_sibling(archive_path, "inject");
    let mut out =
        std::fs::File::create(&tmp_path).map_err(|e| format!("failed to create temp file: {e}"))?;

    let summary = archive
        .inject_file(&mut out, replacements)
        .map_err(|e| format!("injection failed: {e}"))?;
    out.flush().map_err(|e| format!("flush failed: {e}"))?;

    // 校验：重新打开临时文件确认可读
    verify_bsa(&tmp_path)?;

    // 原子替换 + 备份
    let backup = replace_atomically(archive_path, &tmp_path, create_backup)?;

    Ok(InjectResult {
        injected: summary.injected,
        not_found: summary.not_found,
        backup_path: backup,
        output_size: summary.output_size,
    })
}

fn inject_ba2_file(
    archive_path: &Path,
    replacements: &HashMap<String, Vec<u8>>,
    create_backup: bool,
) -> Result<InjectResult, String> {
    let archive = Ba2Archive::open(archive_path)
        .map_err(|e| format!("failed to open BA2 {}: {e}", archive_path.display()))?;

    let missing: Vec<String> = replacements
        .keys()
        .filter(|k| !archive.contains_file(k))
        .cloned()
        .collect();
    if !missing.is_empty() {
        return Err(format!(
            "replacements reference files not in archive: {}",
            missing.join(", ")
        ));
    }

    let tmp_path = temp_sibling(archive_path, "inject");
    let mut out =
        std::fs::File::create(&tmp_path).map_err(|e| format!("failed to create temp file: {e}"))?;

    let summary = archive
        .inject_file(&mut out, replacements)
        .map_err(|e| format!("injection failed: {e}"))?;
    out.flush().map_err(|e| format!("flush failed: {e}"))?;

    verify_ba2(&tmp_path)?;

    let backup = replace_atomically(archive_path, &tmp_path, create_backup)?;

    Ok(InjectResult {
        injected: summary.injected,
        not_found: summary.not_found,
        backup_path: backup,
        output_size: summary.output_size,
    })
}

fn verify_bsa(path: &Path) -> Result<(), String> {
    BsaArchive::open(path)
        .map(|_| ())
        .map_err(|e| format!("verification failed: reopened BSA unreadable: {e}"))
}

fn verify_ba2(path: &Path) -> Result<(), String> {
    Ba2Archive::open(path)
        .map(|_| ())
        .map_err(|e| format!("verification failed: reopened BA2 unreadable: {e}"))
}

/// 生成同目录临时文件路径（确保与目标同文件系统，rename 原子性成立）
fn temp_sibling(target: &Path, tag: &str) -> PathBuf {
    let file_name = target
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("archive");
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    target
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!(".{file_name}.{tag}.{stamp}.tmp"))
}

/// 原子替换：备份原文件（可选）→ 临时文件改名为目标。
#[cfg(windows)]
fn replace_atomically(
    target: &Path,
    tmp: &Path,
    create_backup: bool,
) -> Result<Option<PathBuf>, String> {
    let backup = if create_backup {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let backup_path = target.with_extension(format!(
            "{}.backup.{}",
            target
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("archive"),
            stamp
        ));
        std::fs::copy(target, &backup_path).map_err(|e| format!("failed to create backup: {e}"))?;
        Some(backup_path)
    } else {
        None
    };

    // Windows rename 不能覆盖已存在文件：先删后改（非原子，但同目录操作窗口极小）
    if target.exists() {
        std::fs::remove_file(target)
            .map_err(|e| format!("failed to remove original for replace: {e}"))?;
    }
    std::fs::rename(tmp, target).map_err(|e| format!("failed to replace archive: {e}"))?;
    Ok(backup)
}

/// 原子替换（POSIX：rename 原子覆盖）
#[cfg(not(windows))]
fn replace_atomically(
    target: &Path,
    tmp: &Path,
    create_backup: bool,
) -> Result<Option<PathBuf>, String> {
    let backup = if create_backup {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let backup_path = target.with_extension(format!(
            "{}.backup.{}",
            target
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("archive"),
            stamp
        ));
        std::fs::copy(target, &backup_path).map_err(|e| format!("failed to create backup: {e}"))?;
        Some(backup_path)
    } else {
        None
    };

    std::fs::rename(tmp, target).map_err(|e| format!("failed to replace archive: {e}"))?;
    Ok(backup)
}
