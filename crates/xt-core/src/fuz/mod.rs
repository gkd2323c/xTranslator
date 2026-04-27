//! FUZ file parser — Bethesda voice line container (LIP + WAV)
//!
//! Skyrim/Fallout voice lines are stored as .fuz files containing:
//! - LIP (lip-sync) data
//! - WAV audio data
//!
//! Based on Delphi TESVT_Fuz.pas and xEdit wbFUZ.pas.

use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Cursor, Read, Result};

/// FUZ magic bytes: "FUZE"
const FUZ_MAGIC: &[u8; 4] = b"FUZE";

#[derive(Clone, Debug)]
pub struct FuzFile {
    pub wav_data: Vec<u8>,
    pub duration_secs: f32,
    pub sample_rate: u32,
    pub channels: u16,
}

impl FuzFile {
    pub fn parse<R: Read>(reader: &mut R) -> Result<Self> {
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        if &magic != FUZ_MAGIC {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Not a valid FUZ file (magic mismatch)",
            ));
        }

        // Skip 4 unknown bytes (likely LIP format version)
        let _unk4 = reader.read_u32::<LittleEndian>()?;

        // Read LIP data size
        let lip_size = reader.read_u32::<LittleEndian>()?;

        // Skip LIP data
        let mut lip_buf = vec![0u8; lip_size as usize];
        reader.read_exact(&mut lip_buf)?;

        // Read remaining WAV data
        let mut wav_data = Vec::new();
        reader.read_to_end(&mut wav_data)?;

        let (duration_secs, sample_rate, channels) = parse_wav_header(&wav_data);

        Ok(FuzFile {
            wav_data,
            duration_secs,
            sample_rate,
            channels,
        })
    }
}

fn parse_wav_header(data: &[u8]) -> (f32, u32, u16) {
    if data.len() < 44 {
        return (0.0, 0, 0);
    }

    let mut cur = Cursor::new(data);

    // Parse RIFF/WAV header
    let _riff = cur.read_u32::<LittleEndian>();
    let _file_size = cur.read_u32::<LittleEndian>();
    let _wave = cur.read_u32::<LittleEndian>();
    let _fmt = cur.read_u32::<LittleEndian>();
    let _fmt_size = cur.read_u32::<LittleEndian>();

    let _audio_format = cur.read_u16::<LittleEndian>().unwrap_or(0);
    let channels = cur.read_u16::<LittleEndian>().unwrap_or(1);
    let sample_rate = cur.read_u32::<LittleEndian>().unwrap_or(0);
    let byte_rate = cur.read_u32::<LittleEndian>().unwrap_or(0);
    let _block_align = cur.read_u16::<LittleEndian>();
    let _bits = cur.read_u16::<LittleEndian>();

    // Find "data" chunk to get data size
    let pos = cur.position() as usize;
    let mut data_size = data.len() - pos;
    if let Some(dp) = data[pos..].windows(4).position(|w| w == b"data") {
        let ds = pos + dp + 4;
        if ds + 4 <= data.len() {
            data_size = u32::from_le_bytes([data[ds], data[ds+1], data[ds+2], data[ds+3]]) as usize;
        }
    }

    let duration = if byte_rate > 0 {
        data_size as f32 / byte_rate as f32
    } else {
        0.0
    };

    (duration, sample_rate, channels)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn build_test_fuz() -> Vec<u8> {
        let mut data = Vec::new();

        // FUZE header
        data.extend_from_slice(b"FUZE");
        // unknown 4 bytes
        data.extend_from_slice(&[1u8, 0, 0, 0]);
        // lip_size = 4 (dummy LIP data)
        data.extend_from_slice(&4u32.to_le_bytes());
        // LIP data
        data.extend_from_slice(&[0u8, 0, 0, 0]);

        // Minimal WAV header (44 bytes)
        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&50u32.to_le_bytes()); // file_size - 8
        data.extend_from_slice(b"WAVE");
        data.extend_from_slice(b"fmt ");
        data.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
        data.extend_from_slice(&1u16.to_le_bytes()); // PCM
        data.extend_from_slice(&2u16.to_le_bytes()); // channels
        data.extend_from_slice(&44100u32.to_le_bytes()); // sample_rate
        data.extend_from_slice(&176400u32.to_le_bytes()); // byte_rate
        data.extend_from_slice(&4u16.to_le_bytes()); // block_align
        data.extend_from_slice(&16u16.to_le_bytes()); // bits_per_sample
        data.extend_from_slice(b"data");
        data.extend_from_slice(&10u32.to_le_bytes()); // data_size
        data.extend_from_slice(&[0u8; 10]); // PCM silence

        data
    }

    #[test]
    fn test_parse_minimal_fuz() {
        let data = build_test_fuz();
        let mut cursor = Cursor::new(&data[..]);
        let fuz = FuzFile::parse(&mut cursor).unwrap();

        assert_eq!(fuz.sample_rate, 44100);
        assert_eq!(fuz.channels, 2);
        assert!(fuz.duration_secs > 0.0);
        assert!(!fuz.wav_data.is_empty());
    }
}
