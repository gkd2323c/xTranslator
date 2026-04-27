use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Read, Result, Seek};

/// BSA 文件头
#[derive(Clone, Debug)]
pub struct BsaHeader {
    pub version: u32,
    pub header_size: u32,
    pub archive_flags: u32,
    pub folder_count: u32,
    pub file_count: u32,
    pub total_folder_name_length: u32,
    pub total_file_name_length: u32,
    pub file_flags: u32,
}

/// BSA 版本常量
pub const BSAHEADER_VERSION_OB: u32 = 0x67; // Oblivion
pub const BSAHEADER_VERSION_SK: u32 = 0x68; // Skyrim/Fallout3
pub const BSAHEADER_VERSION_SSE: u32 = 0x69; // Skyrim Special Edition

/// ArchiveFlags 关键位
pub const BSAARCHIVE_COMPRESSFILES: u32 = 0x0004;
pub const BSAARCHIVE_PREFIXFULLFILENAMES: u32 = 0x0100;

/// File 压缩标志
pub const BSAFILE_COMPRESS: u32 = 0x40000000;

impl BsaHeader {
    pub fn read_from<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        if &magic != b"BSA\0" {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid BSA magic",
            ));
        }

        let version = reader.read_u32::<LittleEndian>()?;
        let header_size = reader.read_u32::<LittleEndian>()?;
        let archive_flags = reader.read_u32::<LittleEndian>()?;
        let folder_count = reader.read_u32::<LittleEndian>()?;
        let file_count = reader.read_u32::<LittleEndian>()?;
        let total_folder_name_length = reader.read_u32::<LittleEndian>()?;
        let total_file_name_length = reader.read_u32::<LittleEndian>()?;
        let file_flags = reader.read_u32::<LittleEndian>()?;

        Ok(Self {
            version,
            header_size,
            archive_flags,
            folder_count,
            file_count,
            total_folder_name_length,
            total_file_name_length,
            file_flags,
        })
    }

    pub fn is_valid_version(&self) -> bool {
        matches!(self.version, BSAHEADER_VERSION_SK | BSAHEADER_VERSION_SSE)
    }

    pub fn folder_offset_size(&self) -> usize {
        if self.version == BSAHEADER_VERSION_SSE {
            24 // Hash(8) + FileCount(4) + Unk32(4) + Offset(8)
        } else {
            16 // Hash(8) + FileCount(4) + Offset(4)
        }
    }

    pub fn has_prefix_full_filenames(&self) -> bool {
        (self.archive_flags & BSAARCHIVE_PREFIXFULLFILENAMES) != 0
    }

    pub fn is_file_compressed(&self, raw_size: u32) -> (bool, u32) {
        let mut size = raw_size;
        let mut compressed = (size & BSAFILE_COMPRESS) != 0;
        size &= !BSAFILE_COMPRESS;
        if (self.archive_flags & BSAARCHIVE_COMPRESSFILES) != 0 {
            compressed = !compressed;
        }
        (compressed, size)
    }
}
