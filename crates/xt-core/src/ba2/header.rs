//! BA2 (Bethesda Archive 2) 格式解析 - 文件头
//!
//! 支持版本：
//! - Fallout 4 (v0x01)
//! - Starfield (v0x02)
//! - Fallout 4 B (v0x08)
//!
//! 基于 Delphi TESVT_bsa.pas（源自 xEdit wbBSA.pas）复刻。

use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{self, Read};

/// BA2 文件头魔术：'BTDX'
pub const BA2_MAGIC: [u8; 4] = *b"BTDX";

/// BA2 归档类型：'GNRL'
pub const BA2_TYPE_GNRL: [u8; 4] = *b"GNRL";

/// BA2 版本：Fallout 4
pub const BA2_VERSION_FO4: u32 = 0x01;

/// BA2 版本：Starfield
pub const BA2_VERSION_SF: u32 = 0x02;

/// BA2 版本：Fallout 4 B
pub const BA2_VERSION_FO4B: u32 = 0x08;

/// BA2 文件头
#[derive(Debug, Clone)]
pub struct Ba2Header {
    /// 版本号
    pub version: u32,
    /// 归档类型（'GNRL' 或 'DX10'）
    pub archive_type: [u8; 4],
    /// 文件数量
    pub file_count: u32,
    /// 文件表偏移
    pub file_table_offset: i64,
}

impl Ba2Header {
    /// 从读取器读取文件头
    pub fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        if magic != BA2_MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid BA2 magic: expected 'BTDX', got {:?}", magic),
            ));
        }

        let version = reader.read_u32::<LittleEndian>()?;

        if ![BA2_VERSION_FO4, BA2_VERSION_SF, BA2_VERSION_FO4B].contains(&version) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unsupported BA2 version: 0x{:X}", version),
            ));
        }

        let mut archive_type = [0u8; 4];
        reader.read_exact(&mut archive_type)?;

        let file_count = reader.read_u32::<LittleEndian>()?;
        let file_table_offset = reader.read_i64::<LittleEndian>()?;

        if version == BA2_VERSION_SF {
            reader.read_u32::<LittleEndian>()?;
            reader.read_u32::<LittleEndian>()?;
        }

        Ok(Self {
            version,
            archive_type,
            file_count,
            file_table_offset,
        })
    }

    /// 检查是否为支持的版本
    pub fn is_valid_version(&self) -> bool {
        [BA2_VERSION_FO4, BA2_VERSION_SF, BA2_VERSION_FO4B].contains(&self.version)
    }

    /// 检查是否为 General 类型
    pub fn is_general_type(&self) -> bool {
        self.archive_type == BA2_TYPE_GNRL
    }
}
