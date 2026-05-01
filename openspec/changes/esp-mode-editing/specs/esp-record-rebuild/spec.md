## ADDED Requirements

### Requirement: Field buffer mutation
The system SHALL support writing translated text into an ESP field's buffer using the target codepage. After writing, the field's `dsize` in its header SHALL reflect the new buffer length.

#### Scenario: Write ASCII translation into field buffer
- **WHEN** a field has source text "Hello" and translation "你好"
- **AND** the target codepage is UTF-8
- **THEN** the field's buffer is replaced with the UTF-8 encoding of "你好"
- **AND** the field header `dsize` equals the new byte length

#### Scenario: Write CJK translation with GBK codepage
- **WHEN** the target codepage is CP936 (GBK)
- **THEN** the field buffer is encoded with GBK
- **AND** `dsize` reflects the GBK byte length

### Requirement: Record data rebuild
The system SHALL rebuild a record's data block from its field list, recalculating all field header `dsize` values and the record header `dsize` as the sum of all field headers + field buffers.

#### Scenario: Rebuild record with string field modified
- **WHEN** a record has 3 fields and one field's buffer was modified
- **THEN** all field header `dsize` values are recalculated
- **AND** the record header `dsize` equals sum(field_header_size + field_buffer_size) for all fields

### Requirement: XXXX size prefix management
The system SHALL manage XXXX size prefix fields for fields whose buffer exceeds 65535 bytes. If a field has the `isSizeXXXX` flag and a preceding XXXX field exists, its 4-byte value SHALL be updated to the field's buffer length. If no preceding XXXX field exists, one SHALL be inserted.

#### Scenario: Field exceeds 65535 bytes
- **WHEN** a field's buffer is 70000 bytes
- **AND** the field has `isSizeXXXX` flag set
- **AND** no preceding XXXX field exists
- **THEN** a 4-byte XXXX header field with value 70000 is inserted before the field

#### Scenario: Field shrinks below 65536 bytes
- **WHEN** a field previously had a buffer >65535 bytes (had XXXX prefix)
- **AND** after modification its buffer is 50000 bytes
- **THEN** the preceding XXXX field is removed

### Requirement: Zlib recompression
The system SHALL recompress rebuilt record data using zlib (RFC 1950) when the record has the compressed flag set. The compressed data format SHALL be `[4-byte decompressedSize LE] + [zlib data]`.

#### Scenario: Compress record after rebuild
- **WHEN** a record has the compressed flag (0x00040000)
- **AND** the record's fields have been rebuilt into a contiguous buffer
- **THEN** the buffer is compressed via zlib
- **AND** the output is `[4-byte decompressed_size] + [compressed_blob]`
- **AND** the record header `dsize` equals the compressed output length

#### Scenario: Uncompressed record is not recompressed
- **WHEN** a record does NOT have the compressed flag
- **THEN** the rebuilt fields are serialized directly without compression
