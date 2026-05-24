use super::header::{FieldHeader, GenericHeader, GrupHeader, RecordHeaderData};
use byteorder::{LittleEndian, WriteBytesExt};
use std::io::Write;

/// ESP field — 记录内的单个子记录
///
/// 存储原始头部和数据缓冲区。对于可翻译字段，缓冲区包含字符串数据：
/// - 非本地化 ESP：内联文本
/// - 本地化 ESP：4 字节字符串 ID
///
/// 这是 ESP 记录树的最小单位，用于：
/// - 读取字符串数据
/// - 更新翻译（回写）
/// - 序列化回 ESP 文件
#[derive(Clone, Debug)]
pub struct EspField {
    /// 字段头部（签名 + 大小）
    pub header: FieldHeader,
    /// 字段数据缓冲区
    pub buffer: Vec<u8>,
    /// 是否为 XXXX 大小前缀字段（name == b"XXXX"）
    /// XXXX 字段用于指定下一个字段的实际大小（处理 65535 字节限制）
    pub is_size_xxxx: bool,
}

impl EspField {
    /// 从字节切片解析字段，按顺序返回
    ///
    /// 处理 XXXX 大小前缀字段：
    /// - 读取 4 字节值
    /// - 应用到下一个字段的有效大小
    ///
    /// XXXX 处理说明：
    /// - Bethesda 格式中，字段大小限制为 65535 字节
    /// - 超过此限制时，使用 XXXX 字段存储实际大小
    /// - XXXX 字段本身的大小为 4 字节
    pub fn parse_fields(data: &[u8]) -> std::io::Result<Vec<Self>> {
        let mut pos = 0usize;
        // 根据平均字段大小预分配（~50 字节）
        let estimated_count = (data.len() / 50).max(4);
        let mut fields = Vec::with_capacity(estimated_count);
        let mut next_explicit_size: Option<u32> = None;

        while pos < data.len() {
            if pos + 6 > data.len() {
                break;
            }

            let sig = [data[pos], data[pos + 1], data[pos + 2], data[pos + 3]];
            let dsize = u16::from_le_bytes([data[pos + 4], data[pos + 5]]) as usize;
            pos += 6;

            // 确定此字段的实际数据大小
            let effective_size = if sig == *b"XXXX" {
                // XXXX 字段：数据是 4 字节（下一个字段的大小）
                dsize
            } else if let Some(size) = next_explicit_size.take() {
                // 此字段前面有 XXXX；使用显式大小
                size as usize
            } else {
                dsize
            };

            // 检查数据是否被截断
            let remaining = data.len() - pos;
            let read_size = effective_size.min(remaining);

            let buffer = data[pos..pos + read_size].to_vec();
            pos += read_size;

            let is_size_xxxx = sig == *b"XXXX";

            // 如果这是 XXXX 字段，提取下一个字段的大小
            if is_size_xxxx && buffer.len() >= 4 {
                next_explicit_size = Some(u32::from_le_bytes([
                    buffer[0], buffer[1], buffer[2], buffer[3],
                ]));
            }

            fields.push(EspField {
                header: FieldHeader {
                    name: sig,
                    dsize: read_size as u16,
                },
                buffer,
                is_size_xxxx,
            });
        }

        Ok(fields)
    }

    /// 使用目标代码页更新此字段的缓冲区
    ///
    /// 对于非本地化 ESP：用编码后的翻译替换整个缓冲区
    /// 更新 `header.dsize` 以匹配新缓冲区长度
    ///
    /// 参数：
    /// - `text`: 新的文本内容
    /// - `codepage`: 目标代码页（用于编码）
    pub fn update_buffer(&mut self, text: &str, codepage: &crate::strings::CodepageConfig) {
        let encoded = codepage.encode(text);
        self.header.dsize = encoded.len() as u16;
        self.buffer = encoded;
    }

    /// 使用给定的代码页将字段缓冲区转换为字符串
    pub fn buffer_to_string(&self, codepage: &crate::strings::CodepageConfig) -> String {
        codepage.decode(&self.buffer)
    }

    /// 将此字段写入写入器（头部 + 缓冲区）
    pub fn write_to<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(&self.header.name)?;
        writer.write_u16::<LittleEndian>(self.header.dsize)?;
        writer.write_all(&self.buffer)?;
        Ok(())
    }

    /// 序列化后的总大小：6（头部）+ 缓冲区长度
    pub fn serialized_size(&self) -> usize {
        6 + self.buffer.len()
    }
}

/// ESP 记录 — 包含字段的单条记录（如 INFO、NPC_、CELL）
#[derive(Clone, Debug)]
pub struct EspRecord {
    pub header: GenericHeader,
    pub record_header_data: RecordHeaderData,
    pub fields: Vec<EspField>,
    /// 原始文件中此记录是否被压缩
    pub compressed: bool,
    /// 此记录是否从未解压（原始直通模式）
    pub raw: bool,
    /// 记录的 FormID
    pub form_id: u32,
    /// 记录的编辑器 ID（EDID），如果存在
    pub editor_id: Option<String>,
    /// 原始压缩数据块（用于 raw 记录直通）或重建后的压缩数据
    pub original_raw_data: Vec<u8>,
}

impl EspRecord {
    /// 重建此记录的数据（从字段列表重新构建）
    ///
    /// 遍历字段，管理 XXXX 大小前缀字段（按 Delphi 算法反向迭代），
    /// 重新计算所有 dsize 值，并可选择用 zlib 重新压缩。
    pub fn rebuild_data(&mut self) -> std::io::Result<()> {
        if self.raw {
            return Ok(()); // raw 记录直接透传，不做修改
        }

        // 第一遍：通过反向迭代处理 XXXX 字段
        self.manage_xxxx_fields();

        // 第二遍：重建连续数据缓冲区
        let estimated_size: usize = self.fields.iter().map(|f| 6 + f.buffer.len()).sum();
        let mut data = Vec::with_capacity(estimated_size);
        for field in &self.fields {
            // 写入字段头部（6 字节）+ 字段缓冲区
            data.extend_from_slice(&field.header.name);
            data.extend_from_slice(&field.header.dsize.to_le_bytes());
            data.extend_from_slice(&field.buffer);
        }

        if self.compressed {
            // 使用 zlib 压缩（RFC 1950）
            let decompressed_size = data.len() as u32;
            let compressed = compress_zlib(&data)?;
            // 格式：[4 字节解压大小 LE] + [zlib 数据]
            let mut output = Vec::with_capacity(4 + compressed.len());
            output.extend_from_slice(&decompressed_size.to_le_bytes());
            output.extend_from_slice(&compressed);
            self.header.dsize = output.len() as u32;
            // 将重建后的压缩数据存储到 original_raw_data
            self.original_raw_data = output;
        } else {
            self.header.dsize = data.len() as u32;
            // 对于未压缩的记录，不需要存储额外数据；
            // fields 向量就是数据源。
        }

        Ok(())
    }

    /// 管理 XXXX 大小前缀字段
    ///
    /// 按 Delphi 算法：反向遍历字段。如果字段的 buffer > 65535 字节，
    /// 确保其前面存在 XXXX 字段并包含正确的大小值。如果字段缩小到 65536 以下，
    /// 移除前面的 XXXX 字段。
    fn manage_xxxx_fields(&mut self) {
        let mut i = self.fields.len();
        while i > 0 {
            i -= 1;
            if self.fields[i].is_size_xxxx {
                continue;
            }

            let needs_xxxx = self.fields[i].buffer.len() > 65535;

            // 检查前面是否有 XXXX 字段
            let has_xxxx = i > 0 && self.fields[i - 1].is_size_xxxx;

            if needs_xxxx {
                let size = self.fields[i].buffer.len() as u32;
                if has_xxxx {
                    // 更新已有的 XXXX 字段
                    self.fields[i - 1].buffer = size.to_le_bytes().to_vec();
                    self.fields[i - 1].header.dsize = 4;
                } else {
                    // 在此字段前插入新的 XXXX 字段
                    let xxxx_field = EspField {
                        header: FieldHeader {
                            name: *b"XXXX",
                            dsize: 4,
                        },
                        buffer: size.to_le_bytes().to_vec(),
                        is_size_xxxx: true,
                    };
                    self.fields.insert(i, xxxx_field);
                    i += 1; // 插入后调整索引
                }
            } else if has_xxxx {
                // 字段不再需要 XXXX —— 移除它
                self.fields.remove(i - 1);
                // 不需要递减 i；位置 i-1 的字段被移除后，
                // 当前字段现在位于 i-1
                i -= 1;
            }
        }
    }

    /// 获取重建后的序列化数据
    ///
    /// 对于压缩记录，返回压缩数据块。
    /// 对于未压缩记录，从字段重新构建。
    pub fn get_serialized_data(&self) -> Vec<u8> {
        if self.raw {
            return self.original_raw_data.clone();
        }

        if self.compressed {
            return self.original_raw_data.clone();
        }

        // 未压缩：从字段构建
        let estimated_size: usize = self.fields.iter().map(|f| 6 + f.buffer.len()).sum();
        let mut data = Vec::with_capacity(estimated_size);
        for field in &self.fields {
            data.extend_from_slice(&field.header.name);
            data.extend_from_slice(&field.header.dsize.to_le_bytes());
            data.extend_from_slice(&field.buffer);
        }
        data
    }

    /// 序列化此记录到写入器
    ///
    /// 写入：GenericHeader + RecordHeaderData +（压缩数据块或逐字段序列化）。
    pub fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        // 写入 GenericHeader（类型 + dsize）
        writer.write_all(&self.header.name)?;
        writer.write_u32::<LittleEndian>(self.header.dsize)?;

        // 写入 RecordHeaderData（16 字节）
        writer.write_u32::<LittleEndian>(self.record_header_data.flags)?;
        writer.write_u32::<LittleEndian>(self.record_header_data.form_id)?;
        writer.write_u32::<LittleEndian>(self.record_header_data.version)?;
        writer.write_u16::<LittleEndian>(self.record_header_data.f_version)?;
        writer.write_u16::<LittleEndian>(self.record_header_data.v_info)?;

        // 写入数据
        if self.raw || self.compressed {
            writer.write_all(&self.original_raw_data)?;
        } else {
            for field in &self.fields {
                field.write_to(writer)?;
            }
        }

        Ok(())
    }

    /// 序列化后的总大小（头部 + 数据）
    pub fn serialized_size(&self) -> usize {
        8 + 16 + self.header.dsize as usize
    }
}

/// ESP GRUP — 包含记录和/或嵌套 GRUP 的分组记录
#[derive(Clone, Debug)]
pub struct EspGrup {
    pub header: GenericHeader,
    pub grup_header: GrupHeader,
    pub records: Vec<EspRecord>,
    pub children: Vec<EspGrup>,
}

impl EspGrup {
    /// 重新计算此 GRUP 的 dsize（基于子元素）
    ///
    /// GRUP dsize 包含其自身的 24 字节头部（GenericHeader 8B + GrupHeader 16B）。
    fn recalculate_size(&mut self) {
        let mut total: u32 = 24; // 自身头部

        for record in &self.records {
            total += record.serialized_size() as u32;
        }

        for child in &mut self.children {
            child.recalculate_size();
            total += child.header.dsize;
        }

        self.header.dsize = total;
    }

    /// 序列化后的总大小（包含自身的 24 字节头部）
    fn serialized_size(&self) -> u32 {
        let mut total: u32 = 24;
        for record in &self.records {
            total += record.serialized_size() as u32;
        }
        for child in &self.children {
            total += child.serialized_size();
        }
        total
    }

    /// 序列化此 GRUP 到写入器（递归）
    pub fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        // 写入 GenericHeader（使用计算的 dsize，而非存储的值）
        let dsize = self.serialized_size();
        writer.write_all(&self.header.name)?;
        writer.write_u32::<LittleEndian>(dsize)?;

        // 写入 GrupHeader（16 字节）
        writer.write_all(&self.grup_header.s_ident)?;
        writer.write_u32::<LittleEndian>(self.grup_header.s_type)?;
        writer.write_u16::<LittleEndian>(self.grup_header.s_tstamp)?;
        writer.write_u16::<LittleEndian>(self.grup_header.param1)?;
        writer.write_u16::<LittleEndian>(self.grup_header.param2)?;
        writer.write_u16::<LittleEndian>(self.grup_header.param3)?;

        // 序列化记录
        for record in &self.records {
            record.serialize(writer)?;
        }

        // 序列化子 GRUP
        for child in &self.children {
            child.serialize(writer)?;
        }

        Ok(())
    }
}

/// 使用 zlib（RFC 1950）压缩数据
fn compress_zlib(data: &[u8]) -> std::io::Result<Vec<u8>> {
    use flate2::write::ZlibEncoder;
    use flate2::Compression;

    // 使用快速压缩以提升性能（游戏不关心微小的体积差异）
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(data)?;
    encoder
        .finish()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}

/// TES4 头部记录（文件开头的插件主头部）
#[derive(Clone, Debug)]
pub struct Tes4Header {
    pub generic: GenericHeader,
    pub record_header_data: RecordHeaderData,
    /// 原始字段数据（直通，不做修改）
    pub field_data: Vec<u8>,
}

/// 解析后的 TES4 头部字段（HEDR、CNAM、SNAM、MAST/DATA 对）
#[derive(Clone, Debug, Default)]
pub struct Tes4HeaderInfo {
    /// HEDR: 版本号（f32）
    pub version: f32,
    /// HEDR: 记录数量
    pub num_records: u32,
    /// HEDR: 下一个可用 FormID
    pub next_object_id: u32,
    /// CNAM: 作者名称
    pub author: String,
    /// SNAM: 文件描述
    pub description: String,
    /// MAST/DATA 对：主文件名列表
    pub masters: Vec<String>,
    /// ONAM: 被覆盖的 FormID（原始字节）
    pub overridden_forms: Vec<u32>,
    /// 是否为主文件（记录头中的 ESM 标志）
    pub is_master: bool,
    /// 是否已本地化（本地化标志）
    pub is_localized: bool,
}

impl Tes4Header {
    /// 解析原始 field_data 为结构化的头部信息
    pub fn parse_fields(&self) -> Tes4HeaderInfo {
        let mut info = Tes4HeaderInfo {
            is_master: (self.record_header_data.flags & 0x00000001) != 0,
            is_localized: (self.record_header_data.flags & 0x00000080) != 0,
            ..Default::default()
        };

        let data = &self.field_data;
        let mut pos = 0usize;

        while pos + 6 <= data.len() {
            let sig = [data[pos], data[pos + 1], data[pos + 2], data[pos + 3]];
            let dsize = u16::from_le_bytes([data[pos + 4], data[pos + 5]]) as usize;
            pos += 6;

            if pos + dsize > data.len() {
                break;
            }

            let field_data = &data[pos..pos + dsize];

            match &sig {
                b"HEDR" if dsize >= 12 => {
                    info.version = f32::from_le_bytes([
                        field_data[0],
                        field_data[1],
                        field_data[2],
                        field_data[3],
                    ]);
                    info.num_records = u32::from_le_bytes([
                        field_data[4],
                        field_data[5],
                        field_data[6],
                        field_data[7],
                    ]);
                    info.next_object_id = u32::from_le_bytes([
                        field_data[8],
                        field_data[9],
                        field_data[10],
                        field_data[11],
                    ]);
                }
                b"CNAM" => {
                    info.author = read_cstring(field_data);
                }
                b"SNAM" => {
                    info.description = read_cstring(field_data);
                }
                b"MAST" => {
                    info.masters.push(read_cstring(field_data));
                }
                b"ONAM" => {
                    for chunk in field_data.chunks_exact(4) {
                        info.overridden_forms
                            .push(u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
                    }
                }
                _ => {}
            }

            pos += dsize;
        }

        info
    }
}

/// 从字节中读取 null 终止的 UTF-8 字符串
fn read_cstring(data: &[u8]) -> String {
    let end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
    String::from_utf8_lossy(&data[..end]).into_owned()
}

/// 内存中的 ESP 文件表示（用于回写）
///
/// 包含 TES4 头部和完整的记录树（顶层 GRUP）。
#[derive(Clone, Debug)]
pub struct EspFile {
    pub tes4: Tes4Header,
    pub top_level_grups: Vec<EspGrup>,
}

impl EspFile {
    /// 重建树中所有记录（重新计算大小、重新压缩）
    pub fn rebuild_all(&mut self) -> std::io::Result<()> {
        for grup in &mut self.top_level_grups {
            Self::rebuild_grup(grup)?;
        }
        Ok(())
    }

    fn rebuild_grup(grup: &mut EspGrup) -> std::io::Result<()> {
        for record in &mut grup.records {
            record.rebuild_data()?;
        }
        for child in &mut grup.children {
            Self::rebuild_grup(child)?;
        }
        grup.recalculate_size();
        Ok(())
    }

    /// 序列化整个 ESP 文件到写入器
    ///
    /// 写入：TES4 头部记录 + 所有顶层 GRUP。
    /// 注意：TES4 dsize = 仅 field_data（不包含 RecordHeaderData）。
    pub fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        // TES4 dsize = 仅 field_data 长度（不包含 RecordHeaderData）
        let tes4_dsize = self.tes4.field_data.len() as u32;

        // 写入 TES4 GenericHeader
        writer.write_all(&self.tes4.generic.name)?;
        writer.write_u32::<LittleEndian>(tes4_dsize)?;

        // 写入 TES4 RecordHeaderData（16 字节）
        writer.write_u32::<LittleEndian>(self.tes4.record_header_data.flags)?;
        writer.write_u32::<LittleEndian>(self.tes4.record_header_data.form_id)?;
        writer.write_u32::<LittleEndian>(self.tes4.record_header_data.version)?;
        writer.write_u16::<LittleEndian>(self.tes4.record_header_data.f_version)?;
        writer.write_u16::<LittleEndian>(self.tes4.record_header_data.v_info)?;

        // 写入 TES4 字段数据
        writer.write_all(&self.tes4.field_data)?;

        // 写入所有顶层 GRUP
        for grup in &self.top_level_grups {
            grup.serialize(writer)?;
        }

        Ok(())
    }

    /// 将 ESP 文件保存到磁盘（自动备份）
    ///
    /// 写入前在 `<path>.backup.<timestamp>` 创建原始文件备份，
    /// 除非 `create_backup` 为 false。
    pub fn save_to_file<P: AsRef<std::path::Path>>(
        &self,
        path: P,
        create_backup: bool,
    ) -> std::io::Result<()> {
        let path = path.as_ref();

        // 如果需要，创建备份
        if create_backup && path.exists() {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let backup_path = path.with_extension(format!("backup.{}", timestamp));
            std::fs::copy(path, &backup_path)?;
        }

        // 先重建所有记录
        let mut file = self.clone();
        file.rebuild_all()?;

        // 序列化到文件
        let mut writer = std::io::BufWriter::new(std::fs::File::create(path)?);
        file.serialize(&mut writer)?;
        writer.flush()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_fields_basic() {
        // Create a minimal field buffer: 2 fields
        let mut data = Vec::new();
        // Field 1: EDID, 5 bytes
        data.extend_from_slice(b"EDID");
        data.extend_from_slice(&5u16.to_le_bytes());
        data.extend_from_slice(b"Hello");
        // Field 2: FULL, 3 bytes
        data.extend_from_slice(b"FULL");
        data.extend_from_slice(&3u16.to_le_bytes());
        data.extend_from_slice(b"Bob");

        let fields = EspField::parse_fields(&data).unwrap();
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].header.name, *b"EDID");
        assert_eq!(fields[0].buffer, b"Hello");
        assert_eq!(fields[1].header.name, *b"FULL");
        assert_eq!(fields[1].buffer, b"Bob");
    }

    #[test]
    fn test_parse_fields_with_xxxx() {
        // XXXX field followed by a large field
        let mut data = Vec::new();
        // XXXX field
        data.extend_from_slice(b"XXXX");
        data.extend_from_slice(&4u16.to_le_bytes()); // dsize=4 for XXXX itself
        data.extend_from_slice(&70000u32.to_le_bytes()); // next field size
                                                         // Large field
        data.extend_from_slice(b"DESC");
        // dsize in header is 0 (overridden by XXXX)
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&vec![0xAA; 70000]);

        let fields = EspField::parse_fields(&data).unwrap();
        assert_eq!(fields.len(), 2);
        assert!(fields[0].is_size_xxxx);
        assert_eq!(fields[0].buffer, 70000u32.to_le_bytes());
        assert_eq!(fields[1].header.name, *b"DESC");
        assert_eq!(fields[1].buffer.len(), 70000);
    }

    #[test]
    fn test_compress_decompress_roundtrip() {
        let original = b"Hello, World! This is a test of zlib compression.";
        let compressed = compress_zlib(original).unwrap();

        // Decompress and verify
        use flate2::read::ZlibDecoder;
        let mut decoder = ZlibDecoder::new(&compressed[..]);
        let mut decompressed = Vec::new();
        std::io::Read::read_to_end(&mut decoder, &mut decompressed).unwrap();
        assert_eq!(decompressed, original);
    }

    fn make_test_record(fields: Vec<EspField>, compressed: bool) -> EspRecord {
        let data_len: usize = fields.iter().map(|f| 6 + f.buffer.len()).sum();
        EspRecord {
            header: GenericHeader {
                name: *b"NPC_",
                dsize: data_len as u32,
            },
            record_header_data: RecordHeaderData {
                flags: if compressed { 0x00040000 } else { 0 },
                form_id: 0x1234,
                version: 44,
                f_version: 15,
                v_info: 0,
            },
            fields,
            compressed,
            raw: false,
            form_id: 0x1234,
            editor_id: None,
            original_raw_data: Vec::new(),
        }
    }

    fn make_field(sig: &[u8; 4], data: &[u8]) -> EspField {
        EspField {
            header: FieldHeader {
                name: *sig,
                dsize: data.len() as u16,
            },
            buffer: data.to_vec(),
            is_size_xxxx: false,
        }
    }

    #[test]
    fn test_rebuild_no_change() {
        let fields = vec![
            make_field(b"EDID", b"TestNPC"),
            make_field(b"FULL", b"Test Name"),
        ];
        let mut record = make_test_record(fields, false);
        let original_dsize = record.header.dsize;

        record.rebuild_data().unwrap();

        // dsize should remain the same since nothing changed
        assert_eq!(record.header.dsize, original_dsize);
        assert_eq!(record.fields.len(), 2);
        assert_eq!(record.fields[0].buffer, b"TestNPC");
        assert_eq!(record.fields[1].buffer, b"Test Name");
    }

    #[test]
    fn test_rebuild_with_translation() {
        let fields = vec![
            make_field(b"EDID", b"TestNPC"),
            make_field(b"FULL", b"Hello"),
        ];
        let mut record = make_test_record(fields, false);
        let original_dsize = record.header.dsize;

        // Simulate translation: update FULL field
        record.fields[1].buffer = b"Translated Greeting in Chinese".to_vec();
        record.fields[1].header.dsize = record.fields[1].buffer.len() as u16;

        record.rebuild_data().unwrap();

        // dsize should increase
        assert!(record.header.dsize > original_dsize);
        assert_eq!(record.fields[1].buffer, b"Translated Greeting in Chinese");
    }

    #[test]
    fn test_rebuild_compressed() {
        let fields = vec![
            make_field(b"EDID", b"TestNPC"),
            make_field(b"FULL", b"Some text for compression"),
        ];
        let mut record = make_test_record(fields, true);

        record.rebuild_data().unwrap();

        // Should have compressed data in original_raw_data
        assert!(!record.original_raw_data.is_empty());

        // Verify compressed format: first 4 bytes = decompressed size LE
        assert!(record.original_raw_data.len() >= 4);
        let decompressed_size = u32::from_le_bytes([
            record.original_raw_data[0],
            record.original_raw_data[1],
            record.original_raw_data[2],
            record.original_raw_data[3],
        ]);

        // Decompress and verify
        use flate2::read::ZlibDecoder;
        let mut decoder = ZlibDecoder::new(&record.original_raw_data[4..]);
        let mut decompressed = Vec::new();
        std::io::Read::read_to_end(&mut decoder, &mut decompressed).unwrap();
        assert_eq!(decompressed.len() as u32, decompressed_size);

        // The decompressed data should contain our field data
        assert!(decompressed.windows(4).any(|w| w == b"EDID"));
        assert!(decompressed.windows(4).any(|w| w == b"FULL"));
    }

    #[test]
    fn test_rebuild_xxxx_field() {
        let fields = vec![
            make_field(b"EDID", b"TestNPC"),
            make_field(b"DESC", &vec![0xAA; 70000]), // large field > 65535
        ];
        let mut record = make_test_record(fields, false);

        assert_eq!(record.fields.len(), 2);
        assert!(!record.fields[0].is_size_xxxx);
        assert!(!record.fields[1].is_size_xxxx);

        record.rebuild_data().unwrap();

        // Should have inserted a XXXX field before DESC
        assert_eq!(record.fields.len(), 3);
        assert!(record.fields[0].is_size_xxxx || record.fields[1].is_size_xxxx);

        // Find the XXXX field and verify its value
        let xxxx_idx = record.fields.iter().position(|f| f.is_size_xxxx).unwrap();
        assert!(xxxx_idx < record.fields.len() - 1);
        let xxxx_value = u32::from_le_bytes([
            record.fields[xxxx_idx].buffer[0],
            record.fields[xxxx_idx].buffer[1],
            record.fields[xxxx_idx].buffer[2],
            record.fields[xxxx_idx].buffer[3],
        ]);
        assert_eq!(xxxx_value, 70000);

        // The DESC field should still be 70000 bytes
        let desc_idx = record
            .fields
            .iter()
            .position(|f| f.header.name == *b"DESC")
            .unwrap();
        assert_eq!(record.fields[desc_idx].buffer.len(), 70000);
    }

    #[test]
    fn test_rebuild_xxxx_remove_when_shrink() {
        // Start with a large field that needs XXXX
        let fields = vec![make_field(b"DESC", &vec![0xBB; 70000])];
        let mut record = make_test_record(fields, false);

        record.rebuild_data().unwrap();
        // XXXX should be inserted
        assert_eq!(record.fields.len(), 2);
        assert!(record.fields[0].is_size_xxxx);

        // Now shrink the field below 65536
        record.fields[1].buffer = vec![0xCC; 100];
        record.fields[1].header.dsize = 100;

        record.rebuild_data().unwrap();

        // XXXX should be removed
        assert_eq!(record.fields.len(), 1);
        assert!(!record.fields[0].is_size_xxxx);
        assert_eq!(record.fields[0].header.name, *b"DESC");
    }

    #[test]
    fn test_rebuild_raw_passthrough() {
        let raw_data = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let mut record = EspRecord {
            header: GenericHeader {
                name: *b"NPC_",
                dsize: 4,
            },
            record_header_data: RecordHeaderData {
                flags: 0,
                form_id: 0x1234,
                version: 44,
                f_version: 15,
                v_info: 0,
            },
            fields: Vec::new(),
            compressed: false,
            raw: true,
            form_id: 0x1234,
            editor_id: None,
            original_raw_data: raw_data.clone(),
        };

        record.rebuild_data().unwrap();

        // Raw records should pass through unchanged
        assert_eq!(record.original_raw_data, raw_data);
        assert_eq!(record.header.dsize, 4);
    }

    #[test]
    fn test_serialize_roundtrip() {
        use std::io::Cursor;

        // Build a minimal EspFile
        let fields = vec![
            make_field(b"EDID", b"TestNPC"),
            make_field(b"FULL", b"Hello World"),
        ];
        let record = make_test_record(fields, false);

        let grup = EspGrup {
            header: GenericHeader {
                name: *b"GRUP",
                dsize: 0,
            },
            grup_header: GrupHeader {
                s_ident: [0; 4],
                s_type: 0,
                s_tstamp: 0,
                param1: 0,
                param2: 0,
                param3: 0,
            },
            records: vec![record],
            children: Vec::new(),
        };

        let esp_file = EspFile {
            tes4: Tes4Header {
                generic: GenericHeader {
                    name: *b"TES4",
                    dsize: 0,
                },
                record_header_data: RecordHeaderData {
                    flags: 0,
                    form_id: 0,
                    version: 44,
                    f_version: 15,
                    v_info: 0,
                },
                field_data: Vec::new(),
            },
            top_level_grups: vec![grup],
        };

        // Serialize
        let mut buf = Vec::new();
        esp_file.serialize(&mut Cursor::new(&mut buf)).unwrap();

        // Verify the output starts with TES4
        assert_eq!(&buf[0..4], b"TES4");

        // Find GRUP in the output
        let mut found_grup = false;
        for i in 0..buf.len() - 3 {
            if &buf[i..i + 4] == b"GRUP" {
                found_grup = true;
                break;
            }
        }
        assert!(found_grup, "GRUP not found in serialized output");

        // Find EDID and FULL in the output
        let has_edid = buf.windows(4).any(|w| w == b"EDID");
        let has_full = buf.windows(4).any(|w| w == b"FULL");
        assert!(has_edid, "EDID not found in serialized output");
        assert!(has_full, "FULL not found in serialized output");
    }

    #[test]
    fn test_serialize_roundtrip_with_rebuild() {
        use std::io::Cursor;

        // Build record with translation
        let fields = vec![
            make_field(b"EDID", b"TestNPC"),
            make_field(b"FULL", b"Original"),
        ];
        let mut record = make_test_record(fields, false);

        // Simulate translation
        record.fields[1].buffer = b"Translated Text Here".to_vec();
        record.fields[1].header.dsize = 20;

        let mut grup = EspGrup {
            header: GenericHeader {
                name: *b"GRUP",
                dsize: 0,
            },
            grup_header: GrupHeader {
                s_ident: [0; 4],
                s_type: 0,
                s_tstamp: 0,
                param1: 0,
                param2: 0,
                param3: 0,
            },
            records: vec![record],
            children: Vec::new(),
        };

        // Rebuild the GRUP (which rebuilds records and recalculates sizes)
        for r in &mut grup.records {
            r.rebuild_data().unwrap();
        }
        grup.recalculate_size();

        // Verify GRUP dsize includes the 24-byte header
        let records_size: usize = grup.records.iter().map(|r| r.serialized_size()).sum();
        assert_eq!(grup.header.dsize as usize, 24 + records_size);

        // Serialize
        let esp_file = EspFile {
            tes4: Tes4Header {
                generic: GenericHeader {
                    name: *b"TES4",
                    dsize: 0,
                },
                record_header_data: RecordHeaderData {
                    flags: 0,
                    form_id: 0,
                    version: 44,
                    f_version: 15,
                    v_info: 0,
                },
                field_data: Vec::new(),
            },
            top_level_grups: vec![grup],
        };

        let mut buf = Vec::new();
        esp_file.serialize(&mut Cursor::new(&mut buf)).unwrap();

        // Verify the translated text appears in the output
        let has_translated = buf.windows(20).any(|w| w == b"Translated Text Here");
        assert!(
            has_translated,
            "Translated text not found in serialized output"
        );
    }

    #[test]
    fn test_grup_recalculate_size_nested() {
        // Test nested GRUP size calculation
        let inner_grup = EspGrup {
            header: GenericHeader {
                name: *b"GRUP",
                dsize: 0,
            },
            grup_header: GrupHeader {
                s_ident: [0; 4],
                s_type: 8,
                s_tstamp: 0,
                param1: 0,
                param2: 0,
                param3: 0,
            },
            records: vec![make_test_record(vec![make_field(b"EDID", b"Inner")], false)],
            children: Vec::new(),
        };

        let mut outer_grup = EspGrup {
            header: GenericHeader {
                name: *b"GRUP",
                dsize: 0,
            },
            grup_header: GrupHeader {
                s_ident: [0; 4],
                s_type: 0,
                s_tstamp: 0,
                param1: 0,
                param2: 0,
                param3: 0,
            },
            records: vec![make_test_record(vec![make_field(b"EDID", b"Outer")], false)],
            children: vec![inner_grup],
        };

        outer_grup.recalculate_size();

        // Outer GRUP dsize = 24 (own header) + record_size + inner_grup_dsize
        let outer_record_size = outer_grup.records[0].serialized_size();
        let inner_dsize = outer_grup.children[0].header.dsize;
        let expected = 24 + outer_record_size as u32 + inner_dsize;
        assert_eq!(outer_grup.header.dsize, expected);
    }
}
