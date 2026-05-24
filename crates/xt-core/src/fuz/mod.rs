//! FUZ file parser — Bethesda voice line container (LIP + WAV)
//!
//! Skyrim/Fallout voice lines are stored as .fuz files containing:
//! - LIP (lip-sync) data - 唇形同步关键帧
//! - WAV audio data - 音频数据
//!
//! LIP 格式说明：
//! - 版本号 (4 bytes)
//! - 关键帧数量 (4 bytes)
//! - 每个关键帧: 时间戳 (float) + 口型形状索引 (byte)
//!
//! 口型形状索引对应游戏中的面部动画：
//! 0 = 静音, 1 = A, 2 = E, 3 = I, 4 = O, 5 = U, 6 = F, 7 = V, 8 = 无声等
//!
//! Based on Delphi TESVT_Fuz.pas and xEdit wbFUZ.pas.

use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Cursor, Read, Result};

/// FUZ magic bytes: "FUZE"
const FUZ_MAGIC: &[u8; 4] = b"FUZE";

/// LIP 关键帧 - 唇形同步数据点
#[derive(Clone, Debug, PartialEq)]
pub struct LipKeyframe {
    /// 时间戳（秒）
    pub time: f32,
    /// 口型形状索引 (0-15)
    pub shape: u8,
}

/// LIP 数据结构
#[derive(Clone, Debug)]
pub struct LipData {
    /// LIP 格式版本
    pub version: u32,
    /// 关键帧列表
    pub keyframes: Vec<LipKeyframe>,
}

#[derive(Clone, Debug)]
pub struct FuzFile {
    /// LIP 唇形同步数据
    pub lip_data: Option<LipData>,
    /// WAV 音频数据（包含完整的 WAV 文件头）
    pub wav_data: Vec<u8>,
    /// 音频时长（秒）
    pub duration_secs: f32,
    /// 采样率
    pub sample_rate: u32,
    /// 声道数
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

        // LIP format version
        let lip_version = reader.read_u32::<LittleEndian>()?;

        // Read LIP data size
        let lip_size = reader.read_u32::<LittleEndian>()?;

        // Parse LIP data
        let lip_data = if lip_size > 0 {
            let mut lip_buf = vec![0u8; lip_size as usize];
            reader.read_exact(&mut lip_buf)?;
            Some(Self::parse_lip_data(lip_version, &lip_buf)?)
        } else {
            None
        };

        // Read remaining WAV data
        let mut wav_data = Vec::new();
        reader.read_to_end(&mut wav_data)?;

        let (duration_secs, sample_rate, channels) = parse_wav_header(&wav_data);

        Ok(FuzFile {
            lip_data,
            wav_data,
            duration_secs,
            sample_rate,
            channels,
        })
    }

    /// 解析 LIP 唇形同步数据
    fn parse_lip_data(version: u32, data: &[u8]) -> Result<LipData> {
        let mut cursor = Cursor::new(data);

        // 读取关键帧数量
        let keyframe_count = cursor.read_u32::<LittleEndian>()?;

        let mut keyframes = Vec::with_capacity(keyframe_count as usize);
        for _ in 0..keyframe_count {
            let time = cursor.read_f32::<LittleEndian>()?;
            let shape = cursor.read_u8()?;
            keyframes.push(LipKeyframe { time, shape });
        }

        Ok(LipData { version, keyframes })
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
            data_size =
                u32::from_le_bytes([data[ds], data[ds + 1], data[ds + 2], data[ds + 3]]) as usize;
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

    fn build_test_fuz_with_lip() -> Vec<u8> {
        let mut data = Vec::new();

        // FUZE header
        data.extend_from_slice(b"FUZE");
        // LIP version = 1
        data.extend_from_slice(&1u32.to_le_bytes());

        // LIP data: 4 bytes (keyframe count) + 2 keyframes * 5 bytes each = 14 bytes
        let lip_size = 14u32;
        data.extend_from_slice(&lip_size.to_le_bytes());

        // LIP data content
        let keyframe_count = 2u32;
        data.extend_from_slice(&keyframe_count.to_le_bytes());
        // Keyframe 1: time=0.0, shape=1 (A)
        data.extend_from_slice(&0.0f32.to_le_bytes());
        data.push(1);
        // Keyframe 2: time=0.5, shape=2 (E)
        data.extend_from_slice(&0.5f32.to_le_bytes());
        data.push(2);

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

    fn build_test_fuz_no_lip() -> Vec<u8> {
        let mut data = Vec::new();

        // FUZE header
        data.extend_from_slice(b"FUZE");
        // LIP version = 0
        data.extend_from_slice(&0u32.to_le_bytes());
        // lip_size = 0 (no LIP data)
        data.extend_from_slice(&0u32.to_le_bytes());

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
    fn test_parse_fuz_with_lip_data() {
        let data = build_test_fuz_with_lip();
        let mut cursor = Cursor::new(&data[..]);
        let fuz = FuzFile::parse(&mut cursor).unwrap();

        assert_eq!(fuz.sample_rate, 44100);
        assert_eq!(fuz.channels, 2);
        assert!(fuz.duration_secs > 0.0);
        assert!(!fuz.wav_data.is_empty());

        // Check LIP data
        assert!(fuz.lip_data.is_some());
        let lip_data = fuz.lip_data.unwrap();
        assert_eq!(lip_data.version, 1);
        assert_eq!(lip_data.keyframes.len(), 2);

        assert_eq!(lip_data.keyframes[0].time, 0.0);
        assert_eq!(lip_data.keyframes[0].shape, 1);
        assert_eq!(lip_data.keyframes[1].time, 0.5);
        assert_eq!(lip_data.keyframes[1].shape, 2);
    }

    #[test]
    fn test_parse_fuz_without_lip_data() {
        let data = build_test_fuz_no_lip();
        let mut cursor = Cursor::new(&data[..]);
        let fuz = FuzFile::parse(&mut cursor).unwrap();

        assert_eq!(fuz.sample_rate, 44100);
        assert_eq!(fuz.channels, 2);
        assert!(fuz.lip_data.is_none());
    }

    #[test]
    fn test_parse_invalid_magic() {
        let mut data = Vec::new();
        data.extend_from_slice(b"XXXX"); // Invalid magic
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());

        let mut cursor = Cursor::new(&data[..]);
        let result = FuzFile::parse(&mut cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_lip_keyframe_equality() {
        let kf1 = LipKeyframe {
            time: 0.5,
            shape: 3,
        };
        let kf2 = LipKeyframe {
            time: 0.5,
            shape: 3,
        };
        let kf3 = LipKeyframe {
            time: 1.0,
            shape: 3,
        };

        assert_eq!(kf1, kf2);
        assert_ne!(kf1, kf3);
    }
}
