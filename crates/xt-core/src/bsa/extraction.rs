use crate::bsa::header::{BsaHeader, BSAARCHIVE_PREFIXFULLFILENAMES, BSAHEADER_VERSION_SSE};
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Read, Result, Seek, SeekFrom};

/// 从 BSA 中提取文件数据
///
/// # 参数
/// * `reader` - BSA 文件读取器
/// * `header` - BSA 头部
/// * `offset` - 文件数据偏移
/// * `raw_size` - 原始大小（含压缩标志）
pub fn extract_file_data<R: Read + Seek>(
    reader: &mut R,
    header: &BsaHeader,
    offset: u32,
    raw_size: u32,
) -> Result<Vec<u8>> {
    let (is_compressed, mut size) = header.is_file_compressed(raw_size);

    reader.seek(SeekFrom::Start(offset as u64))?;

    // SSE 前缀文件名处理
    if header.version >= BSAHEADER_VERSION_SSE
        && (header.archive_flags & BSAARCHIVE_PREFIXFULLFILENAMES) != 0
    {
        // 读取文件名长度（不含终止符）
        let name_len = reader.read_u8()? as u32;
        let mut name_buf = vec![0u8; name_len as usize];
        reader.read_exact(&mut name_buf)?;
        // 跳过终止符
        reader.read_u8()?;
        size = size.saturating_sub(name_len + 1);
    }

    if is_compressed {
        // 压缩数据：前 4 字节是解压后大小
        let decompressed_size = reader.read_u32::<LittleEndian>()?;
        size = size.saturating_sub(4);

        let mut compressed = vec![0u8; size as usize];
        reader.read_exact(&mut compressed)?;

        if header.version == BSAHEADER_VERSION_SSE {
            // SSE 使用 LZ4
            decompress_lz4(&compressed, decompressed_size as usize)
        } else {
            // Skyrim 使用 zlib
            decompress_zlib(&compressed, decompressed_size as usize)
        }
    } else {
        let mut data = vec![0u8; size as usize];
        reader.read_exact(&mut data)?;
        Ok(data)
    }
}

/// LZ4 解压（SSE BSA）
fn decompress_lz4(compressed: &[u8], expected_size: usize) -> Result<Vec<u8>> {
    match lz4::block::decompress(compressed, Some(expected_size as i32)) {
        Ok(data) => Ok(data),
        Err(e) => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("LZ4 decompression failed: {}", e),
        )),
    }
}

/// zlib 解压（Skyrim BSA）
fn decompress_zlib(compressed: &[u8], expected_size: usize) -> Result<Vec<u8>> {
    use flate2::read::ZlibDecoder;
    use std::io::Read;

    let mut decoder = ZlibDecoder::new(compressed);
    let mut result = vec![0u8; expected_size];
    match decoder.read_exact(&mut result) {
        Ok(()) => Ok(result),
        Err(e) => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Zlib decompression failed: {}", e),
        )),
    }
}
