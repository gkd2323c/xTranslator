use super::header::{FieldHeader, GenericHeader, GrupHeader, RecordHeaderData};
use byteorder::{LittleEndian, WriteBytesExt};
use std::io::Write;

/// ESP field — a single subrecord within a record.
///
/// Holds the raw header and data buffer. For translatable fields, the buffer
/// contains the string data (either inline text for delocalized ESPs or a
/// 4-byte string ID for localized ESPs).
#[derive(Clone, Debug)]
pub struct EspField {
    pub header: FieldHeader,
    pub buffer: Vec<u8>,
    /// Whether this field is a XXXX size-prefix field (name == b"XXXX").
    pub is_size_xxxx: bool,
}

impl EspField {
    /// Parse fields from a byte slice, returning them in order.
    /// Handles XXXX size-prefix fields by reading the 4-byte value and
    /// applying it to the next field's effective size.
    pub fn parse_fields(data: &[u8]) -> std::io::Result<Vec<Self>> {
        let mut pos = 0usize;
        // Pre-allocate based on average field size (~50 bytes)
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

            // Determine the actual data size for this field
            let effective_size = if sig == *b"XXXX" {
                // XXXX field: data is 4 bytes (the size of the NEXT field)
                dsize
            } else if let Some(size) = next_explicit_size.take() {
                // This field was preceded by a XXXX; use the explicit size
                size as usize
            } else {
                dsize
            };

            // Check for truncated data
            let remaining = data.len() - pos;
            let read_size = effective_size.min(remaining);

            let buffer = data[pos..pos + read_size].to_vec();
            pos += read_size;

            let is_size_xxxx = sig == *b"XXXX";

            // If this is a XXXX field, extract the next field size
            if is_size_xxxx && buffer.len() >= 4 {
                next_explicit_size =
                    Some(u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]));
            }

            fields.push(EspField {
                header: FieldHeader { name: sig, dsize: read_size as u16 },
                buffer,
                is_size_xxxx,
            });
        }

        Ok(fields)
    }

    /// Update this field's buffer with new text encoded in the target codepage.
    ///
    /// For delocalized ESPs: replaces the entire buffer with the encoded translation.
    /// Updates `header.dsize` to match the new buffer length.
    pub fn update_buffer(&mut self, text: &str, codepage: &crate::strings::CodepageConfig) {
        let encoded = codepage.encode(text);
        self.header.dsize = encoded.len() as u16;
        self.buffer = encoded;
    }

    /// Convert field buffer to string using the given codepage.
    pub fn buffer_to_string(&self, codepage: &crate::strings::CodepageConfig) -> String {
        codepage.decode(&self.buffer)
    }

    /// Write this field to a writer (header + buffer).
    pub fn write_to<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(&self.header.name)?;
        writer.write_u16::<LittleEndian>(self.header.dsize)?;
        writer.write_all(&self.buffer)?;
        Ok(())
    }

    /// Total serialized size: 6 (header) + buffer length.
    pub fn serialized_size(&self) -> usize {
        6 + self.buffer.len()
    }
}

/// ESP record — a single record (e.g., INFO, NPC_, CELL) containing fields.
#[derive(Clone, Debug)]
pub struct EspRecord {
    pub header: GenericHeader,
    pub record_header_data: RecordHeaderData,
    pub fields: Vec<EspField>,
    /// Whether this record was compressed in the original file.
    pub compressed: bool,
    /// Whether this record was never decompressed (raw pass-through).
    pub raw: bool,
    /// The record's FormID.
    pub form_id: u32,
    /// The record's Editor ID (EDID), if present.
    pub editor_id: Option<String>,
    /// Original compressed data blob (for raw records) or rebuilt compressed data.
    pub original_raw_data: Vec<u8>,
}

impl EspRecord {
    /// Rebuild this record's data from its field list.
    ///
    /// Walks fields, manages XXXX size prefix fields (backward iteration
    /// per Delphi algorithm), recalculates all dsize values, and optionally
    /// recompresses with zlib.
    pub fn rebuild_data(&mut self) -> std::io::Result<()> {
        if self.raw {
            return Ok(()); // raw records pass through unchanged
        }

        // First pass: handle XXXX fields via backward iteration
        self.manage_xxxx_fields();

        // Second pass: rebuild the contiguous data buffer
        let estimated_size: usize = self.fields.iter().map(|f| 6 + f.buffer.len()).sum();
        let mut data = Vec::with_capacity(estimated_size);
        for field in &self.fields {
            // Write field header (6 bytes) + field buffer
            data.extend_from_slice(&field.header.name);
            data.extend_from_slice(&field.header.dsize.to_le_bytes());
            data.extend_from_slice(&field.buffer);
        }

        if self.compressed {
            // Compress with zlib (RFC 1950)
            let decompressed_size = data.len() as u32;
            let compressed = compress_zlib(&data)?;
            // Format: [4-byte decompressed size LE] + [zlib data]
            let mut output = Vec::with_capacity(4 + compressed.len());
            output.extend_from_slice(&decompressed_size.to_le_bytes());
            output.extend_from_slice(&compressed);
            self.header.dsize = output.len() as u32;
            // Store the rebuilt compressed blob in original_raw_data
            self.original_raw_data = output;
        } else {
            self.header.dsize = data.len() as u32;
            // For uncompressed records, we don't need to store extra data;
            // the fields vector is the source of truth.
        }

        Ok(())
    }

    /// Manage XXXX size prefix fields.
    ///
    /// Per Delphi algorithm: walk backward through fields. If a field has
    /// `is_size_xxxx` and its buffer > 65535 bytes, ensure a preceding XXXX
    /// field exists with the correct size. If the field shrinks below 65536,
    /// remove the preceding XXXX field.
    fn manage_xxxx_fields(&mut self) {
        let mut i = self.fields.len();
        while i > 0 {
            i -= 1;
            if self.fields[i].is_size_xxxx {
                continue;
            }

            let needs_xxxx = self.fields[i].buffer.len() > 65535;

            // Check if there's a preceding XXXX field
            let has_xxxx = i > 0 && self.fields[i - 1].is_size_xxxx;

            if needs_xxxx {
                let size = self.fields[i].buffer.len() as u32;
                if has_xxxx {
                    // Update existing XXXX field
                    self.fields[i - 1].buffer = size.to_le_bytes().to_vec();
                    self.fields[i - 1].header.dsize = 4;
                } else {
                    // Insert new XXXX field before this field
                    let xxxx_field = EspField {
                        header: FieldHeader {
                            name: *b"XXXX",
                            dsize: 4,
                        },
                        buffer: size.to_le_bytes().to_vec(),
                        is_size_xxxx: true,
                    };
                    self.fields.insert(i, xxxx_field);
                    i += 1; // adjust index since we inserted
                }
            } else if has_xxxx {
                // Field no longer needs XXXX — remove it
                self.fields.remove(i - 1);
                // Don't decrement i; the field at i-1 was removed, so the
                // current field is now at i-1
                i -= 1;
            }
        }
    }

    /// Get the rebuilt data for serialization.
    ///
    /// For compressed records, returns the compressed blob.
    /// For uncompressed, rebuilds from fields.
    pub fn get_serialized_data(&self) -> Vec<u8> {
        if self.raw {
            return self.original_raw_data.clone();
        }

        if self.compressed {
            return self.original_raw_data.clone();
        }

        // Uncompressed: build from fields
        let estimated_size: usize = self.fields.iter().map(|f| 6 + f.buffer.len()).sum();
        let mut data = Vec::with_capacity(estimated_size);
        for field in &self.fields {
            data.extend_from_slice(&field.header.name);
            data.extend_from_slice(&field.header.dsize.to_le_bytes());
            data.extend_from_slice(&field.buffer);
        }
        data
    }

    /// Serialize this record to a writer.
    ///
    /// Writes: GenericHeader + RecordHeaderData + (compressed blob or fields sequentially).
    pub fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        // Write GenericHeader (type + dsize)
        writer.write_all(&self.header.name)?;
        writer.write_u32::<LittleEndian>(self.header.dsize)?;

        // Write RecordHeaderData (16 bytes)
        writer.write_u32::<LittleEndian>(self.record_header_data.flags)?;
        writer.write_u32::<LittleEndian>(self.record_header_data.form_id)?;
        writer.write_u32::<LittleEndian>(self.record_header_data.version)?;
        writer.write_u16::<LittleEndian>(self.record_header_data.f_version)?;
        writer.write_u16::<LittleEndian>(self.record_header_data.v_info)?;

        // Write data
        if self.raw || self.compressed {
            writer.write_all(&self.original_raw_data)?;
        } else {
            for field in &self.fields {
                field.write_to(writer)?;
            }
        }

        Ok(())
    }

    /// Total serialized size of this record (header + data).
    pub fn serialized_size(&self) -> usize {
        8 + 16 + self.header.dsize as usize
    }
}

/// ESP GRUP — a group record containing records and/or nested GRUPs.
#[derive(Clone, Debug)]
pub struct EspGrup {
    pub header: GenericHeader,
    pub grup_header: GrupHeader,
    pub records: Vec<EspRecord>,
    pub children: Vec<EspGrup>,
}

impl EspGrup {
    /// Recalculate this GRUP's dsize from its children.
    ///
    /// GRUP dsize includes its own 24-byte header (GenericHeader 8B + GrupHeader 16B).
    fn recalculate_size(&mut self) {
        let mut total: u32 = 24; // own header

        for record in &self.records {
            total += record.serialized_size() as u32;
        }

        for child in &mut self.children {
            child.recalculate_size();
            total += child.header.dsize;
        }

        self.header.dsize = total;
    }

    /// Total serialized size of this GRUP (including its own 24-byte header).
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

    /// Serialize this GRUP to a writer (recursive).
    pub fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        // Write GenericHeader with correct dsize (computed, not stored)
        let dsize = self.serialized_size();
        writer.write_all(&self.header.name)?;
        writer.write_u32::<LittleEndian>(dsize)?;

        // Write GrupHeader (16 bytes)
        writer.write_all(&self.grup_header.s_ident)?;
        writer.write_u32::<LittleEndian>(self.grup_header.s_type)?;
        writer.write_u16::<LittleEndian>(self.grup_header.s_tstamp)?;
        writer.write_u16::<LittleEndian>(self.grup_header.param1)?;
        writer.write_u16::<LittleEndian>(self.grup_header.param2)?;
        writer.write_u16::<LittleEndian>(self.grup_header.param3)?;

        // Serialize records
        for record in &self.records {
            record.serialize(writer)?;
        }

        // Serialize child GRUPs
        for child in &self.children {
            child.serialize(writer)?;
        }

        Ok(())
    }
}

/// Compress data using zlib (RFC 1950).
fn compress_zlib(data: &[u8]) -> std::io::Result<Vec<u8>> {
    use flate2::write::ZlibEncoder;
    use flate2::Compression;

    // Use fast compression for better performance (game doesn't care about minor size differences)
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(data)?;
    encoder.finish().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}

/// TES4 header record (the plugin's main header at the start of the file).
#[derive(Clone, Debug)]
pub struct Tes4Header {
    pub generic: GenericHeader,
    pub record_header_data: RecordHeaderData,
    /// Raw field data of the TES4 record (passed through unchanged).
    pub field_data: Vec<u8>,
}

/// Parsed TES4 header fields (HEDR, CNAM, SNAM, MAST/DATA pairs).
#[derive(Clone, Debug, Default)]
pub struct Tes4HeaderInfo {
    /// HEDR: version (f32)
    pub version: f32,
    /// HEDR: number of records
    pub num_records: u32,
    /// HEDR: next available FormID
    pub next_object_id: u32,
    /// CNAM: author name
    pub author: String,
    /// SNAM: file description
    pub description: String,
    /// MAST/DATA pairs: list of master file names
    pub masters: Vec<String>,
    /// ONAM: overridden FormIDs (raw bytes)
    pub overridden_forms: Vec<u32>,
    /// Whether the file is a master (ESM flag in record header)
    pub is_master: bool,
    /// Whether the file is localized (localization flag)
    pub is_localized: bool,
}

impl Tes4Header {
    /// Parse raw field_data into structured header info.
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
                        field_data[0], field_data[1], field_data[2], field_data[3],
                    ]);
                    info.num_records = u32::from_le_bytes([
                        field_data[4], field_data[5], field_data[6], field_data[7],
                    ]);
                    info.next_object_id = u32::from_le_bytes([
                        field_data[8], field_data[9], field_data[10], field_data[11],
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
                        info.overridden_forms.push(u32::from_le_bytes([
                            chunk[0], chunk[1], chunk[2], chunk[3],
                        ]));
                    }
                }
                _ => {}
            }

            pos += dsize;
        }

        info
    }
}

/// Read a null-terminated UTF-8 string from bytes.
fn read_cstring(data: &[u8]) -> String {
    let end = data.iter().position(|&b| b == 0).unwrap_or(data.len());
    String::from_utf8_lossy(&data[..end]).into_owned()
}

/// In-memory ESP file representation for write-back.
///
/// Holds the TES4 header and the full record tree (top-level GRUPs).
#[derive(Clone, Debug)]
pub struct EspFile {
    pub tes4: Tes4Header,
    pub top_level_grups: Vec<EspGrup>,
}

impl EspFile {
    /// Rebuild all records in the tree (recalculate sizes, recompress).
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

    /// Serialize the entire ESP file to a writer.
    ///
    /// Writes: TES4 header record + all top-level GRUPs.
    /// Note: TES4 dsize = field_data only (does NOT include RecordHeaderData).
    pub fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        // TES4 dsize = field_data length only (not including RecordHeaderData)
        let tes4_dsize = self.tes4.field_data.len() as u32;

        // Write TES4 GenericHeader
        writer.write_all(&self.tes4.generic.name)?;
        writer.write_u32::<LittleEndian>(tes4_dsize)?;

        // Write TES4 RecordHeaderData (16 bytes)
        writer.write_u32::<LittleEndian>(self.tes4.record_header_data.flags)?;
        writer.write_u32::<LittleEndian>(self.tes4.record_header_data.form_id)?;
        writer.write_u32::<LittleEndian>(self.tes4.record_header_data.version)?;
        writer.write_u16::<LittleEndian>(self.tes4.record_header_data.f_version)?;
        writer.write_u16::<LittleEndian>(self.tes4.record_header_data.v_info)?;

        // Write TES4 field data
        writer.write_all(&self.tes4.field_data)?;

        // Write all top-level GRUPs
        for grup in &self.top_level_grups {
            grup.serialize(writer)?;
        }

        Ok(())
    }

    /// Save the ESP file to disk with automatic backup.
    ///
    /// Creates a backup of the original file at `<path>.backup.<timestamp>`
    /// before writing, unless `create_backup` is false.
    pub fn save_to_file<P: AsRef<std::path::Path>>(
        &self,
        path: P,
        create_backup: bool,
    ) -> std::io::Result<()> {
        let path = path.as_ref();

        // Create backup if requested
        if create_backup && path.exists() {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let backup_path =
                path.with_extension(format!("backup.{}", timestamp));
            std::fs::copy(path, &backup_path)?;
        }

        // Rebuild all records first
        let mut file = self.clone();
        file.rebuild_all()?;

        // Serialize to file
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
            header: GenericHeader { name: *b"NPC_", dsize: data_len as u32 },
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
            header: FieldHeader { name: *sig, dsize: data.len() as u16 },
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
        let desc_idx = record.fields.iter().position(|f| f.header.name == *b"DESC").unwrap();
        assert_eq!(record.fields[desc_idx].buffer.len(), 70000);
    }

    #[test]
    fn test_rebuild_xxxx_remove_when_shrink() {
        // Start with a large field that needs XXXX
        let fields = vec![
            make_field(b"DESC", &vec![0xBB; 70000]),
        ];
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
            header: GenericHeader { name: *b"NPC_", dsize: 4 },
            record_header_data: RecordHeaderData {
                flags: 0, form_id: 0x1234, version: 44, f_version: 15, v_info: 0,
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
            header: GenericHeader { name: *b"GRUP", dsize: 0 },
            grup_header: GrupHeader {
                s_ident: [0; 4], s_type: 0, s_tstamp: 0,
                param1: 0, param2: 0, param3: 0,
            },
            records: vec![record],
            children: Vec::new(),
        };

        let esp_file = EspFile {
            tes4: Tes4Header {
                generic: GenericHeader { name: *b"TES4", dsize: 0 },
                record_header_data: RecordHeaderData {
                    flags: 0, form_id: 0, version: 44, f_version: 15, v_info: 0,
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
            header: GenericHeader { name: *b"GRUP", dsize: 0 },
            grup_header: GrupHeader {
                s_ident: [0; 4], s_type: 0, s_tstamp: 0,
                param1: 0, param2: 0, param3: 0,
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
                generic: GenericHeader { name: *b"TES4", dsize: 0 },
                record_header_data: RecordHeaderData {
                    flags: 0, form_id: 0, version: 44, f_version: 15, v_info: 0,
                },
                field_data: Vec::new(),
            },
            top_level_grups: vec![grup],
        };

        let mut buf = Vec::new();
        esp_file.serialize(&mut Cursor::new(&mut buf)).unwrap();

        // Verify the translated text appears in the output
        let has_translated = buf.windows(20).any(|w| w == b"Translated Text Here");
        assert!(has_translated, "Translated text not found in serialized output");
    }

    #[test]
    fn test_grup_recalculate_size_nested() {
        // Test nested GRUP size calculation
        let inner_grup = EspGrup {
            header: GenericHeader { name: *b"GRUP", dsize: 0 },
            grup_header: GrupHeader {
                s_ident: [0; 4], s_type: 8, s_tstamp: 0,
                param1: 0, param2: 0, param3: 0,
            },
            records: vec![make_test_record(vec![make_field(b"EDID", b"Inner")], false)],
            children: Vec::new(),
        };

        let mut outer_grup = EspGrup {
            header: GenericHeader { name: *b"GRUP", dsize: 0 },
            grup_header: GrupHeader {
                s_ident: [0; 4], s_type: 0, s_tstamp: 0,
                param1: 0, param2: 0, param3: 0,
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
