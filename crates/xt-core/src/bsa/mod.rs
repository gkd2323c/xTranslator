//! BSA (Bethesda Softworks Archive) 格式解析模块
//!
//! 支持版本：
//! - Skyrim (v0x68) — zlib 压缩
//! - Skyrim Special Edition (v0x69) — LZ4 压缩
//!
//! 基于 Delphi TESVT_bsa.pas（源自 xEdit wbBSA.pas）复刻。

pub mod directory;
pub mod extraction;
pub mod header;

use directory::BsaDirectory;
use extraction::extract_file_data;
use header::BsaHeader;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// BSA 归档文件
pub struct BsaArchive {
    header: BsaHeader,
    directory: BsaDirectory,
    path: std::path::PathBuf,
}

impl BsaArchive {
    /// 打开 BSA 文件
    pub fn open<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let path = path.as_ref();
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        let header = BsaHeader::read_from(&mut reader)?;
        if !header.is_valid_version() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Unsupported BSA version: 0x{:X}", header.version),
            ));
        }

        let directory = BsaDirectory::read_from(&mut reader, &header)?;

        Ok(Self {
            header,
            directory,
            path: path.to_path_buf(),
        })
    }

    /// 按路径提取文件
    ///
    /// 路径格式：`folder/filename.ext`（如 `strings/skyrim_english.strings`）
    pub fn extract_file(&self, path: &str) -> std::io::Result<Vec<u8>> {
        let (_folder, file) = self
            .directory
            .find_by_path(path)
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("File not found in BSA: {}", path),
                )
            })?;

        let mut file_handle = File::open(&self.path)?;
        extract_file_data(
            &mut file_handle,
            &self.header,
            file.offset,
            file.raw_size,
        )
    }

    /// 检查文件是否存在
    pub fn contains_file(&self, path: &str) -> bool {
        self.directory.find_by_path(path).is_some()
    }

    /// 列出文件夹中的所有文件
    pub fn list_files_in_folder(&self, folder_name: &str) -> Vec<&str> {
        let folder_hash = directory::bsa_hash64(&folder_name.to_lowercase(), "");
        if let Some(&idx) = self.directory.folder_map.get(&folder_hash) {
            self.directory.folders[idx]
                .files
                .iter()
                .map(|f| f.name.as_str())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// 获取版本号
    pub fn version(&self) -> u32 {
        self.header.version
    }

    /// 获取文件夹数量
    pub fn folder_count(&self) -> u32 {
        self.header.folder_count
    }

    /// 获取文件数量
    pub fn file_count(&self) -> u32 {
        self.header.file_count
    }

    /// 获取所有文件夹名称
    pub fn folder_names(&self) -> Vec<&str> {
        self.directory
            .folders
            .iter()
            .filter(|f| !f.name.is_empty())
            .map(|f| f.name.as_str())
            .collect()
    }

    /// 获取归档文件名
    pub fn archive_name(&self) -> Option<&str> {
        self.path.file_name().and_then(|s| s.to_str())
    }

    /// 列出所有文件及其元数据
    pub fn list_all_files(&self) -> Vec<BsaFileEntry> {
        let mut entries = Vec::new();
        for folder in &self.directory.folders {
            for file in &folder.files {
                let is_compressed = (self.header.archive_flags & 0x0004) != 0
                    && file.raw_size > 0;
                entries.push(BsaFileEntry {
                    path: if folder.name.is_empty() {
                        file.name.clone()
                    } else {
                        format!("{}/{}", folder.name, file.name)
                    },
                    size: file.raw_size as u64,
                    compressed: is_compressed,
                    folder: folder.name.clone(),
                });
            }
        }
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        entries
    }
}

/// BSA 文件条目（用于前端展示）
#[derive(Clone, Debug)]
pub struct BsaFileEntry {
    pub path: String,
    pub size: u64,
    pub compressed: bool,
    pub folder: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn get_test_bsa_path() -> Option<PathBuf> {
        // Interface.bsa contains strings files needed for testing
        let paths = [
            r"D:\SteamLibrary\steamapps\common\Skyrim Special Edition\Data\Skyrim - Interface.bsa",
            r"C:\Program Files (x86)\Steam\steamapps\common\Skyrim Special Edition\Data\Skyrim - Interface.bsa",
        ];
        for p in &paths {
            let path = PathBuf::from(p);
            if path.exists() {
                return Some(path);
            }
        }
        None
    }

    #[test]
    fn test_hash_against_known_bsa_values() {
        use crate::bsa::directory::bsa_hash64;
        // Known hashes from Skyrim - Interface.bsa
        assert_eq!(bsa_hash64("strings", ""), 0x4DA2984373076773,
            "folder hash mismatch for 'strings'");
        // Verify skyrim_english files (all share lower 32 bits 0x730E7368)
        assert_eq!(bsa_hash64("skyrim_english", ".strings"), 0x195E35F8730E7368,
            "file hash mismatch for 'skyrim_english.strings'");
        assert_eq!(bsa_hash64("skyrim_english", ".ilstrings"), 0x0ACD17F5730E7368,
            "file hash mismatch for 'skyrim_english.ilstrings'");
        assert_eq!(bsa_hash64("skyrim_english", ".dlstrings"), 0xEB4C61F0730E7368,
            "file hash mismatch for 'skyrim_english.dlstrings'");
        // Verify skyrim_german.strings
        assert_eq!(bsa_hash64("skyrim_german", ".strings"), 0x05A6AE04730D616E,
            "file hash mismatch for 'skyrim_german.strings'");
    }

    #[test]
    fn test_find_strings_in_interface_bsa() {
        let path = r"D:\SteamLibrary\steamapps\common\Skyrim Special Edition\Data\Skyrim - Interface.bsa";
        if let Ok(bsa) = BsaArchive::open(path) {
            assert!(bsa.contains_file("strings/skyrim_english.strings"),
                "should find skyrim_english.strings in Interface.bsa");
            assert!(bsa.contains_file("strings/skyrim_english.ilstrings"),
                "should find skyrim_english.ilstrings in Interface.bsa");
        }
    }

    #[test]
    fn test_open_bsa() {
        if let Some(path) = get_test_bsa_path() {
            let bsa = BsaArchive::open(&path).unwrap();
            assert_eq!(bsa.version(), 0x69);
            assert!(bsa.folder_count() > 0);
            assert!(bsa.file_count() > 0);
            println!(
                "BSA: {} folders, {} files",
                bsa.folder_count(),
                bsa.file_count()
            );
        }
    }

    #[test]
    fn test_contains_strings_file() {
        if let Some(path) = get_test_bsa_path() {
            let bsa = BsaArchive::open(&path).unwrap();
            assert!(bsa.contains_file("strings/skyrim_english.strings"));
        }
    }

    #[test]
    fn test_extract_strings_file() {
        if let Some(path) = get_test_bsa_path() {
            let bsa = BsaArchive::open(&path).unwrap();
            let data = bsa.extract_file("strings/skyrim_english.strings").unwrap();
            // Strings 文件头：count(u32) + data_size(u32)
            assert!(data.len() >= 8);
            println!("Extracted {} bytes", data.len());
        }
    }

    #[test]
    fn test_list_strings_folder() {
        if let Some(path) = get_test_bsa_path() {
            let bsa = BsaArchive::open(&path).unwrap();
            let files = bsa.list_files_in_folder("strings");
            assert!(!files.is_empty());
            println!("Strings folder files: {:?}", files);
        }
    }
}
