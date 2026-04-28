//! BA2 文件数据提取

use flate2::read::ZlibDecoder;
use std::io::{self, Read, Seek, SeekFrom};

/// 从 BA2 文件提取数据
///
/// BA2 General 格式使用 zlib 压缩
pub fn extract_file_data<R: Read + Seek>(
    reader: &mut R,
    offset: i64,
    packed_size: u32,
    uncompressed_size: u32,
    is_compressed: bool,
) -> io::Result<Vec<u8>> {
    reader.seek(SeekFrom::Start(offset as u64))?;

    if is_compressed {
        let mut compressed_data = vec![0u8; packed_size as usize];
        reader.read_exact(&mut compressed_data)?;

        let mut decoder = ZlibDecoder::new(&compressed_data[..]);
        let mut decompressed = Vec::with_capacity(uncompressed_size as usize);
        decoder.read_to_end(&mut decompressed)?;

        Ok(decompressed)
    } else {
        let mut data = vec![0u8; uncompressed_size as usize];
        reader.read_exact(&mut data)?;
        Ok(data)
    }
}
