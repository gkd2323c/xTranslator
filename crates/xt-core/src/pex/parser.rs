//! PEX 二进制解析器 — 从编译的 Papyrus 脚本中提取可翻译字符串。
//! 严格支持真实 Bethesda PEX 规范：Big-Endian (Skyrim) 与 Little-Endian (FO4/FO76/Starfield)。

use byteorder::{BigEndian, ByteOrder, LittleEndian, ReadBytesExt};
use std::io::{self, Cursor, Read, Result};

use super::types::*;

pub const PEX_MAGIC_BIG: u32 = 0xFA57C0DE;
pub const PEX_MAGIC_LITTLE: u32 = 0xDEC057FA;

/// 辅助读取器：根据大小端读取整数与字符串
pub struct PexReader<R> {
    pub reader: R,
    pub endian: PexEndian,
}

impl<R: Read> PexReader<R> {
    pub fn new(reader: R, endian: PexEndian) -> Self {
        Self { reader, endian }
    }

    pub fn read_u8(&mut self) -> Result<u8> {
        self.reader.read_u8()
    }

    pub fn read_u16(&mut self) -> Result<u16> {
        match self.endian {
            PexEndian::LittleEndian => self.reader.read_u16::<LittleEndian>(),
            PexEndian::BigEndian => self.reader.read_u16::<BigEndian>(),
        }
    }

    pub fn read_u32(&mut self) -> Result<u32> {
        match self.endian {
            PexEndian::LittleEndian => self.reader.read_u32::<LittleEndian>(),
            PexEndian::BigEndian => self.reader.read_u32::<BigEndian>(),
        }
    }

    pub fn read_i32(&mut self) -> Result<i32> {
        match self.endian {
            PexEndian::LittleEndian => self.reader.read_i32::<LittleEndian>(),
            PexEndian::BigEndian => self.reader.read_i32::<BigEndian>(),
        }
    }

    pub fn read_u64(&mut self) -> Result<u64> {
        match self.endian {
            PexEndian::LittleEndian => self.reader.read_u64::<LittleEndian>(),
            PexEndian::BigEndian => self.reader.read_u64::<BigEndian>(),
        }
    }

    pub fn read_f32(&mut self) -> Result<f32> {
        match self.endian {
            PexEndian::LittleEndian => self.reader.read_f32::<LittleEndian>(),
            PexEndian::BigEndian => self.reader.read_f32::<BigEndian>(),
        }
    }

    pub fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        self.reader.read_exact(buf)
    }

    pub fn read_string(&mut self) -> Result<String> {
        let len = self.read_u16()? as usize;
        let mut bytes = vec![0u8; len];
        self.reader.read_exact(&mut bytes)?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }
}

impl<T: io::Seek> PexReader<T> {
    /// 返回当前读取位置的绝对偏移
    pub fn position(&mut self) -> Result<u64> {
        self.reader.stream_position()
    }
}

/// 检测 PEX 大小端
pub fn detect_endian(data: &[u8]) -> Option<PexEndian> {
    if data.len() < 4 {
        return None;
    }
    if data[0..4] == [0xFA, 0x57, 0xC0, 0xDE] {
        Some(PexEndian::BigEndian)
    } else if data[0..4] == [0xDE, 0xC0, 0x57, 0xFA] {
        Some(PexEndian::LittleEndian)
    } else {
        None
    }
}

pub fn parse_pex<R: Read>(reader: &mut R) -> Result<PexScript> {
    // 读取全部字节以便精确捕获 header_raw 和 data_raw
    let mut raw_bytes = Vec::new();
    reader.read_to_end(&mut raw_bytes)?;

    if raw_bytes.len() < 16 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "PEX file too short",
        ));
    }

    let endian = detect_endian(&raw_bytes).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid PEX magic: {:?}", &raw_bytes[0..4]),
        )
    })?;

    let raw_magic = match endian {
        PexEndian::BigEndian => BigEndian::read_u32(&raw_bytes[0..4]),
        PexEndian::LittleEndian => LittleEndian::read_u32(&raw_bytes[0..4]),
    };

    // 在整个 raw_bytes 上建立游标，保证 position() 是绝对偏移
    let mut cur = Cursor::new(&raw_bytes);
    let _ = cur.read_u32::<BigEndian>()?; // 跳过 magic (已通过 detect_endian 验证)

    let mut pex_reader = PexReader::new(&mut cur, endian);

    let major_version = pex_reader.read_u8()?;
    let minor_version = pex_reader.read_u8()?;
    let game_id = pex_reader.read_u16()?;
    let compile_time = pex_reader.read_u64()?;

    let source_file_name = pex_reader.read_string()?;
    let user_name = pex_reader.read_string()?;
    let computer_name = pex_reader.read_string()?;

    let header = PexHeader {
        magic: raw_magic,
        endian,
        major_version,
        minor_version,
        game_id,
        compile_time,
        source_file_name,
        user_name,
        computer_name,
    };

    // 字符串表 count（header_raw 必须包含这个 u16！Delphi headerPexBuffer 在读 tableCount 后截取）
    let st_count = pex_reader.read_u16()? as usize;

    // header_raw: 从 magic 到 stringTableCount (含) 的完整字节
    let header_end_pos = pex_reader.position()? as usize;
    let header_raw = raw_bytes[0..header_end_pos].to_vec();

    // 字符串表
    let mut string_table = Vec::with_capacity(st_count);
    for i in 0..st_count {
        let text = pex_reader.read_string()?;
        string_table.push(PexStringEntry {
            index: i as u16,
            text,
        });
    }

    // data_raw: stringTable 之后的全部字节 (hasDebugInfo + debugInfo + userFlags + objects)
    let data_start_pos = pex_reader.position()? as usize;
    let data_raw = raw_bytes[data_start_pos..].to_vec();

    // 解析数据区以提取可翻译文本
    let mut translatable = Vec::new();
    let st = &string_table;

    // Debug info flag
    let has_debug_info = pex_reader.read_u8().unwrap_or(0);
    if has_debug_info == 1 {
        let _debug_mod_time = pex_reader.read_u64()?;
        let debug_func_count = pex_reader.read_u16()? as usize;
        for _ in 0..debug_func_count {
            let _obj_name_idx = pex_reader.read_u16()?;
            let _state_name_idx = pex_reader.read_u16()?;
            let _func_name_idx = pex_reader.read_u16()?;
            let _func_type = pex_reader.read_u8()?;
            let line_count = pex_reader.read_u16()? as usize;
            let mut skip_buf = vec![0u8; line_count * 2];
            pex_reader.read_exact(&mut skip_buf)?;
        }

        // Fallout 4 / Starfield (endian == LittleEndian)
        if endian == PexEndian::LittleEndian {
            let group_count = pex_reader.read_u16().unwrap_or(0) as usize;
            for _ in 0..group_count {
                let _obj_idx = pex_reader.read_u16()?;
                let _state_idx = pex_reader.read_u16()?;
                let _func_idx = pex_reader.read_u16()?;
                let _group_type = pex_reader.read_u32()?;
                let prop_count = pex_reader.read_u16()? as usize;
                let mut skip_buf = vec![0u8; prop_count * 2];
                pex_reader.read_exact(&mut skip_buf)?;
            }

            let struct_count = pex_reader.read_u16().unwrap_or(0) as usize;
            for _ in 0..struct_count {
                let _obj_idx = pex_reader.read_u16()?;
                let _state_idx = pex_reader.read_u16()?;
                let count = pex_reader.read_u16()? as usize;
                let mut skip_buf = vec![0u8; count * 2];
                pex_reader.read_exact(&mut skip_buf)?;
            }
        }
    }

    // User flags
    let uf_count = pex_reader.read_u16().unwrap_or(0) as usize;
    for _ in 0..uf_count {
        let _name_idx = pex_reader.read_u16()?;
        let _flag = pex_reader.read_u8()?;
    }

    // Objects
    let object_count = pex_reader.read_u16().unwrap_or(0) as usize;
    for _ in 0..object_count {
        let name_idx = pex_reader.read_u16()?;
        let obj_name = lookup_str(st, name_idx);

        // Delphi: tmpSize := readValue_Int(fstream) - 4  — size 字段包含 4 字节 size 自身
        let raw_size = pex_reader.read_u32()? as usize;
        let body_size = raw_size.saturating_sub(4);
        let mut body_bytes = vec![0u8; body_size];
        pex_reader.read_exact(&mut body_bytes)?;

        let mut body_cur = Cursor::new(&body_bytes[..]);
        let mut body_reader = PexReader::new(&mut body_cur, endian);
        parse_object_body(&mut body_reader, &obj_name, st, &mut translatable, game_id)?;
    }

    Ok(PexScript {
        header,
        string_table,
        translatable,
        header_raw,
        data_raw,
    })
}

type StrTab = Vec<PexStringEntry>;

fn lookup_str(st: &StrTab, idx: u16) -> String {
    st.get(idx as usize)
        .map(|e| e.text.clone())
        .unwrap_or_default()
}

/// 按 Delphi `checkObjectData` 顺序解析对象体（parent → doc → [LE] uConst → uFlags → autoState → [LE] structs → vars → [Starfield] guards → props → states）
fn parse_object_body<R: Read>(
    r: &mut PexReader<R>,
    obj_name: &str,
    st: &StrTab,
    out: &mut Vec<PexTranslatableString>,
    game_id: u16,
) -> Result<()> {
    let _parent = r.read_u16()?;
    let doc_idx = r.read_u16()?;
    let doc = lookup_str(st, doc_idx);
    if !doc.is_empty() {
        out.push(PexTranslatableString {
            object_name: obj_name.to_string(),
            state_name: String::new(),
            function_name: String::new(),
            string_type: "DocString".to_string(),
            source_text: doc,
            translation: String::new(),
        });
    }

    // [LE] uConst (u8)
    if r.endian == PexEndian::LittleEndian {
        let _uconst = r.read_u8()?;
    }

    // userFlags (u32)
    let _user_flags = r.read_u32()?;

    let _auto_state = r.read_u16()?;

    // [LE] structs
    if r.endian == PexEndian::LittleEndian {
        let struct_count = r.read_u16().unwrap_or(0) as usize;
        for _ in 0..struct_count {
            let _struct_name = r.read_u16()?;
            let var_count = r.read_u16().unwrap_or(0) as usize;
            for _ in 0..var_count {
                parse_variable(r, st, obj_name, "", "", out, true)?;
            }
        }
    }

    // Variables
    let var_count = r.read_u16().unwrap_or(0) as usize;
    for _ in 0..var_count {
        parse_variable(r, st, obj_name, "", "", out, false)?;
    }

    // [Starfield only, game_id == 4] Guards — 每个 guard 只是一个 string ID
    if game_id == 4 {
        let guard_count = r.read_u16().unwrap_or(0) as usize;
        for _ in 0..guard_count {
            let _guard_name = r.read_u16()?;
        }
    }

    // Properties
    let prop_count = r.read_u16().unwrap_or(0) as usize;
    for _ in 0..prop_count {
        parse_property(r, obj_name, st, out)?;
    }

    // States
    let state_count = r.read_u16().unwrap_or(0) as usize;
    for _ in 0..state_count {
        let state_name_idx = r.read_u16()?;
        let state_name = lookup_str(st, state_name_idx);
        let func_count = r.read_u16()? as usize;
        for _ in 0..func_count {
            parse_function(r, obj_name, &state_name, st, out)?;
        }
    }

    Ok(())
}

/// 按 Delphi `checkVariables` 顺序解析变量：name → type → uFlags(u32) → value → [LE] group → [struct] docType
fn parse_variable<R: Read>(
    r: &mut PexReader<R>,
    st: &StrTab,
    obj_name: &str,
    state_name: &str,
    func_name: &str,
    out: &mut Vec<PexTranslatableString>,
    use_doc_string: bool,
) -> Result<()> {
    let _name_idx = r.read_u16()?;
    let _type_idx = r.read_u16()?;
    let _uflags = r.read_u32()?;

    // value (checkVariableData)
    parse_var_data(r, st, obj_name, state_name, func_name, out)?;

    if r.endian == PexEndian::LittleEndian {
        let _group = r.read_u8()?;
        if use_doc_string {
            let _doc_type = r.read_u16()?;
        }
    }

    Ok(())
}

fn parse_property<R: Read>(
    r: &mut PexReader<R>,
    obj_name: &str,
    st: &StrTab,
    out: &mut Vec<PexTranslatableString>,
) -> Result<()> {
    let name_idx = r.read_u16()?;
    let prop_name = lookup_str(st, name_idx);
    let _type_idx = r.read_u16()?;
    let doc_idx = r.read_u16()?;
    let doc = lookup_str(st, doc_idx);
    if !doc.is_empty() {
        out.push(PexTranslatableString {
            object_name: obj_name.to_string(),
            state_name: String::new(),
            function_name: prop_name.clone(),
            string_type: "PropertyName".to_string(),
            source_text: doc,
            translation: String::new(),
        });
    }

    let _uflags = r.read_u32()?;
    let flags = r.read_u8()?;

    // auto var (flag bit 2 = 0x04)
    if flags & 0x04 != 0 {
        let _auto_var = r.read_u16()?;
    }
    // read handler (flag bit 0 = 0x01)
    if flags & 0x01 != 0 {
        parse_function_no_name(r, obj_name, st, out)?;
    }
    // write handler (flag bit 1 = 0x02)
    if flags & 0x02 != 0 {
        parse_function_no_name(r, obj_name, st, out)?;
    }
    Ok(())
}

fn parse_function<R: Read>(
    r: &mut PexReader<R>,
    obj_name: &str,
    state_name: &str,
    st: &StrTab,
    out: &mut Vec<PexTranslatableString>,
) -> Result<()> {
    let func_name_idx = r.read_u16()?;
    let func_name = lookup_str(st, func_name_idx);
    let _ret_type_idx = r.read_u16()?;
    let doc_idx = r.read_u16()?;
    let doc = lookup_str(st, doc_idx);
    if !doc.is_empty() {
        out.push(PexTranslatableString {
            object_name: obj_name.to_string(),
            state_name: state_name.to_string(),
            function_name: func_name.clone(),
            string_type: "DocString".to_string(),
            source_text: doc,
            translation: String::new(),
        });
    }

    let _uflags = r.read_u32()?;
    let _flags = r.read_u8()?;

    // params
    let param_count = r.read_u16()? as usize;
    for _ in 0..param_count {
        let _pn = r.read_u16()?;
        let _pt = r.read_u16()?;
    }

    // locals
    let local_count = r.read_u16()? as usize;
    for _ in 0..local_count {
        let _ln = r.read_u16()?;
        let _lt = r.read_u16()?;
    }

    // instructions
    let inst_count = r.read_u16()? as usize;
    for _ in 0..inst_count {
        parse_instruction_for_strings(r, obj_name, state_name, &func_name, st, out)?;
    }

    Ok(())
}

/// property read/write handler 函数（无函数名，getFirst=false）
fn parse_function_no_name<R: Read>(
    r: &mut PexReader<R>,
    obj_name: &str,
    st: &StrTab,
    out: &mut Vec<PexTranslatableString>,
) -> Result<()> {
    let _ret_type_idx = r.read_u16()?;
    let doc_idx = r.read_u16()?;
    let doc = lookup_str(st, doc_idx);
    if !doc.is_empty() {
        out.push(PexTranslatableString {
            object_name: obj_name.to_string(),
            state_name: String::new(),
            function_name: "(handler)".to_string(),
            string_type: "DocString".to_string(),
            source_text: doc,
            translation: String::new(),
        });
    }

    let _uflags = r.read_u32()?;
    let _flags = r.read_u8()?;

    let param_count = r.read_u16()? as usize;
    for _ in 0..param_count {
        let _pn = r.read_u16()?;
        let _pt = r.read_u16()?;
    }

    let local_count = r.read_u16()? as usize;
    for _ in 0..local_count {
        let _ln = r.read_u16()?;
        let _lt = r.read_u16()?;
    }

    let inst_count = r.read_u16()? as usize;
    for _ in 0..inst_count {
        parse_instruction_for_strings(r, obj_name, "", "(handler)", st, out)?;
    }

    Ok(())
}

/// Delphi `checkVariableData`：1 字节 type tag + payload
fn parse_var_data<R: Read>(
    r: &mut PexReader<R>,
    st: &StrTab,
    obj_name: &str,
    state_name: &str,
    func_name: &str,
    out: &mut Vec<PexTranslatableString>,
) -> Result<()> {
    let type_tag = r.read_u8()?;
    match type_tag {
        0 => Ok(()),
        1 | 2 => {
            let str_idx = r.read_u16()?;
            let text = lookup_str(st, str_idx);
            if type_tag == 2 && !text.is_empty() {
                out.push(PexTranslatableString {
                    object_name: obj_name.to_string(),
                    state_name: state_name.to_string(),
                    function_name: func_name.to_string(),
                    string_type: "StringLiteral".to_string(),
                    source_text: text,
                    translation: String::new(),
                });
            }
            Ok(())
        }
        3 => {
            let _val = r.read_u32()?;
            Ok(())
        }
        4 => {
            let _val = r.read_f32()?;
            Ok(())
        }
        5 => {
            let _val = r.read_u8()?;
            Ok(())
        }
        6 => {
            let count = r.read_u32()? as usize;
            for _ in 0..count {
                parse_var_data(r, st, obj_name, state_name, func_name, out)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn parse_instruction_for_strings<R: Read>(
    r: &mut PexReader<R>,
    obj_name: &str,
    state_name: &str,
    func_name: &str,
    st: &StrTab,
    out: &mut Vec<PexTranslatableString>,
) -> Result<()> {
    let opcode_raw = r.read_u8()?;
    let opcode = super::decompile::Opcode::from_u8(opcode_raw);
    let fixed_count = opcode.fixed_arg_count();

    let mut extra_args_count: i32 = 0;
    for _ in 0..fixed_count {
        let val_tag = r.read_u8()?;
        match val_tag {
            0 => {}
            1 => {
                let _idx = r.read_u16()?;
            }
            2 => {
                let idx = r.read_u16()?;
                let text = lookup_str(st, idx);
                if !text.is_empty() {
                    out.push(PexTranslatableString {
                        object_name: obj_name.to_string(),
                        state_name: state_name.to_string(),
                        function_name: func_name.to_string(),
                        string_type: "StringLiteral".to_string(),
                        source_text: text,
                        translation: String::new(),
                    });
                }
            }
            3 => {
                let val = r.read_i32()?;
                extra_args_count = val;
            }
            4 => {
                let _val = r.read_f32()?;
                extra_args_count = 0;
            }
            5 => {
                let _val = r.read_u8()?;
                extra_args_count = 0;
            }
            _ => {
                extra_args_count = 0;
            }
        }
    }

    if opcode.is_extended_proc() && extra_args_count > 0 {
        for _ in 0..extra_args_count {
            let val_tag = r.read_u8()?;
            match val_tag {
                0 => {}
                1 => {
                    let _idx = r.read_u16()?;
                }
                2 => {
                    let idx = r.read_u16()?;
                    let text = lookup_str(st, idx);
                    if !text.is_empty() {
                        out.push(PexTranslatableString {
                            object_name: obj_name.to_string(),
                            state_name: state_name.to_string(),
                            function_name: func_name.to_string(),
                            string_type: "StringLiteral".to_string(),
                            source_text: text,
                            translation: String::new(),
                        });
                    }
                }
                3 => {
                    let _val = r.read_i32()?;
                }
                4 => {
                    let _val = r.read_f32()?;
                }
                5 => {
                    let _val = r.read_u8()?;
                }
                _ => {}
            }
        }
    }

    Ok(())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use byteorder::{BigEndian, WriteBytesExt};

    #[test]
    fn test_reject_invalid_magic() {
        let mut data = Cursor::new(vec![0x12, 0x34, 0x56, 0x78]);
        assert!(parse_pex(&mut data).is_err());
    }

    #[test]
    fn test_empty_string_table_skyrim_big_endian() {
        let mut buf = Vec::new();

        buf.write_u32::<BigEndian>(PEX_MAGIC_BIG).unwrap();
        buf.push(3); // major
        buf.push(9); // minor
        buf.write_u16::<BigEndian>(1).unwrap(); // game_id = 1 (Skyrim)
        buf.write_u64::<BigEndian>(12345678).unwrap(); // compile_time

        // source, user, computer (all empty strings: len = 0)
        buf.write_u16::<BigEndian>(0).unwrap();
        buf.write_u16::<BigEndian>(0).unwrap();
        buf.write_u16::<BigEndian>(0).unwrap();

        // string table count = 0
        buf.write_u16::<BigEndian>(0).unwrap();

        // has_debug_info = 0
        buf.push(0);
        // user_flags = 0
        buf.write_u16::<BigEndian>(0).unwrap();
        // objects = 0
        buf.write_u16::<BigEndian>(0).unwrap();

        let mut cursor = Cursor::new(buf);
        let script = parse_pex(&mut cursor).unwrap();
        assert_eq!(script.header.game_id, 1);
        assert_eq!(script.header.endian, PexEndian::BigEndian);
        assert!(script.string_table.is_empty());
        // header_raw 必须包含 stringTableCount
        assert!(script.header_raw.len() >= 18);
        // 验证 header_raw 尾两字节是大端 0 (count)
        let n = script.header_raw.len();
        assert_eq!(&script.header_raw[n - 2..], &[0x00, 0x00]);
    }
}
