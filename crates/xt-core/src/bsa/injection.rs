//! BSA replacement injection（DP-06）
//!
//! 基于 Delphi `TESVT_bsa.pas::TwbBSAFile.InjectData/InjectDataFile` 复刻。
//!
//! 只实现"替换归档内已存在文件"：
//! - 复制 header + 目录区到临时文件
//! - 逐文件注入：命中替换映射 → 用新数据（保留原压缩策略）；否则原样复制
//! - 重写文件表的 size（含压缩标志位）与 offset
//! - 写入临时文件后由调用方完成校验与原子替换

use crate::bsa::directory::BsaDirectory;
use crate::bsa::header::{
    BsaHeader, BSAARCHIVE_COMPRESSFILES, BSAFILE_COMPRESS, BSAHEADER_VERSION_SSE,
};
use std::collections::HashMap;
use std::io::{Read, Result, Seek, SeekFrom, Write};

/// BSA 注入结果
#[derive(Debug, Clone)]
pub struct BsaInjectionSummary {
    /// 注入的文件数
    pub injected: usize,
    /// 未在归档中找到的请求路径（key 为请求的原始路径）
    pub not_found: Vec<String>,
    /// 输出的临时文件字节数
    pub output_size: u64,
}

/// 计算数据区起始偏移：目录中所有文件 offset 的最小值。
/// 空归档无文件时返回 0（仅 header，由调用方决定是否允许）。
fn compute_data_start(directory: &BsaDirectory) -> u64 {
    directory
        .folders
        .iter()
        .flat_map(|f| f.files.iter())
        .map(|f| f.offset as u64)
        .min()
        .unwrap_or(0)
}

/// 构造存储的 raw_size（含压缩标志位与 flag 9 名字前缀长度）。
///
/// Delphi `SetFileCompressedFlag(a, compressed, addLen, addLen_flag)`：
/// - `packed_len`：写入的数据块总长（压缩条目已含 [u32 uSize] 前缀，即 Delphi 的 a + addLen）
/// - `name_prefix_len`：flag 9 时写入的名字前缀长度（Delphi 的 addLen_flag）
/// - archive COMPRESSFILES 置位：compressed → 长度（标志位 0）；未压缩 → 长度 | BSAFILE_COMPRESS
/// - archive COMPRESSFILES 未置位：compressed → 长度 | BSAFILE_COMPRESS；未压缩 → 长度
fn make_raw_size(header: &BsaHeader, packed_len: u32, name_prefix_len: u32, is_compressed: bool) -> u32 {
    let len_with_prefix = packed_len + name_prefix_len;
    let compress_flag_set = (header.archive_flags & BSAARCHIVE_COMPRESSFILES) != 0;
    if compress_flag_set {
        if is_compressed {
            len_with_prefix
        } else {
            len_with_prefix | BSAFILE_COMPRESS
        }
    } else if is_compressed {
        len_with_prefix | BSAFILE_COMPRESS
    } else {
        len_with_prefix
    }
}

fn read_u8<R: Read>(r: &mut R) -> Result<u8> {
    let mut buf = [0u8; 1];
    r.read_exact(&mut buf)?;
    Ok(buf[0])
}

fn copy_exact<R: Read, W: Write>(r: &mut R, w: &mut W, mut len: u64) -> Result<()> {
    let mut buf = [0u8; 64 * 1024];
    while len > 0 {
        let chunk = len.min(buf.len() as u64) as usize;
        r.read_exact(&mut buf[..chunk])?;
        w.write_all(&buf[..chunk])?;
        len -= chunk as u64;
    }
    Ok(())
}

/// 把注入数据写入输出流；保留原压缩策略。
///
/// `replacements`: `folder\filename`（小写）→ 新数据。
/// 返回 (新 offset, 新 raw_size)。
fn write_file_data<W: Write + Seek, R: Read + Seek>(
    out: &mut W,
    source: &mut R,
    header: &BsaHeader,
    folder_name: &str,
    file: &crate::bsa::directory::BsaFileRecord,
    replacements: &HashMap<String, Vec<u8>>,
) -> Result<(u64, u32)> {
    let data_offset = out.stream_position()?;
    let rel_path = format!("{}/{}", folder_name, file.name);
    let (is_compressed, stored_size) = header.is_file_compressed(file.raw_size);

    // flag 9（PREFIXFULLFILENAMES）：每个文件数据前有一个名字前缀（len + name + null）。
    // 注入时保留原始前缀（Delphi InjectDataFile 从源复制）。
    let mut name_prefix_len: u32 = 0;
    if header.has_prefix_full_filenames() {
        source.seek(SeekFrom::Start(file.offset as u64))?;
        let len_byte = read_u8(source)?;
        out.write_all(&[len_byte])?;
        let mut name_buf = vec![0u8; len_byte as usize];
        source.read_exact(&mut name_buf)?;
        out.write_all(&name_buf)?;
        out.write_all(&[0u8])?; // 终止 null
        name_prefix_len = len_byte as u32 + 2; // len 字节 + name + null
    }

    if let Some(new_data) = replacements.get(&rel_path) {
        // ── 注入新数据 ──
        let new_size = new_data.len() as u32;
        let packed_len = if is_compressed {
            // 压缩：写 [u32 解压后大小] + 压缩数据
            out.write_all(&new_size.to_le_bytes())?;
            if header.version == BSAHEADER_VERSION_SSE {
                let compressed = lz4::block::compress(new_data, None, true)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
                out.write_all(&compressed)?;
                new_size + 4 + compressed.len() as u32
            } else {
                let mut encoder = flate2::write::ZlibEncoder::new(
                    Vec::new(),
                    flate2::Compression::default(),
                );
                encoder
                    .write_all(new_data)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
                let compressed = encoder
                    .finish()
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
                out.write_all(&compressed)?;
                new_size + 4 + compressed.len() as u32
            }
        } else {
            out.write_all(new_data)?;
            new_size
        };

        let raw_size = make_raw_size(header, packed_len, name_prefix_len, is_compressed);
        Ok((data_offset, raw_size))
    } else {
        // ── 原样复制整个数据块（含 uSize 前缀与名字前缀） ──
        // stored_size = raw_size 去掉压缩标志位，包含压缩条目的 [u32 uSize] 前缀
        source.seek(SeekFrom::Start(file.offset as u64 + name_prefix_len as u64))?;
        copy_exact(source, out, stored_size as u64 - name_prefix_len as u64)?;
        Ok((data_offset, file.raw_size))
    }
}

/// 计算文件表条目的绝对位置。
///
/// BSA 目录区布局：header → folder records → 每个 folder: [folder name][file records]
/// → file names → data。因此第 N 个文件记录的位置需要累加此前所有 folder name 长度。
fn file_table_position(
    header: &BsaHeader,
    directory: &BsaDirectory,
    target_folder_idx: usize,
    file_idx_in_folder: usize,
) -> u64 {
    let header_bytes = 36u32; // BSA\0(4) + 8 u32（解析器 read_from 读取的总长）
    let folder_record_bytes = header.folder_offset_size() as u64;

    let mut pos = header_bytes as u64 + directory.folders.len() as u64 * folder_record_bytes;
    // 累加 target folder 之前的 folder name 与 file records
    for (i, folder) in directory.folders.iter().enumerate() {
        if i == target_folder_idx {
            break;
        }
        // folder name（len 字节 + name + null）
        pos += folder.name.len() as u64 + 2;
        pos += folder.files.len() as u64 * 16;
    }
    // target folder 自身的 name 前缀（len 字节 + name + null）
    pos += directory.folders[target_folder_idx].name.len() as u64 + 2;
    // 该 folder 内的文件记录
    pos += file_idx_in_folder as u64 * 16;
    pos
}

/// 重写文件表条目：在 `out` 的指定位置写 size（含标志位）与 offset。
/// 文件表布局（每个文件 16 字节）：hash(u64) + raw_size(u32) + offset(u32)。
fn rewrite_table_entry<W: Write + Seek>(
    out: &mut W,
    table_pos: u64,
    raw_size: u32,
    offset: u64,
) -> Result<()> {
    out.seek(SeekFrom::Start(table_pos + 8))?; // 跳过 hash
    out.write_all(&raw_size.to_le_bytes())?;
    out.write_all(&(offset as u32).to_le_bytes())?;
    Ok(())
}

/// 执行 BSA replacement injection，写入 `output` 流。
///
/// `source_path`: 源 BSA 路径（需要重新打开读取原始数据）。
/// `replacements`: 小写 `folder\filename` → 新数据。
///
/// 调用方负责：输出到临时文件 → 完整校验 → 备份原文件 → 原子替换。
pub fn inject_bsa<W: Write + Seek>(
    source_path: &std::path::Path,
    header: &BsaHeader,
    directory: &BsaDirectory,
    output: &mut W,
    replacements: &HashMap<String, Vec<u8>>,
) -> Result<BsaInjectionSummary> {
    let data_start = compute_data_start(directory);
    let mut source = std::fs::File::open(source_path)?;

    // 1. 复制 header + 目录区（到数据起始）
    source.seek(SeekFrom::Start(0))?;
    copy_exact(&mut source, output, data_start)?;

    // 2. 逐文件夹逐文件注入；维护 folder/file 索引以定位文件表条目
    let mut injected = 0usize;
    let mut not_found: Vec<String> = Vec::new();
    let mut requested: Vec<&String> = replacements.keys().collect();

    for (folder_idx, folder) in directory.folders.iter().enumerate() {
        for (file_idx, file) in folder.files.iter().enumerate() {
            let rel_path = format!("{}/{}", folder.name, file.name);
            let was_injected = replacements.contains_key(&rel_path);
            let (new_offset, new_raw_size) =
                write_file_data(output, &mut source, header, &folder.name, file, replacements)?;
            if was_injected {
                injected += 1;
                if let Some(pos) = requested.iter().position(|p| *p == &rel_path) {
                    requested.remove(pos);
                }
            }
            // 记录数据末尾位置，重写表条目后恢复
            let data_end = output.stream_position()?;
            let table_pos = file_table_position(header, directory, folder_idx, file_idx);
            rewrite_table_entry(output, table_pos, new_raw_size, new_offset)?;
            output.seek(SeekFrom::Start(data_end))?;
        }
    }

    for remaining in requested {
        not_found.push(remaining.clone());
    }

    let output_size = output.stream_position()?;
    Ok(BsaInjectionSummary {
        injected,
        not_found,
        output_size,
    })
}
