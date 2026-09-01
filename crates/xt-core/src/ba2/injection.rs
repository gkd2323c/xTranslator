//! BA2 GNRL replacement injection（DP-06）
//!
//! 基于 Delphi `TESVT_bsa.pas::TwbBA2File.InjectData` 复刻。
//!
//! 只实现 GNRL（通用文件）归档的"替换已存在文件"：
//! - 复制 header 与数据区
//! - 逐文件注入：命中替换映射 → 用新数据（保留原压缩策略，zlib）；否则原样复制
//! - 重写文件表中每个 entry 的 offset / packed_size / size
//! - 复制 string table，更新文件表偏移
//! - DX10 纹理归档不支持（与 Delphi 主路径一致）

use crate::ba2::directory::Ba2FileRecord;
use crate::ba2::header::Ba2Header;
use std::collections::HashMap;
use std::io::{Read, Result, Seek, SeekFrom, Write};

/// BA2 注入结果
#[derive(Debug, Clone)]
pub struct Ba2InjectionSummary {
    /// 注入的文件数
    pub injected: usize,
    /// 未在归档中找到的请求路径
    pub not_found: Vec<String>,
    /// 输出的临时文件字节数
    pub output_size: u64,
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

/// 文件表中单个记录的长度（name 长度前缀字节 + UTF-16 字符 + 终止 null + 36 字节 record）。
fn table_record_len(name: &str) -> u64 {
    // name 以 UTF-16 LE 存储；读入时已解码为 String，原始字节数 = char 数 * 2
    let char_count = name.encode_utf16().count() as u64;
    1 + char_count * 2 + 2 + 36
}

/// 写入注入数据（zlib 压缩或原样），返回 (packed_size, size)。
fn write_injected<W: Write>(out: &mut W, data: &[u8], is_compressed: bool) -> Result<(u32, u32)> {
    let size = data.len() as u32;
    if is_compressed {
        let mut encoder =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder
            .write_all(data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        let compressed = encoder
            .finish()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        let packed = compressed.len() as u32;
        out.write_all(&compressed)?;
        Ok((packed, size))
    } else {
        out.write_all(data)?;
        Ok((size, size))
    }
}

/// 执行 BA2 GNRL replacement injection，写入 `output` 流。
///
/// `source_path`: 源 BA2 路径。
/// `replacements`: 小写路径（`/` 或 `\`）→ 新数据。
/// `files`: 已解析的文件记录（顺序即文件表顺序）。
///
/// 调用方负责：输出到临时文件 → 校验 → 备份 → 原子替换。
pub fn inject_ba2<W: Write + Seek>(
    source_path: &std::path::Path,
    header: &Ba2Header,
    files: &[Ba2FileRecord],
    output: &mut W,
    replacements: &HashMap<String, Vec<u8>>,
) -> Result<Ba2InjectionSummary> {
    let mut source = std::fs::File::open(source_path)?;
    let table_offset = header.file_table_offset as u64;

    // 预计算文件表起始处每个记录的绝对位置（源内）
    let mut table_positions: Vec<u64> = Vec::with_capacity(files.len());
    {
        let mut pos = table_offset;
        for file in files {
            table_positions.push(pos);
            pos += table_record_len(&file.name);
        }
    }

    // 1. 复制 header（24 字节 FO4 / 32 字节 SF）—— 只复制 header，数据区随后重建
    //    header 大小：FO4/FO4B = 24，SF = 32
    let header_size = if header.version == crate::ba2::header::BA2_VERSION_SF {
        32
    } else {
        24
    };
    source.seek(SeekFrom::Start(0))?;
    copy_exact(&mut source, output, header_size as u64)?;

    // 2. 逐文件重建数据区
    let mut new_offsets: Vec<(u64, u32, u32)> = Vec::with_capacity(files.len());
    let mut injected = 0usize;
    let mut not_found: Vec<String> = Vec::new();
    let mut found_keys: Vec<String> = Vec::new();

    for file in files {
        let data_offset = output.stream_position()?;
        let rel_key = file.name.replace('\\', "/").to_lowercase();

        if let Some(new_data) = replacements.get(&rel_key) {
            let (packed, size) = write_injected(output, new_data, file.is_packed)?;
            injected += 1;
            new_offsets.push((data_offset, packed, size));
            found_keys.push(rel_key);
        } else {
            source.seek(SeekFrom::Start(file.offset as u64))?;
            let copy_len = if file.is_packed {
                file.packed_size as u64
            } else {
                file.size as u64
            };
            copy_exact(&mut source, output, copy_len)?;
            new_offsets.push((data_offset, file.packed_size, file.size));
        }
    }

    // 3. 记录新的文件表起始位置
    let new_table_offset = output.stream_position()?;

    // 4. 逐文件复制文件表（原样），并在复制后重写该 entry 的 offset/size
    let mut entry_start = new_table_offset;
    for (i, file) in files.iter().enumerate() {
        let src_pos = table_positions[i];
        let record_len = table_record_len(&file.name);
        // 复制该 entry 的原始字节（name + record）
        source.seek(SeekFrom::Start(src_pos))?;
        copy_exact(&mut source, output, record_len)?;
        // 重写 record 内的 offset/packedSize/size
        // record 起始 = 该 entry 末尾 - 36；offset 在 record+16，packedSize +24，size +28
        let record_start = entry_start + record_len - 36;
        let (offset, packed, size) = new_offsets[i];
        output.seek(SeekFrom::Start(record_start + 16))?;
        output.write_all(&(offset as i64).to_le_bytes())?;
        output.write_all(&packed.to_le_bytes())?;
        output.write_all(&size.to_le_bytes())?;
        // 回到写入位置末尾
        entry_start += record_len;
        output.seek(SeekFrom::Start(entry_start))?;
    }

    // 5. 复制 string table（源文件表之后的所有内容）
    let source_end = source.metadata().map(|m| m.len()).unwrap_or(table_offset);
    source.seek(SeekFrom::Start(table_offset))?;
    // 从文件表起始复制到源文件末尾（含文件表与 string table；文件表我们已重写，
    // 但这里用源文件表字节会覆盖已写内容——必须从"我们复制到的位置"继续）
    // 修正：string table 起始 = 源文件表起始 + 全部 entry 长度
    let source_table_total: u64 = files.iter().map(|f| table_record_len(&f.name)).sum();
    let string_table_start = table_offset + source_table_total;
    copy_exact(&mut source, output, source_end - string_table_start)?;

    // 6. 更新 header 中的 file_table_offset（16 处，i64）
    output.seek(SeekFrom::Start(16))?;
    output.write_all(&(new_table_offset as i64).to_le_bytes())?;

    // 未找到的请求
    for key in replacements.keys() {
        if !found_keys.iter().any(|f| f == key) {
            not_found.push(key.clone());
        }
    }

    let output_size = output.stream_position()?;
    Ok(Ba2InjectionSummary {
        injected,
        not_found,
        output_size,
    })
}
