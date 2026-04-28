//! BA2 (Bethesda Archive 2) 格式解析模块
//!
//! 支持版本：
//! - Fallout 4 (v0x01)
//! - Starfield (v0x02)
//! - Fallout 4 B (v0x08)
//!
//! 支持类型：
//! - GNRL (General) - 通用文件归档
//!
//! 基于 Delphi TESVT_bsa.pas（源自 xEdit wbBSA.pas）复刻。

pub mod directory;
pub mod extraction;
pub mod header;

use directory::{Ba2FileEntryDto, Ba2FileRecord, Ba2FolderEntry};
use extraction::extract_file_data;
use header::Ba2Header;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, Seek, SeekFrom};
use std::path::Path;

/// BA2 归档文件
pub struct Ba2Archive {
    header: Ba2Header,
    files: Vec<Ba2FileRecord>,
    folder_map: HashMap<String, Ba2FolderEntry>,
    path: std::path::PathBuf,
}

impl Ba2Archive {
    /// 打开 BA2 文件
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let path = path.as_ref();
        let mut file = BufReader::new(File::open(path)?);

        let header = Ba2Header::read_from(&mut file)?;
        if !header.is_valid_version() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unsupported BA2 version: 0x{:X}", header.version),
            ));
        }

        if !header.is_general_type() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Unsupported BA2 type: {:?} (only GNRL supported)",
                    std::str::from_utf8(&header.archive_type).unwrap_or("invalid")
                ),
            ));
        }

        file.seek(SeekFrom::Start(header.file_table_offset as u64))?;
        let mut files = Vec::with_capacity(header.file_count as usize);
        for _ in 0..header.file_count {
            let name = Ba2FileRecord::read_name(&mut file)?;
            let mut record = Ba2FileRecord::read_from(&mut file)?;
            record.name = name;
            files.push(record);
        }

        let mut folder_map: HashMap<String, Ba2FolderEntry> = HashMap::new();
        for (idx, file) in files.iter().enumerate() {
            let (folder, _filename) = Self::split_path(&file.name);
            let folder_entry = folder_map
                .entry(folder.clone())
                .or_insert_with(|| Ba2FolderEntry {
                    path: folder,
                    hash: file.dir_hash,
                    file_indices: Vec::new(),
                });
            folder_entry.file_indices.push(idx);
        }

        Ok(Self {
            header,
            files,
            folder_map,
            path: path.to_path_buf(),
        })
    }

    /// 分割路径为文件夹和文件名
    fn split_path(path: &str) -> (String, String) {
        if let Some(pos) = path.rfind('\\') {
            (
                path[..pos].to_lowercase(),
                path[pos + 1..].to_lowercase(),
            )
        } else {
            (String::new(), path.to_lowercase())
        }
    }

    /// 按路径提取文件
    pub fn extract_file(&self, path: &str) -> io::Result<Vec<u8>> {
        let normalized_path = path.replace('/', "\\");
        let (folder, filename) = Self::split_path(&normalized_path);

        if let Some(folder_entry) = self.folder_map.get(&folder) {
            for &idx in &folder_entry.file_indices {
                if self.files[idx].name == filename {
                    return self.extract_file_by_index(idx);
                }
            }
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("File not found in BA2: {}", path),
        ))
    }

    /// 按索引提取文件
    fn extract_file_by_index(&self, index: usize) -> io::Result<Vec<u8>> {
        let file = &self.files[index];
        let mut file_handle = File::open(&self.path)?;

        extract_file_data(
            &mut file_handle,
            file.offset,
            file.packed_size,
            file.size,
            file.is_packed,
        )
    }

    /// 检查文件是否存在
    pub fn contains_file(&self, path: &str) -> bool {
        let normalized_path = path.replace('/', "\\");
        let (folder, filename) = Self::split_path(&normalized_path);

        if let Some(folder_entry) = self.folder_map.get(&folder) {
            for &idx in &folder_entry.file_indices {
                if self.files[idx].name == filename {
                    return true;
                }
            }
        }
        false
    }

    /// 列出文件夹中的所有文件
    pub fn list_files_in_folder(&self, folder_name: &str) -> Vec<&str> {
        let folder_lower = folder_name.to_lowercase();
        if let Some(folder_entry) = self.folder_map.get(&folder_lower) {
            folder_entry
                .file_indices
                .iter()
                .map(|&idx| self.files[idx].name.as_str())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// 获取版本号
    pub fn version(&self) -> u32 {
        self.header.version
    }

    /// 获取文件数量
    pub fn file_count(&self) -> u32 {
        self.header.file_count
    }

    /// 获取所有文件夹名称
    pub fn folder_names(&self) -> Vec<&str> {
        let mut folders: Vec<&str> = self.folder_map.keys().map(|s| s.as_str()).collect();
        folders.sort();
        folders
    }

    /// 获取归档文件名
    pub fn archive_name(&self) -> Option<&str> {
        self.path.file_name().and_then(|s| s.to_str())
    }

    /// 列出所有文件及其元数据
    pub fn list_all_files(&self) -> Vec<Ba2FileEntryDto> {
        let mut entries = Vec::new();
        for file in &self.files {
            let (folder, _filename) = Self::split_path(&file.name);
            entries.push(Ba2FileEntryDto {
                path: file.name.replace('\\', "/"),
                size: file.size as u64,
                compressed: file.is_packed,
                folder,
            });
        }
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_path() {
        let (folder, file) = Ba2Archive::split_path("Strings\\Skyrim_English.strings");
        assert_eq!(folder, "strings");
        assert_eq!(file, "skyrim_english.strings");

        let (folder, file) = Ba2Archive::split_path("filename.txt");
        assert_eq!(folder, "");
        assert_eq!(file, "filename.txt");
    }
}
