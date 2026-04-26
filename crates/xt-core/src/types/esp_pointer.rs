use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Result, Write};

/// Delphi `sHeaderSig` = `array [0..3] of AnsiChar`，4 字节签名
pub type HeaderSig = [u8; 4];

/// ESP 指针 - 24 字节，SST v8 格式中使用
///
/// 对应 Delphi 的 `rEspPointerLite` 结构，用于精确定位字符串在 ESP 文件中的位置
/// 布局（小端序）：
/// - str_id: i32 (4字节)    - Strings 文件中的字符串 ID
/// - form_id: u32 (4字节)   - 记录的 FormID
/// - record_sig: [u8; 4] (4字节) - 记录类型签名（如 "INFO", "QUST"）
/// - field_sig: [u8; 4] (4字节) - 字段签名（如 "NAM1", "FULL"）
/// - index: u16 (2字节)      - 字段在记录中的索引
/// - index_max: u16 (2字节)  - 记录中字段总数
/// - edid_hash: u32 (4字节)  - Editor ID 的哈希值
/// = 24 字节
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EspPointer {
    /// Strings 文件中的字符串 ID（-1 表示未设置）
    pub str_id: i32,
    /// 记录的 FormID（唯一标识游戏中的记录）
    pub form_id: u32,
    /// 记录类型签名（4 字节 ASCII，如 "INFO", "QUST"）
    pub record_sig: HeaderSig,
    /// 字段签名（4 字节 ASCII，如 "NAM1", "FULL"）
    pub field_sig: HeaderSig,
    /// 字段在记录中的索引位置
    pub index: u16,
    /// 记录中字段的总数
    pub index_max: u16,
    /// Editor ID 的 FNV-1a 哈希值
    pub edid_hash: u32,
}

impl EspPointer {
    pub const SIZE: usize = 24;

    /// 创建空的 ESP 指针（所有字段为零）
    pub const fn null() -> Self {
        Self {
            str_id: -1,
            form_id: 0,
            record_sig: [0; 4],
            field_sig: [0; 4],
            index: 0,
            index_max: 0,
            edid_hash: 0,
        }
    }

    /// 从 SST v8 读取（小端序）
    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        // 读取顺序必须与 Delphi/SST v8 二进制布局完全一致。
        let str_id = reader.read_i32::<LittleEndian>()?;
        let form_id = reader.read_u32::<LittleEndian>()?;
        let mut record_sig = [0u8; 4];
        reader.read_exact(&mut record_sig)?;
        let mut field_sig = [0u8; 4];
        reader.read_exact(&mut field_sig)?;
        let index = reader.read_u16::<LittleEndian>()?;
        let index_max = reader.read_u16::<LittleEndian>()?;
        let edid_hash = reader.read_u32::<LittleEndian>()?;
        Ok(Self {
            str_id,
            form_id,
            record_sig,
            field_sig,
            index,
            index_max,
            edid_hash,
        })
    }

    /// 写入 SST v8（小端序）
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        // 写出顺序与 read_from 对称，确保可逆 roundtrip。
        writer.write_i32::<LittleEndian>(self.str_id)?;
        writer.write_u32::<LittleEndian>(self.form_id)?;
        writer.write_all(&self.record_sig)?;
        writer.write_all(&self.field_sig)?;
        writer.write_u16::<LittleEndian>(self.index)?;
        writer.write_u16::<LittleEndian>(self.index_max)?;
        writer.write_u32::<LittleEndian>(self.edid_hash)?;
        Ok(())
    }
}

/// Delphi 版 `StringHash` 算法 - FNV-1a 哈希
///
/// 关键点：Delphi 的 UnicodeString 中，`byte(str[i])` 只取字符的低字节。
/// 例如：字符 "你" (U+4F60) 的 UTF-16 编码为 [0x60, 0x4F]，只取 0x60 参与哈希计算。
/// 这种设计确保了与 Delphi 原版工具的哈希值完全一致。
///
/// 算法：FNV-1a (Fowler–Noll–Vo)
/// - 初始值: 2166136261
/// - 质数: 16777619
/// - 对每个 UTF-16 编码单元的低字节进行哈希
pub fn string_hash(s: &str) -> u32 {
    const FNV_OFFSET_BASIS: u32 = 2166136261;
    const FNV_PRIME: u32 = 16777619;

    let mut hash = FNV_OFFSET_BASIS;
    // 遍历 UTF-16 编码单元（而非 UTF-8 字节）。
    for c in s.encode_utf16() {
        // 只取低字节（兼容 Delphi 的 byte(str[i]) 行为）
        let b = (c & 0xFF) as u8;
        // FNV-1a 核心计算：异或后乘以质数
        hash = (hash ^ b as u32).wrapping_mul(FNV_PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_esp_pointer_roundtrip() {
        let ptr = EspPointer {
            str_id: 12345,
            form_id: 0xDEADBEEF,
            record_sig: *b"INFO",
            field_sig: *b"NAM1",
            index: 2,
            index_max: 5,
            edid_hash: 0x12345678,
        };

        let mut buf = Vec::new();
        ptr.write_to(&mut buf).unwrap();
        assert_eq!(buf.len(), EspPointer::SIZE);

        let ptr2 = EspPointer::read_from(&mut buf.as_slice()).unwrap();
        assert_eq!(ptr, ptr2);
    }

    #[test]
    fn test_string_hash_ascii() {
        // 纯 ASCII 字符串：Delphi UnicodeString 的低字节 = ASCII 值
        // 结果等同于标准 FNV-1a 字节级哈希
        let h = string_hash("Hello");
        let mut expected: u32 = 2166136261;
        for b in "Hello".bytes() {
            expected = (expected ^ b as u32).wrapping_mul(16777619);
        }
        assert_eq!(h, expected);
    }

    #[test]
    fn test_string_hash_non_ascii() {
        // 非 ASCII 字符：必须取 UTF-16 低字节
        // "你" = U+4F60 -> UTF-16LE: [0x60, 0x4F] -> 低字节 = 0x60
        let h = string_hash("你");
        let mut expected: u32 = 2166136261;
        expected = (expected ^ 0x60).wrapping_mul(16777619);
        assert_eq!(h, expected);
    }
}
