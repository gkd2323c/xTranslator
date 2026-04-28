//! BA2 目录和文件记录结构

use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{self, Read};

/// BA2 文件记录（GNRL 类型，36 字节）
#[derive(Debug, Clone)]
pub struct Ba2FileRecord {
    /// 文件完整路径（UTF-16 LE）
    pub name: String,
    /// 文件名哈希
    pub name_hash: u32,
    /// 扩展名哈希
    pub ext_hash: u32,
    /// 目录哈希
    pub dir_hash: u32,
    /// 未知字段
    pub unk_0c: u32,
    /// 数据偏移
    pub offset: i64,
    /// 压缩后大小
    pub packed_size: u32,
    /// 解压缩后大小
    pub size: u32,
    /// 标记
    pub flag: u32,
    /// 是否压缩
    pub is_packed: bool,
}

impl Ba2FileRecord {
    /// 从读取器读取文件记录
    pub fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let name_hash = reader.read_u32::<LittleEndian>()?;
        let ext_hash = reader.read_u32::<LittleEndian>()?;
        let dir_hash = reader.read_u32::<LittleEndian>()?;
        let unk_0c = reader.read_u32::<LittleEndian>()?;
        let offset = reader.read_i64::<LittleEndian>()?;
        let packed_size = reader.read_u32::<LittleEndian>()?;
        let size = reader.read_u32::<LittleEndian>()?;
        let flag = reader.read_u32::<LittleEndian>()?;

        Ok(Self {
            name: String::new(),
            name_hash,
            ext_hash,
            dir_hash,
            unk_0c,
            offset,
            packed_size,
            size,
            flag,
            is_packed: packed_size > 0,
        })
    }

    /// 从读取器读取文件名（UTF-16 LE 带长度前缀）
    pub fn read_name<R: Read>(reader: &mut R) -> io::Result<String> {
        let name_len = reader.read_u8()?;
        if name_len == 0 {
            return Ok(String::new());
        }

        let mut chars = Vec::with_capacity(name_len as usize);
        for _ in 0..name_len {
            chars.push(reader.read_u16::<LittleEndian>()?);
        }

        reader.read_u16::<LittleEndian>()?; // 跳过终止 null

        String::from_utf16(&chars).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid UTF-16 in BA2 filename: {}", e),
            )
        })
    }
}

/// BA2 文件夹条目
#[derive(Debug, Clone)]
pub struct Ba2FolderEntry {
    /// 文件夹路径
    pub path: String,
    /// 文件夹哈希
    pub hash: u32,
    /// 文件索引列表
    pub file_indices: Vec<usize>,
}

/// BA2 文件条目 DTO（用于前端展示）
#[derive(Clone, Debug)]
pub struct Ba2FileEntryDto {
    pub path: String,
    pub size: u64,
    pub compressed: bool,
    pub folder: String,
}
