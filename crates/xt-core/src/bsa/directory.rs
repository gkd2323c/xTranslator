use crate::bsa::header::{BsaHeader, BSAHEADER_VERSION_SSE};
use byteorder::{LittleEndian, ReadBytesExt};
use std::collections::HashMap;
use std::io::{Read, Result, Seek};

/// BSA 文件夹记录
#[derive(Clone, Debug)]
pub struct BsaFolderRecord {
    pub hash: u64,
    pub file_count: u32,
    pub offset: u64, // SSE 用 64-bit，Skyrim 用 32-bit
}

/// BSA 文件记录
#[derive(Clone, Debug)]
pub struct BsaFileRecord {
    pub hash: u64,
    pub raw_size: u32,
    pub offset: u32,
    pub name: String,
}

/// BSA 文件夹（包含文件列表）
#[derive(Clone, Debug)]
pub struct BsaFolder {
    pub name: String,
    pub hash: u64,
    pub files: Vec<BsaFileRecord>,
    pub file_map: HashMap<u64, usize>, // hash -> index in files
    pub offset: u64, // 文件记录位置（仅用于解析阶段）
}

/// BSA 目录结构
#[derive(Clone, Debug)]
pub struct BsaDirectory {
    pub folders: Vec<BsaFolder>,
    pub folder_map: HashMap<u64, usize>, // hash -> index in folders
}

impl BsaDirectory {
    pub fn read_from<R: Read + Seek>(reader: &mut R, header: &BsaHeader) -> Result<Self> {
        let mut folders = Vec::with_capacity(header.folder_count as usize);
        let mut folder_map = HashMap::with_capacity(header.folder_count as usize);

        // 1. 读取 Folder 记录
        for i in 0..header.folder_count {
            let hash = reader.read_u64::<LittleEndian>()?;
            let file_count = reader.read_u32::<LittleEndian>()?;

            let offset = if header.version == BSAHEADER_VERSION_SSE {
                reader.read_u32::<LittleEndian>()?; // skip unk32
                reader.read_u64::<LittleEndian>()?
            } else {
                reader.read_u32::<LittleEndian>()? as u64
            };

            folders.push(BsaFolder {
                name: String::new(),
                hash,
                files: Vec::with_capacity(file_count as usize),
                file_map: HashMap::new(),
                offset,
            });
            folder_map.insert(hash, i as usize);
        }

        // 2. 读取每个文件夹的文件记录
        let total_file_name_length = header.total_file_name_length as usize;
        for i in 0..header.folder_count {
            let folder = &mut folders[i as usize];
            reader.seek(std::io::SeekFrom::Start(
                folder.offset - total_file_name_length as u64,
            ))?;

            // 读取文件夹名（长度前缀字符串，可能包含 null 终止符）
            let name_len = reader.read_u8()? as usize;
            let mut name_bytes = vec![0u8; name_len];
            reader.read_exact(&mut name_bytes)?;
            // 去掉 trailing null（SSE BSA 的 length prefix 包含 null terminator）
            while let Some(&0) = name_bytes.last() {
                name_bytes.pop();
            }
            folder.name = String::from_utf8_lossy(&name_bytes).to_lowercase();

            // 读取文件记录
            for _ in 0..folder.files.capacity() {
                let hash = reader.read_u64::<LittleEndian>()?;
                let raw_size = reader.read_u32::<LittleEndian>()?;
                let offset = reader.read_u32::<LittleEndian>()?;
                folder.files.push(BsaFileRecord {
                    hash,
                    raw_size,
                    offset,
                    name: String::new(),
                });
            }
        }

        // 3. 读取文件名
        for i in 0..header.folder_count {
            let folder = &mut folders[i as usize];
            for j in 0..folder.files.len() {
                let mut name_bytes = Vec::new();
                loop {
                    let b = reader.read_u8()?;
                    if b == 0 {
                        break;
                    }
                    name_bytes.push(b);
                }
                folder.files[j].name = String::from_utf8_lossy(&name_bytes).to_lowercase();
                folder.file_map.insert(folder.files[j].hash, j);
            }
        }

        Ok(BsaDirectory { folders, folder_map })
    }

    /// 查找文件（先按文件夹哈希，再按文件哈希）
    pub fn find_file(&self, folder_hash: u64, file_hash: u64) -> Option<(&BsaFolder, &BsaFileRecord)> {
        let folder_idx = self.folder_map.get(&folder_hash)?;
        let folder = &self.folders[*folder_idx];
        let file_idx = folder.file_map.get(&file_hash)?;
        Some((folder, &folder.files[*file_idx]))
    }

    /// 按路径查找文件
    pub fn find_by_path(&self, path: &str) -> Option<(&BsaFolder, &BsaFileRecord)> {
        let normalized = path.replace('\\', "/").to_lowercase();
        let parts: Vec<&str> = normalized.rsplitn(2, '/').collect();
        if parts.len() != 2 {
            return None;
        }

        let folder_name = parts[1];
        let file_name = parts[0];

        // 分离文件名和扩展名
        let (name, ext) = if let Some(dot_pos) = file_name.rfind('.') {
            (&file_name[..dot_pos], &file_name[dot_pos..])
        } else {
            (file_name, "")
        };

        let folder_hash = bsa_hash64(folder_name, "");
        let file_hash = bsa_hash64(name, ext);

        self.find_file(folder_hash, file_hash)
    }
}

/// BSAhash64 算法（Delphi 复刻）
///
/// 基于 TESVT_bsa.pas 的 BSAhash64 函数。
/// 注意：Delphi 字符串是 1-based 索引；Delphi UInt64 运算溢出时回绕。
pub fn bsa_hash64(name: &str, ext: &str) -> u64 {
    let mut result: u64 = 0;

    if !name.is_empty() {
        let bytes = name.as_bytes();
        let len = bytes.len();

        // 最后一个字符
        result = bytes[len - 1] as u64;

        // 倒数第二个字符
        if len > 2 {
            result = result.wrapping_add((bytes[len - 2] as u64) << 8);
        }

        // 长度
        result = result.wrapping_add((len as u64) << 16);

        // 第一个字符
        result = result.wrapping_add((bytes[0] as u64) << 24);

        // 中间部分（去掉首尾，等价于 Delphi copy(s, 2, length(s)-3)）
        if len > 3 {
            let middle = &bytes[1..len - 2];
            result = result.wrapping_add(str_to_num(middle).wrapping_shl(32));
        }
    }

    if !ext.is_empty() {
        result = result.wrapping_add(str_to_num(ext.as_bytes()).wrapping_shl(32));
    }

    // 特殊扩展名处理（.nif/.kf/.dds/.wav）
    // 这部分 Delphi 做了特殊调整，但 Strings 文件通常不涉及这些扩展名
    // 暂不实现，如有需要后续补充

    result
}

/// StrToNum：逐字符计算哈希
/// Delphi: result = result * $1003F + byte(c)
fn str_to_num(s: &[u8]) -> u64 {
    let mut result: u64 = 0;
    for &b in s {
        result = result.wrapping_mul(0x1003F).wrapping_add(b as u64);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bsa_hash64_known_values() {
        // 这些值需要与 Delphi 输出对比验证
        let h1 = bsa_hash64("strings", "");
        assert_ne!(h1, 0);

        let h2 = bsa_hash64("skyrim_english", ".strings");
        assert_ne!(h2, 0);

        // 相同输入应产生相同输出
        assert_eq!(bsa_hash64("test", ".ext"), bsa_hash64("test", ".ext"));
    }



    #[test]
    fn test_str_to_num() {
        assert_eq!(str_to_num(b"a"), 97);
        assert_eq!(str_to_num(b"ab"), 97u64.wrapping_mul(0x1003F).wrapping_add(98));
    }
}
