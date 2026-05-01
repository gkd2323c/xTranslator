use super::header::{FieldHeader, GenericHeader, GrupHeader, RecordHeaderData};
use byteorder::{LittleEndian, WriteBytesExt};
use std::io::{Cursor, Read, Write};

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
        let mut cursor = Cursor::new(data);
        let mut fields = Vec::new();
        let mut next_explicit_size: Option<u32> = None;

        while (cursor.position() as usize) < data.len() {
            let pos = cursor.position() as usize;
            if pos + 6 > data.len() {
                break;
            }

            let field_header = FieldHeader::read_from(&mut cursor)?;

            // Determine the actual data size for this field
            let effective_size = if field_header.is_xxxx() {
                // XXXX field: data is 4 bytes (the size of the NEXT field)
                field_header.dsize as usize
            } else if let Some(size) = next_explicit_size.take() {
                // This field was preceded by a XXXX; use the explicit size
                size as usize
            } else {
                field_header.dsize as usize
            };

            // Check for truncated data
            let remaining = data.len() - cursor.position() as usize;
            let read_size = effective_size.min(remaining);

            let mut buffer = vec![0u8; read_size];
            cursor.read_exact(&mut buffer)?;

            let is_size_xxxx = field_header.is_xxxx();

            // If this is a XXXX field, extract the next field size
            if is_size_xxxx && buffer.len() >= 4 {
                next_explicit_size =
                    Some(u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]));
            }

            fields.push(EspField {
                header: field_header,
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
    pub fn rebuild_data(&mut self) {
        if self.raw {
            return; // raw records pass through unchanged
        }

        // First pass: handle XXXX fields via backward iteration
        self.manage_xxxx_fields();

        // Second pass: rebuild the contiguous data buffer
        let mut data = Vec::new();
        for field in &self.fields {
            // Write field header (6 bytes) + field buffer
            data.extend_from_slice(&field.header.name);
            data.extend_from_slice(&field.header.dsize.to_le_bytes());
            data.extend_from_slice(&field.buffer);
        }

        if self.compressed {
            // Compress with zlib (RFC 1950)
            let decompressed_size = data.len() as u32;
            let compressed = compress_zlib(&data);
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
        let mut data = Vec::new();
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

    /// Serialize this GRUP to a writer (recursive).
    pub fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        // Write GenericHeader
        writer.write_all(&self.header.name)?;
        writer.write_u32::<LittleEndian>(self.header.dsize)?;

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
fn compress_zlib(data: &[u8]) -> Vec<u8> {
    use flate2::write::ZlibEncoder;
    use flate2::Compression;

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(data).expect("zlib compression failed");
    encoder.finish().expect("zlib compression finish failed")
}

/// TES4 header record (the plugin's main header at the start of the file).
#[derive(Clone, Debug)]
pub struct Tes4Header {
    pub generic: GenericHeader,
    pub record_header_data: RecordHeaderData,
    /// Raw field data of the TES4 record (passed through unchanged).
    pub field_data: Vec<u8>,
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
    pub fn rebuild_all(&mut self) {
        for grup in &mut self.top_level_grups {
            Self::rebuild_grup(grup);
        }
    }

    fn rebuild_grup(grup: &mut EspGrup) {
        for record in &mut grup.records {
            record.rebuild_data();
        }
        for child in &mut grup.children {
            Self::rebuild_grup(child);
        }
        grup.recalculate_size();
    }

    /// Serialize the entire ESP file to a writer.
    ///
    /// Writes: TES4 header record + all top-level GRUPs.
    pub fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        // Write TES4 GenericHeader
        writer.write_all(&self.tes4.generic.name)?;
        writer.write_u32::<LittleEndian>(self.tes4.generic.dsize)?;

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
        file.rebuild_all();

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
        let compressed = compress_zlib(original);

        // Decompress and verify
        use flate2::read::ZlibDecoder;
        let mut decoder = ZlibDecoder::new(&compressed[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed).unwrap();
        assert_eq!(decompressed, original);
    }
}
