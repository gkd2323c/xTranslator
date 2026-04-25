use std::io::{Read, Result, Write};

/// 将 Delphi UnicodeString (UTF-16LE) 读为 Rust String
pub fn read_delphi_string<R: Read>(reader: &mut R) -> Result<String> {
    use byteorder::{LittleEndian, ReadBytesExt};
    let size = reader.read_i32::<LittleEndian>()?;
    if size <= 0 {
        return Ok(String::new());
    }
    let mut buf = vec![0u8; size as usize];
    reader.read_exact(&mut buf)?;

    // UTF-16LE -> String
    // 处理可能的未配对代理对 (Delphi 可能产生无效 UTF-16)
    let mut utf16: Vec<u16> = Vec::with_capacity(buf.len() / 2);
    for chunk in buf.chunks_exact(2) {
        let code_unit = u16::from_le_bytes([chunk[0], chunk[1]]);
        utf16.push(code_unit);
    }

    // 使用 lossy 解码以处理 Delphi 可能产生的无效序列
    Ok(String::from_utf16_lossy(&utf16))
}

/// 将 Rust String 写为 Delphi UnicodeString (UTF-16LE)
pub fn write_delphi_string<W: Write>(writer: &mut W, s: &str) -> Result<()> {
    use byteorder::{LittleEndian, WriteBytesExt};
    let utf16: Vec<u16> = s.encode_utf16().collect();
    let bytes: Vec<u8> = utf16.iter().flat_map(|&c| c.to_le_bytes()).collect();
    writer.write_i32::<LittleEndian>(bytes.len() as i32)?;
    writer.write_all(&bytes)?;
    Ok(())
}
