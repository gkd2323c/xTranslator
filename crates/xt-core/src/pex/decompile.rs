//! PEX 反编译器 — 将 PEX 二进制文件解析为结构化类型，并输出与 Delphi xTranslator 完全等价的 Papyrus 伪代码。
//! 严格支持真实 Bethesda PEX 规范与大小端模式（Skyrim Big-Endian 与 FO4/Starfield Little-Endian）。

use byteorder::{BigEndian, ByteOrder, LittleEndian};
use std::fmt::Write;
use std::io::{self, Cursor, Read};

use super::parser::PexReader;
use super::types::{PexEndian, PexHeader, PexStringEntry};

// ── 结构化类型 ──────────────────────────────────────────────────────

/// PEX 指令参数的类型化值（严格对应 Delphi TpexVarData / Papyrus VariableData）
#[derive(Clone, Debug, PartialEq)]
pub enum PexValue {
    None,
    Identifier(String),
    StringLiteral(String),
    Integer(i32),
    Float(f32),
    Bool(bool),
}

impl PexValue {
    /// 格式化为与 Delphi `TpexVarData.getStrValue` 严格一致的字符串表示
    pub fn get_str_value(&self, add_equal: bool) -> String {
        let tag = if add_equal { "= " } else { "" };
        match self {
            PexValue::None => {
                if !add_equal {
                    "none".to_string()
                } else {
                    String::new()
                }
            }
            PexValue::Identifier(s) => format!("{}{}", tag, s),
            PexValue::StringLiteral(s) => format!("{}\"{}\"", tag, s),
            PexValue::Integer(i) => format!("{}{}", tag, i),
            PexValue::Float(f) => format!("{}{:.4}", tag, f),
            PexValue::Bool(b) => format!("{}{}", tag, if *b { "True" } else { "False" }),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            PexValue::Identifier(s) | PexValue::StringLiteral(s) => s.as_str(),
            _ => "",
        }
    }
}

/// 解码后的 PEX 指令
#[derive(Clone, Debug, PartialEq)]
pub struct Instruction {
    pub opcode: Opcode,
    pub raw_opcode: u8,
    pub args: Vec<PexValue>,
}

/// 所有已知的 Papyrus 操作码（严格与 Delphi TESVT_scriptPex.pas 对齐，0x00 ..= 0x32）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Opcode {
    Nop = 0x00,
    Iadd = 0x01,
    Fadd = 0x02,
    Isub = 0x03,
    Fsub = 0x04,
    Imul = 0x05,
    Fmul = 0x06,
    Idiv = 0x07,
    Fdiv = 0x08,
    Imod = 0x09,
    Not = 0x0A,
    Ineg = 0x0B,
    Fneg = 0x0C,
    Assign = 0x0D,
    Cast = 0x0E,
    CmpEq = 0x0F,
    CmpLt = 0x10,
    CmpLte = 0x11,
    CmpGt = 0x12,
    CmpGte = 0x13,
    Jump = 0x14,
    Jz = 0x15,
    Jnz = 0x16,
    Callmethod = 0x17,
    Callparent = 0x18,
    Callstatic = 0x19,
    Return = 0x1A,
    Strcat = 0x1B,
    Propget = 0x1C,
    Propset = 0x1D,
    ArrayCreate = 0x1E,
    ArrayLength = 0x1F,
    ArrayGetElement = 0x20,
    ArraySetElement = 0x21,
    ArrayFindElement = 0x22,
    ArrayRfindElement = 0x23,
    // Fallout 4 新增操作码
    Is = 0x24,
    StructCreate = 0x25,
    StructGet = 0x26,
    StructSet = 0x27,
    StructFind = 0x28,
    StructRFind = 0x29,
    ArrayAdd = 0x2A,
    ArrayInsert = 0x2B,
    ArrayRemoveLast = 0x2C,
    ArrayRemove = 0x2D,
    ArrayClear = 0x2E,
    // Starfield 新增操作码
    GetAllMatchingStruct = 0x2F,
    GuardLock = 0x30,
    GuardUnlock = 0x31,
    GuardTryLock = 0x32,
    /// 未知或未识别的操作码（保留原始数值）
    Unknown(u8),
}

impl Opcode {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0x00 => Self::Nop,
            0x01 => Self::Iadd,
            0x02 => Self::Fadd,
            0x03 => Self::Isub,
            0x04 => Self::Fsub,
            0x05 => Self::Imul,
            0x06 => Self::Fmul,
            0x07 => Self::Idiv,
            0x08 => Self::Fdiv,
            0x09 => Self::Imod,
            0x0A => Self::Not,
            0x0B => Self::Ineg,
            0x0C => Self::Fneg,
            0x0D => Self::Assign,
            0x0E => Self::Cast,
            0x0F => Self::CmpEq,
            0x10 => Self::CmpLt,
            0x11 => Self::CmpLte,
            0x12 => Self::CmpGt,
            0x13 => Self::CmpGte,
            0x14 => Self::Jump,
            0x15 => Self::Jz,
            0x16 => Self::Jnz,
            0x17 => Self::Callmethod,
            0x18 => Self::Callparent,
            0x19 => Self::Callstatic,
            0x1A => Self::Return,
            0x1B => Self::Strcat,
            0x1C => Self::Propget,
            0x1D => Self::Propset,
            0x1E => Self::ArrayCreate,
            0x1F => Self::ArrayLength,
            0x20 => Self::ArrayGetElement,
            0x21 => Self::ArraySetElement,
            0x22 => Self::ArrayFindElement,
            0x23 => Self::ArrayRfindElement,
            0x24 => Self::Is,
            0x25 => Self::StructCreate,
            0x26 => Self::StructGet,
            0x27 => Self::StructSet,
            0x28 => Self::StructFind,
            0x29 => Self::StructRFind,
            0x2A => Self::ArrayAdd,
            0x2B => Self::ArrayInsert,
            0x2C => Self::ArrayRemoveLast,
            0x2D => Self::ArrayRemove,
            0x2E => Self::ArrayClear,
            0x2F => Self::GetAllMatchingStruct,
            0x30 => Self::GuardLock,
            0x31 => Self::GuardUnlock,
            0x32 => Self::GuardTryLock,
            other => Self::Unknown(other),
        }
    }

    pub fn to_u8(self) -> u8 {
        match self {
            Self::Nop => 0x00,
            Self::Iadd => 0x01,
            Self::Fadd => 0x02,
            Self::Isub => 0x03,
            Self::Fsub => 0x04,
            Self::Imul => 0x05,
            Self::Fmul => 0x06,
            Self::Idiv => 0x07,
            Self::Fdiv => 0x08,
            Self::Imod => 0x09,
            Self::Not => 0x0A,
            Self::Ineg => 0x0B,
            Self::Fneg => 0x0C,
            Self::Assign => 0x0D,
            Self::Cast => 0x0E,
            Self::CmpEq => 0x0F,
            Self::CmpLt => 0x10,
            Self::CmpLte => 0x11,
            Self::CmpGt => 0x12,
            Self::CmpGte => 0x13,
            Self::Jump => 0x14,
            Self::Jz => 0x15,
            Self::Jnz => 0x16,
            Self::Callmethod => 0x17,
            Self::Callparent => 0x18,
            Self::Callstatic => 0x19,
            Self::Return => 0x1A,
            Self::Strcat => 0x1B,
            Self::Propget => 0x1C,
            Self::Propset => 0x1D,
            Self::ArrayCreate => 0x1E,
            Self::ArrayLength => 0x1F,
            Self::ArrayGetElement => 0x20,
            Self::ArraySetElement => 0x21,
            Self::ArrayFindElement => 0x22,
            Self::ArrayRfindElement => 0x23,
            Self::Is => 0x24,
            Self::StructCreate => 0x25,
            Self::StructGet => 0x26,
            Self::StructSet => 0x27,
            Self::StructFind => 0x28,
            Self::StructRFind => 0x29,
            Self::ArrayAdd => 0x2A,
            Self::ArrayInsert => 0x2B,
            Self::ArrayRemoveLast => 0x2C,
            Self::ArrayRemove => 0x2D,
            Self::ArrayClear => 0x2E,
            Self::GetAllMatchingStruct => 0x2F,
            Self::GuardLock => 0x30,
            Self::GuardUnlock => 0x31,
            Self::GuardTryLock => 0x32,
            Self::Unknown(raw) => raw,
        }
    }

    pub fn mnemonic(self) -> &'static str {
        match self {
            Self::Nop => "nop",
            Self::Iadd => "iadd",
            Self::Fadd => "fadd",
            Self::Isub => "isub",
            Self::Fsub => "fsub",
            Self::Imul => "imul",
            Self::Fmul => "fmul",
            Self::Idiv => "idiv",
            Self::Fdiv => "fdiv",
            Self::Imod => "imod",
            Self::Not => "not",
            Self::Ineg => "ineg",
            Self::Fneg => "fneg",
            Self::Assign => "assign",
            Self::Cast => "cast",
            Self::CmpEq => "cmpeq",
            Self::CmpLt => "cmplt",
            Self::CmpLte => "cmplte",
            Self::CmpGt => "cmpgt",
            Self::CmpGte => "cmpgte",
            Self::Jump => "jump",
            Self::Jz => "jz",
            Self::Jnz => "jnz",
            Self::Callmethod => "callmethod",
            Self::Callparent => "callparent",
            Self::Callstatic => "callstatic",
            Self::Return => "return",
            Self::Strcat => "strcat",
            Self::Propget => "propget",
            Self::Propset => "propset",
            Self::ArrayCreate => "array_create",
            Self::ArrayLength => "array_length",
            Self::ArrayGetElement => "array_getelement",
            Self::ArraySetElement => "array_setelement",
            Self::ArrayFindElement => "array_findelement",
            Self::ArrayRfindElement => "array_rfindelement",
            Self::Is => "is",
            Self::StructCreate => "struct_create",
            Self::StructGet => "struct_get",
            Self::StructSet => "struct_set",
            Self::StructFind => "struct_find",
            Self::StructRFind => "struct_rfind",
            Self::ArrayAdd => "array_add",
            Self::ArrayInsert => "array_insert",
            Self::ArrayRemoveLast => "array_removelast",
            Self::ArrayRemove => "array_remove",
            Self::ArrayClear => "array_clear",
            Self::GetAllMatchingStruct => "getallmatchingstruct",
            Self::GuardLock => "guardlock",
            Self::GuardUnlock => "guardunlock",
            Self::GuardTryLock => "guardtrylock",
            Self::Unknown(_) => "unknown",
        }
    }

    /// 固定基础参数数量（严格对应 Delphi `instructionData: array [0 .. $32] of integer`）
    pub fn fixed_arg_count(self) -> usize {
        match self {
            Self::Nop => 0,
            Self::Iadd | Self::Fadd | Self::Isub | Self::Fsub | Self::Imul | Self::Fmul
            | Self::Idiv | Self::Fdiv | Self::Imod => 3,
            Self::Not | Self::Ineg | Self::Fneg | Self::Assign | Self::Cast => 2,
            Self::CmpEq | Self::CmpLt | Self::CmpLte | Self::CmpGt | Self::CmpGte => 3,
            Self::Jump => 1,
            Self::Jz | Self::Jnz => 2,
            Self::Callmethod => 4, // method, target, result, arg_count (+ args)
            Self::Callparent => 3, // method, result, arg_count (+ args)
            Self::Callstatic => 4, // class, method, result, arg_count (+ args)
            Self::Return => 1,
            Self::Strcat | Self::Propget | Self::Propset => 3,
            Self::ArrayCreate | Self::ArrayLength => 2,
            Self::ArrayGetElement | Self::ArraySetElement => 3,
            Self::ArrayFindElement | Self::ArrayRfindElement => 4,
            Self::Is => 3,
            Self::StructCreate => 1,
            Self::StructGet | Self::StructSet => 3,
            Self::StructFind | Self::StructRFind => 5,
            Self::ArrayAdd | Self::ArrayInsert => 3,
            Self::ArrayRemoveLast => 1,
            Self::ArrayRemove => 3,
            Self::ArrayClear => 1,
            Self::GetAllMatchingStruct => 6,
            Self::GuardLock | Self::GuardUnlock => 1, // guard_count (+ guards)
            Self::GuardTryLock => 2,                  // dest, guard_count (+ guards)
            Self::Unknown(_) => 0,
        }
    }

    /// 是否为变长参数指令（严格对应 Delphi `extendedproc = [$17, $18, $19, $30, $31, $32]`）
    pub fn is_extended_proc(self) -> bool {
        matches!(
            self,
            Self::Callmethod
                | Self::Callparent
                | Self::Callstatic
                | Self::GuardLock
                | Self::GuardUnlock
                | Self::GuardTryLock
        )
    }

    /// 判断指令是否属于特定游戏体系（GameID: 1=Skyrim, 2=FO4, 3=FO76, 4=Starfield）
    pub fn is_supported_in_game(self, game_id: u16) -> bool {
        let raw = self.to_u8();
        match game_id {
            // Skyrim / Skyrim SE / Skyrim VR (GameID = 1)
            1 => raw <= 0x23,
            // Fallout 4 (GameID = 2) 或 Fallout 76 (GameID = 3)
            2 | 3 => raw <= 0x2E,
            // Starfield (GameID = 4 及更高)
            _ => raw <= 0x32,
        }
    }
}

// ── 语法与模型结构 ──────────────────────────────────────────────────

/// 变量定义
#[derive(Clone, Debug)]
pub struct PexVariable {
    pub name: String,
    pub type_name: String,
    pub flags: u32,
    pub doc: String,
    pub user_flags: u32,
    pub default_value: VarValue,
}

/// 变量默认值
#[derive(Clone, Debug)]
pub enum VarValue {
    None,
    Bool(bool),
    Integer(u32),
    Float(f32),
    String(String),
    Array(Vec<VarValue>),
}

/// 属性定义
#[derive(Clone, Debug)]
pub struct PexProperty {
    pub name: String,
    pub type_name: String,
    pub doc: String,
    pub flags: u32,
    pub user_flags: u8,
    pub auto_var: Option<u16>,
    pub default_value: VarValue,
}

/// 属性组
#[derive(Clone, Debug)]
pub struct PexPropertyGroup {
    pub name: String,
    pub doc: String,
    pub flags: u32,
    pub properties: Vec<PexProperty>,
}

/// 函数参数
#[derive(Clone, Debug)]
pub struct PexParam {
    pub name: String,
    pub type_name: String,
}

/// 函数中的局部变量
#[derive(Clone, Debug)]
pub struct PexLocal {
    pub name: String,
    pub type_name: String,
}

/// 函数定义
#[derive(Clone, Debug)]
pub struct PexFunction {
    pub name: String,
    pub return_type: String,
    pub doc: String,
    pub flags: u8,
    pub user_flags: Vec<(String, u8)>,
    pub params: Vec<PexParam>,
    pub locals: Vec<PexLocal>,
    pub instructions: Vec<Instruction>,
}

/// 状态定义
#[derive(Clone, Debug)]
pub struct PexState {
    pub name: String,
    pub functions: Vec<PexFunction>,
}

/// Guard 定义
#[derive(Clone, Debug)]
pub struct PexGuard {
    pub name: String,
    pub user_flags: Vec<u32>,
}

/// 完全解析的对象
#[derive(Clone, Debug)]
pub struct PexObject {
    pub name: String,
    pub parent_class: String,
    pub doc: String,
    pub user_flags: Vec<(String, u8)>,
    pub auto_state_name: String,
    pub variables: Vec<PexVariable>,
    pub guards: Vec<PexGuard>,
    pub property_groups: Vec<PexPropertyGroup>,
    pub states: Vec<PexState>,
}

/// 完全反编译的 PEX
#[derive(Clone, Debug)]
pub struct DecompiledPex {
    pub header: PexHeader,
    pub game_id: u16,
    pub major_version: u8,
    pub minor_version: u8,
    pub compile_time: u64,
    pub source_file_name: String,
    pub user_name: String,
    pub computer_name: String,
    pub objects: Vec<PexObject>,
    pub string_table: Vec<PexStringEntry>,
}

type StrTab = Vec<PexStringEntry>;

fn lookup(st: &StrTab, idx: u16) -> String {
    st.get(idx as usize)
        .map(|e| e.text.clone())
        .unwrap_or_default()
}

// ── 解析实现 ────────────────────────────────────────────────────────

fn parse_pex_value<R: Read>(r: &mut PexReader<R>, st: &StrTab) -> io::Result<PexValue> {
    let type_flag = r.read_u8()?;
    match type_flag {
        0 => Ok(PexValue::None),
        1 => {
            let idx = r.read_u16()?;
            Ok(PexValue::Identifier(lookup(st, idx)))
        }
        2 => {
            let idx = r.read_u16()?;
            Ok(PexValue::StringLiteral(lookup(st, idx)))
        }
        3 => {
            let val = r.read_i32()?;
            Ok(PexValue::Integer(val))
        }
        4 => {
            let val = r.read_f32()?;
            Ok(PexValue::Float(val))
        }
        5 => {
            let val = r.read_u8()?;
            Ok(PexValue::Bool(val != 0))
        }
        other => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Unknown PEX variable type flag: {}", other),
        )),
    }
}

fn parse_var_value<R: Read>(r: &mut PexReader<R>, st: &StrTab) -> io::Result<VarValue> {
    let type_tag = r.read_u8()?;
    match type_tag {
        0 => Ok(VarValue::None),
        1 => {
            let idx = r.read_u16()?;
            Ok(VarValue::String(lookup(st, idx)))
        }
        2 => {
            let idx = r.read_u16()?;
            Ok(VarValue::String(lookup(st, idx)))
        }
        3 => {
            let val = r.read_u32()?;
            Ok(VarValue::Integer(val))
        }
        4 => {
            let val = r.read_f32()?;
            Ok(VarValue::Float(val))
        }
        5 => {
            let val = r.read_u8()?;
            Ok(VarValue::Bool(val != 0))
        }
        6 => {
            let count = r.read_u32()? as usize;
            let mut arr = Vec::with_capacity(count);
            for _ in 0..count {
                arr.push(parse_var_value(r, st)?);
            }
            Ok(VarValue::Array(arr))
        }
        _ => Ok(VarValue::None),
    }
}

fn parse_instruction<R: Read>(r: &mut PexReader<R>, st: &StrTab) -> io::Result<Instruction> {
    let raw_opcode = r.read_u8()?;
    let opcode = Opcode::from_u8(raw_opcode);
    let fixed_count = opcode.fixed_arg_count();
    let mut args = Vec::with_capacity(fixed_count);

    let mut extra_args_count: i32 = 0;
    for _ in 0..fixed_count {
        let val = parse_pex_value(r, st)?;
        if let PexValue::Integer(i) = val {
            extra_args_count = i;
        } else {
            extra_args_count = 0;
        }
        args.push(val);
    }

    if opcode.is_extended_proc() && extra_args_count > 0 {
        for _ in 0..extra_args_count {
            args.push(parse_pex_value(r, st)?);
        }
    }

    Ok(Instruction {
        opcode,
        raw_opcode,
        args,
    })
}

/// 将 PEX 二进制文件完全反编译为结构化类型（严格支持真实 Bethesda PEX 规范）。
pub fn decompile_pex(data: &[u8]) -> io::Result<DecompiledPex> {
    if data.len() < 16 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "PEX file too short",
        ));
    }

    let endian = if data[0..4] == [0xFA, 0x57, 0xC0, 0xDE] {
        PexEndian::BigEndian
    } else if data[0..4] == [0xDE, 0xC0, 0x57, 0xFA] {
        PexEndian::LittleEndian
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid PEX magic: {:?}", &data[0..4]),
        ));
    };

    let raw_magic = match endian {
        PexEndian::BigEndian => BigEndian::read_u32(&data[0..4]),
        PexEndian::LittleEndian => LittleEndian::read_u32(&data[0..4]),
    };

    let mut cur = Cursor::new(&data[4..]);
    let mut r = PexReader::new(&mut cur, endian);

    let major_version = r.read_u8()?;
    let minor_version = r.read_u8()?;
    let game_id = r.read_u16()?;
    let compile_time = r.read_u64()?;

    let source_file_name = r.read_string()?;
    let user_name = r.read_string()?;
    let computer_name = r.read_string()?;

    let header = PexHeader {
        magic: raw_magic,
        endian,
        major_version,
        minor_version,
        game_id,
        compile_time,
        source_file_name: source_file_name.clone(),
        user_name: user_name.clone(),
        computer_name: computer_name.clone(),
    };

    // 字符串表
    let st_count = r.read_u16()? as usize;
    let mut string_table = Vec::with_capacity(st_count);
    for i in 0..st_count {
        let text = r.read_string()?;
        string_table.push(PexStringEntry {
            index: i as u16,
            text,
        });
    }

    // 调试信息（跳过）
    let has_debug = r.read_u8().unwrap_or(0);
    if has_debug == 1 {
        let _debug_mod_time = r.read_u64()?;
        let debug_func_count = r.read_u16()? as usize;
        for _ in 0..debug_func_count {
            let _obj_name = r.read_u16()?;
            let _state_name = r.read_u16()?;
            let _func_name = r.read_u16()?;
            let _func_type = r.read_u8()?;
            let line_count = r.read_u16()? as usize;
            let mut skip_buf = vec![0u8; line_count * 2];
            r.read_exact(&mut skip_buf)?;
        }

        if endian == PexEndian::LittleEndian {
            let group_count = r.read_u16().unwrap_or(0) as usize;
            for _ in 0..group_count {
                let _obj_idx = r.read_u16()?;
                let _state_idx = r.read_u16()?;
                let _func_idx = r.read_u16()?;
                let _group_type = r.read_u32()?;
                let prop_count = r.read_u16()? as usize;
                let mut skip_buf = vec![0u8; prop_count * 2];
                r.read_exact(&mut skip_buf)?;
            }

            let struct_count = r.read_u16().unwrap_or(0) as usize;
            for _ in 0..struct_count {
                let _obj_idx = r.read_u16()?;
                let _state_idx = r.read_u16()?;
                let count = r.read_u16()? as usize;
                let mut skip_buf = vec![0u8; count * 2];
                r.read_exact(&mut skip_buf)?;
            }
        }
    }

    // 用户标志（跳过）
    let uf_count = r.read_u16().unwrap_or(0) as usize;
    for _ in 0..uf_count {
        let _n = r.read_u16()?;
        let _f = r.read_u8()?;
    }

    // 对象
    let obj_count = r.read_u16().unwrap_or(0) as usize;
    let st = &string_table;
    let mut objects = Vec::with_capacity(obj_count);

    for _ in 0..obj_count {
        let name_idx = r.read_u16()?;
        let obj_name = lookup(st, name_idx);
        let body_size = r.read_u32()? as usize;
        let mut body = vec![0u8; body_size];
        r.read_exact(&mut body)?;

        let mut body_cur = Cursor::new(&body[..]);
        let mut body_r = PexReader::new(&mut body_cur, endian);
        let obj = parse_object_body_full(&mut body_r, &obj_name, st)?;
        objects.push(obj);
    }

    Ok(DecompiledPex {
        header,
        game_id,
        major_version,
        minor_version,
        compile_time,
        source_file_name,
        user_name,
        computer_name,
        objects,
        string_table,
    })
}

fn parse_object_body_full<R: Read>(
    r: &mut PexReader<R>,
    obj_name: &str,
    st: &StrTab,
) -> io::Result<PexObject> {
    let parent_idx = r.read_u16()?;
    let parent_class = lookup(st, parent_idx);

    let doc_idx = r.read_u16()?;
    let doc = lookup(st, doc_idx);

    // 用户标志
    let uf_count = r.read_u16()? as usize;
    let mut user_flags = Vec::with_capacity(uf_count);
    for _ in 0..uf_count {
        let name_idx = r.read_u16()?;
        let flag = r.read_u8()?;
        user_flags.push((lookup(st, name_idx), flag));
    }

    let auto_state_idx = r.read_u16()?;
    let auto_state_name = lookup(st, auto_state_idx);

    // 变量
    let var_count = r.read_u16()? as usize;
    let mut variables = Vec::with_capacity(var_count);
    for _ in 0..var_count {
        let name = lookup(st, r.read_u16()?);
        let type_name = lookup(st, r.read_u16()?);
        let flags = r.read_u32()?;
        let doc = lookup(st, r.read_u16()?);
        let user_flags = r.read_u32()?;
        let default_value = parse_var_value(r, st)?;
        variables.push(PexVariable {
            name,
            type_name,
            flags,
            doc,
            user_flags,
            default_value,
        });
    }

    // Guards (Starfield / FO4)
    let mut guards = Vec::new();
    if r.endian == PexEndian::LittleEndian {
        let guard_count = r.read_u16().unwrap_or(0) as usize;
        for _ in 0..guard_count {
            let name = lookup(st, r.read_u16()?);
            let sc = r.read_u32()? as usize;
            let mut user_flags = Vec::with_capacity(sc);
            for _ in 0..sc {
                user_flags.push(r.read_u16()? as u32);
            }
            guards.push(PexGuard { name, user_flags });
        }
    }

    // 属性组
    let pg_count = r.read_u16().unwrap_or(0) as usize;
    let mut property_groups = Vec::with_capacity(pg_count);
    for _ in 0..pg_count {
        let name = lookup(st, r.read_u16()?);
        let doc = lookup(st, r.read_u16()?);
        let flags = r.read_u32()?;

        let prop_count = r.read_u16()? as usize;
        let mut properties = Vec::with_capacity(prop_count);
        for _ in 0..prop_count {
            let prop = parse_property(r, st)?;
            properties.push(prop);
        }
        property_groups.push(PexPropertyGroup {
            name,
            doc,
            flags,
            properties,
        });
    }

    // 状态
    let state_count = r.read_u16().unwrap_or(0) as usize;
    let mut states = Vec::with_capacity(state_count);
    for _ in 0..state_count {
        let name = lookup(st, r.read_u16()?);
        let func_count = r.read_u16()? as usize;
        let mut functions = Vec::with_capacity(func_count);
        for _ in 0..func_count {
            let func = parse_function(r, st)?;
            functions.push(func);
        }
        states.push(PexState { name, functions });
    }

    Ok(PexObject {
        name: obj_name.to_string(),
        parent_class,
        doc,
        user_flags,
        auto_state_name,
        variables,
        guards,
        property_groups,
        states,
    })
}

fn parse_property<R: Read>(r: &mut PexReader<R>, st: &StrTab) -> io::Result<PexProperty> {
    let name = lookup(st, r.read_u16()?);
    let type_name = lookup(st, r.read_u16()?);
    let doc = lookup(st, r.read_u16()?);
    let flags = r.read_u32()?;
    let user_flags = r.read_u8()?;
    let has_auto = r.read_u8()?;
    let auto_var = if has_auto != 0 {
        Some(r.read_u16()?)
    } else {
        None
    };
    let default_value = parse_var_value(r, st)?;
    Ok(PexProperty {
        name,
        type_name,
        doc,
        flags,
        user_flags,
        auto_var,
        default_value,
    })
}

fn parse_function<R: Read>(r: &mut PexReader<R>, st: &StrTab) -> io::Result<PexFunction> {
    let name = lookup(st, r.read_u16()?);
    let return_type = lookup(st, r.read_u16()?);
    let doc = lookup(st, r.read_u16()?);
    let flags = r.read_u8()?;

    let fuf_count = r.read_u16()? as usize;
    let mut user_flags = Vec::with_capacity(fuf_count);
    for _ in 0..fuf_count {
        let un = lookup(st, r.read_u16()?);
        let uf = r.read_u8()?;
        user_flags.push((un, uf));
    }

    let param_count = r.read_u16()? as usize;
    let mut params = Vec::with_capacity(param_count);
    for _ in 0..param_count {
        let pn = lookup(st, r.read_u16()?);
        let pt = lookup(st, r.read_u16()?);
        params.push(PexParam {
            name: pn,
            type_name: pt,
        });
    }

    let local_count = r.read_u16()? as usize;
    let mut locals = Vec::with_capacity(local_count);
    for _ in 0..local_count {
        let ln = lookup(st, r.read_u16()?);
        let lt = lookup(st, r.read_u16()?);
        locals.push(PexLocal {
            name: ln,
            type_name: lt,
        });
    }

    let inst_count = r.read_u16()? as usize;
    let mut instructions = Vec::with_capacity(inst_count);
    for _ in 0..inst_count {
        instructions.push(parse_instruction(r, st)?);
    }

    Ok(PexFunction {
        name,
        return_type,
        doc,
        flags,
        user_flags,
        params,
        locals,
        instructions,
    })
}

// ── 伪代码发射器 ────────────────────────────────────────────────────

/// 从反编译的 PEX 发射类似于 Papyrus 的伪代码（格式完全对齐 Delphi xTranslator）。
pub fn emit_pseudocode(pex: &DecompiledPex) -> String {
    let mut out = String::with_capacity(4096);

    for obj in &pex.objects {
        emit_object(&mut out, obj);
    }

    out
}

fn emit_object(out: &mut String, obj: &PexObject) {
    let _ = write!(out, "ScriptName {}", obj.name);
    if !obj.parent_class.is_empty() {
        let _ = write!(out, " Extends {}", obj.parent_class);
    }
    out.push('\n');

    if !obj.doc.is_empty() {
        let _ = writeln!(out, "; {}", obj.doc);
        out.push('\n');
    }

    // 变量
    for var in &obj.variables {
        if !var.doc.is_empty() {
            let _ = writeln!(out, "; {}", var.doc);
        }
        let _ = write!(out, "{} {}", var.type_name, var.name);
        match &var.default_value {
            VarValue::None => {}
            VarValue::Bool(b) => {
                let _ = write!(out, " = {}", if *b { "true" } else { "false" });
            }
            VarValue::Integer(i) => {
                let _ = write!(out, " = {}", i);
            }
            VarValue::Float(f) => {
                let _ = write!(out, " = {:.4}", f);
            }
            VarValue::String(s) => {
                let _ = write!(out, " = \"{}\"", s);
            }
            VarValue::Array(_) => {
                out.push_str(" = new ...[]");
            }
        }
        out.push('\n');
    }
    if !obj.variables.is_empty() {
        out.push('\n');
    }

    // 属性组
    for pg in &obj.property_groups {
        if !pg.name.is_empty() {
            let _ = writeln!(out, "; Group {}", pg.name);
        }
        for prop in &pg.properties {
            emit_property(out, prop);
        }
        if !pg.name.is_empty() {
            let _ = writeln!(out, "; EndGroup");
        }
        out.push('\n');
    }

    // 状态
    for state in &obj.states {
        if state.name.is_empty() {
            for func in &state.functions {
                emit_function(out, func);
                out.push('\n');
            }
        } else {
            let _ = writeln!(out, "State {}", state.name);
            for func in &state.functions {
                emit_function(out, func);
            }
            let _ = writeln!(out, "EndState");
            out.push('\n');
        }
    }
}

fn emit_property(out: &mut String, prop: &PexProperty) {
    if !prop.doc.is_empty() {
        let _ = writeln!(out, "; {}", prop.doc);
    }
    let _ = write!(out, "{} Property {}", prop.type_name, prop.name);
    match &prop.default_value {
        VarValue::None => {}
        VarValue::Bool(b) => {
            let _ = write!(out, " = {}", if *b { "true" } else { "false" });
        }
        VarValue::Integer(i) => {
            let _ = write!(out, " = {}", i);
        }
        VarValue::Float(f) => {
            let _ = write!(out, " = {:.4}", f);
        }
        VarValue::String(s) => {
            let _ = write!(out, " = \"{}\"", s);
        }
        VarValue::Array(_) => {}
    }
    out.push('\n');
    let _ = writeln!(out, "EndProperty");
}

fn emit_function(out: &mut String, func: &PexFunction) {
    if !func.doc.is_empty() {
        let _ = writeln!(out, "    ; {}", func.doc);
    }

    let ret = if func.return_type.is_empty() {
        String::from("Function")
    } else {
        format!("{} Function", func.return_type)
    };
    let _ = write!(out, "    {} {}", ret, func.name);
    out.push('(');
    for (i, p) in func.params.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        let _ = write!(out, "{} {}", p.type_name, p.name);
    }
    out.push(')');
    out.push('\n');

    for local in &func.locals {
        let _ = writeln!(out, "        {} {}", local.type_name, local.name);
    }
    if !func.locals.is_empty() {
        out.push('\n');
    }

    for inst in &func.instructions {
        emit_instruction(out, inst, &func.locals);
    }

    let _ = writeln!(out, "    EndFunction");
}

fn get_arg(inst: &Instruction, idx: usize) -> String {
    inst.args
        .get(idx)
        .map(|v| v.get_str_value(false))
        .unwrap_or_default()
}

fn get_var_type(var_name: &str, locals: &[PexLocal]) -> String {
    for local in locals {
        if local.name == var_name {
            return local.type_name.clone();
        }
    }
    String::new()
}

fn set_method_result(res: &str) -> String {
    if res == "::NoneVar" || res.is_empty() {
        String::new()
    } else {
        format!("{} = ", res)
    }
}

/// 严格按照 Delphi `tPexDecompiler.includeNewArray` 实现
fn include_new_array(type_str: &str, size_str: &str) -> String {
    if let Some(pos) = type_str.find(']') {
        let mut res = type_str.to_string();
        res.insert_str(pos, size_str);
        res
    } else {
        type_str.to_string()
    }
}

fn get_method_args(inst: &Instruction, start_idx: usize) -> String {
    let mut res = String::new();
    for i in start_idx..inst.args.len() {
        res.push_str(&inst.args[i].get_str_value(false));
        if i < inst.args.len() - 1 {
            res.push_str(", ");
        }
    }
    res
}

/// 严格按照 Delphi `tPexDecompiler.drawInstruction` 发射单条指令的伪代码
fn emit_instruction(out: &mut String, inst: &Instruction, locals: &[PexLocal]) {
    let strtmp = match inst.opcode {
        Opcode::Nop => "none".to_string(),
        Opcode::Iadd | Opcode::Fadd => {
            format!("{} = {} + {}", get_arg(inst, 0), get_arg(inst, 1), get_arg(inst, 2))
        }
        Opcode::Isub | Opcode::Fsub => {
            format!("{} = {} - {}", get_arg(inst, 0), get_arg(inst, 1), get_arg(inst, 2))
        }
        Opcode::Imul | Opcode::Fmul => {
            format!("{} = {} * {}", get_arg(inst, 0), get_arg(inst, 1), get_arg(inst, 2))
        }
        Opcode::Idiv | Opcode::Fdiv => {
            format!("{} = {} / {}", get_arg(inst, 0), get_arg(inst, 1), get_arg(inst, 2))
        }
        Opcode::Imod => {
            format!("{} = {} mod {}", get_arg(inst, 0), get_arg(inst, 1), get_arg(inst, 2))
        }
        Opcode::Not => {
            format!("{} = not {}", get_arg(inst, 0), get_arg(inst, 1))
        }
        Opcode::Ineg | Opcode::Fneg => {
            format!("{} = -{}", get_arg(inst, 0), get_arg(inst, 1))
        }
        Opcode::Assign => {
            format!("{} = {}", get_arg(inst, 0), get_arg(inst, 1))
        }
        Opcode::Cast => {
            let target_type = get_var_type(&get_arg(inst, 0), locals);
            format!("{} = {} as {}", get_arg(inst, 0), get_arg(inst, 1), target_type)
        }
        Opcode::CmpEq => {
            format!("{} = {} == {}", get_arg(inst, 0), get_arg(inst, 1), get_arg(inst, 2))
        }
        Opcode::CmpLt => {
            format!("{} = {} < {}", get_arg(inst, 0), get_arg(inst, 1), get_arg(inst, 2))
        }
        Opcode::CmpLte => {
            format!("{} = {} <= {}", get_arg(inst, 0), get_arg(inst, 1), get_arg(inst, 2))
        }
        Opcode::CmpGt => {
            format!("{} = {} > {}", get_arg(inst, 0), get_arg(inst, 1), get_arg(inst, 2))
        }
        Opcode::CmpGte => {
            format!("{} = {} >= {}", get_arg(inst, 0), get_arg(inst, 1), get_arg(inst, 2))
        }
        Opcode::Jump => {
            format!("jump {}", get_arg(inst, 0))
        }
        Opcode::Jz => {
            format!("if {} then jump {}", get_arg(inst, 0), get_arg(inst, 1))
        }
        Opcode::Jnz => {
            format!("if not {} then jump {}", get_arg(inst, 0), get_arg(inst, 1))
        }
        Opcode::Callmethod => {
            format!(
                "{}{}.{}({})",
                set_method_result(&get_arg(inst, 2)),
                get_arg(inst, 1),
                get_arg(inst, 0),
                get_method_args(inst, 4)
            )
        }
        Opcode::Callparent => {
            format!(
                "{}parent.{}({})",
                set_method_result(&get_arg(inst, 1)),
                get_arg(inst, 0),
                get_method_args(inst, 3)
            )
        }
        Opcode::Callstatic => {
            format!(
                "{}{}.{}({})",
                set_method_result(&get_arg(inst, 2)),
                get_arg(inst, 0),
                get_arg(inst, 1),
                get_method_args(inst, 4)
            )
        }
        Opcode::Return => {
            format!("return {}", get_arg(inst, 0))
        }
        Opcode::Strcat => {
            format!("{} = {} + {}", get_arg(inst, 0), get_arg(inst, 1), get_arg(inst, 2))
        }
        Opcode::Propget => {
            format!("{} = {}.{}", get_arg(inst, 2), get_arg(inst, 1), get_arg(inst, 0))
        }
        Opcode::Propset => {
            format!("{}.{} = {}", get_arg(inst, 1), get_arg(inst, 0), get_arg(inst, 2))
        }
        Opcode::ArrayCreate => {
            let var_type = get_var_type(&get_arg(inst, 0), locals);
            format!(
                "{} = new {}",
                get_arg(inst, 0),
                include_new_array(&var_type, &get_arg(inst, 1))
            )
        }
        Opcode::ArrayLength => {
            format!("{} = {}.length", get_arg(inst, 0), get_arg(inst, 1))
        }
        Opcode::ArrayGetElement => {
            format!("{} = {}[{}]", get_arg(inst, 0), get_arg(inst, 1), get_arg(inst, 2))
        }
        Opcode::ArraySetElement => {
            format!("{}[{}] = {}", get_arg(inst, 0), get_arg(inst, 1), get_arg(inst, 2))
        }
        Opcode::ArrayFindElement => {
            format!(
                "{} = {}.find({}, {})",
                get_arg(inst, 1),
                get_arg(inst, 0),
                get_arg(inst, 2),
                get_arg(inst, 3)
            )
        }
        Opcode::ArrayRfindElement => {
            format!(
                "{} = {}.rfind({}, {})",
                get_arg(inst, 1),
                get_arg(inst, 0),
                get_arg(inst, 2),
                get_arg(inst, 3)
            )
        }
        // Fallout 4
        Opcode::Is => {
            format!("{} = {} is {}", get_arg(inst, 0), get_arg(inst, 1), get_arg(inst, 2))
        }
        Opcode::StructCreate => {
            format!("new {}", get_arg(inst, 0))
        }
        Opcode::StructGet => {
            format!("{} = {}.{}", get_arg(inst, 0), get_arg(inst, 1), get_arg(inst, 2))
        }
        Opcode::StructSet => {
            format!("{}.{} = {}", get_arg(inst, 0), get_arg(inst, 1), get_arg(inst, 2))
        }
        Opcode::StructFind => {
            format!(
                "{} = {}.findstruct({}, {}, {})",
                get_arg(inst, 1),
                get_arg(inst, 0),
                get_arg(inst, 2),
                get_arg(inst, 3),
                get_arg(inst, 4)
            )
        }
        Opcode::StructRFind => {
            format!(
                "{} = {}.rfindstruct({}, {}, {})",
                get_arg(inst, 1),
                get_arg(inst, 0),
                get_arg(inst, 2),
                get_arg(inst, 3),
                get_arg(inst, 4)
            )
        }
        Opcode::ArrayAdd => {
            format!("{}.Add({}, {})", get_arg(inst, 0), get_arg(inst, 1), get_arg(inst, 2))
        }
        Opcode::ArrayInsert => {
            format!("{}.Insert({}, {})", get_arg(inst, 0), get_arg(inst, 1), get_arg(inst, 2))
        }
        Opcode::ArrayRemoveLast => {
            format!("{}.RemoveLast", get_arg(inst, 0))
        }
        Opcode::ArrayRemove => {
            format!("{}.Remove({}, {})", get_arg(inst, 0), get_arg(inst, 1), get_arg(inst, 2))
        }
        Opcode::ArrayClear => {
            format!("{}.Clear", get_arg(inst, 0))
        }
        // Starfield
        Opcode::GetAllMatchingStruct => {
            format!(
                "{}.GetAllMatchingStruct({}, {}, {}, {})",
                get_arg(inst, 0),
                get_arg(inst, 2),
                get_arg(inst, 3),
                get_arg(inst, 4),
                get_arg(inst, 5)
            )
        }
        Opcode::GuardLock => {
            format!("GuardLock({})", get_arg(inst, 1))
        }
        Opcode::GuardUnlock => {
            format!("GuardUnlock({})", get_arg(inst, 1))
        }
        Opcode::GuardTryLock => {
            format!("{} = GuardTryLock({})", get_arg(inst, 0), get_arg(inst, 2))
        }
        Opcode::Unknown(raw) => {
            format!("unknown OpCode: {:02x}", raw)
        }
    };

    let _ = writeln!(out, "        {}", strtmp);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pex::parser::PEX_MAGIC_BIG;
    use byteorder::{BigEndian, LittleEndian, WriteBytesExt};

    #[test]
    fn test_opcode_mnemonic() {
        assert_eq!(Opcode::Nop.mnemonic(), "nop");
        assert_eq!(Opcode::Return.mnemonic(), "return");
        assert_eq!(Opcode::Callmethod.mnemonic(), "callmethod");
        assert_eq!(Opcode::Is.mnemonic(), "is");
        assert_eq!(Opcode::GuardLock.mnemonic(), "guardlock");
    }

    #[test]
    fn test_game_version_modeling() {
        // Skyrim (GameID = 1) 只支持 0x00..=0x23
        assert!(Opcode::Iadd.is_supported_in_game(1));
        assert!(Opcode::ArrayRfindElement.is_supported_in_game(1));
        assert!(!Opcode::Is.is_supported_in_game(1));
        assert!(!Opcode::GuardLock.is_supported_in_game(1));

        // Fallout 4 (GameID = 2) 支持 0x00..=0x2E
        assert!(Opcode::Is.is_supported_in_game(2));
        assert!(Opcode::ArrayClear.is_supported_in_game(2));
        assert!(!Opcode::GetAllMatchingStruct.is_supported_in_game(2));
        assert!(!Opcode::GuardLock.is_supported_in_game(2));

        // Fallout 76 (GameID = 3) 支持 0x00..=0x2E
        assert!(Opcode::Is.is_supported_in_game(3));
        assert!(Opcode::ArrayClear.is_supported_in_game(3));
        assert!(!Opcode::GetAllMatchingStruct.is_supported_in_game(3));

        // Starfield (GameID = 4) 支持 0x00..=0x32
        assert!(Opcode::GetAllMatchingStruct.is_supported_in_game(4));
        assert!(Opcode::GuardLock.is_supported_in_game(4));
        assert!(Opcode::GuardTryLock.is_supported_in_game(4));
    }

    #[test]
    fn test_decompile_real_skyrim_big_endian_pex() {
        let mut data = Vec::new();

        // Magic 0xFA57C0DE (Big Endian)
        data.write_u32::<BigEndian>(PEX_MAGIC_BIG).unwrap();
        // Major=3, Minor=9, GameID=1 (Skyrim Big Endian), CompileTime=1600000000
        data.push(3);
        data.push(9);
        data.write_u16::<BigEndian>(1).unwrap();
        data.write_u64::<BigEndian>(1600000000).unwrap();

        // SourceFileName="Source/TestScript.psc", UserName="Builder", ComputerName="SkyrimDev"
        for s in &["Source/TestScript.psc", "Builder", "SkyrimDev"] {
            data.write_u16::<BigEndian>(s.len() as u16).unwrap();
            data.extend_from_slice(s.as_bytes());
        }

        // String Table: 10 strings
        let strings = [
            "",
            "TestScript",
            "",
            "",
            "",
            "Int",
            "count",
            "",
            "GetCount",
            "Int",
        ];
        data.write_u16::<BigEndian>(strings.len() as u16).unwrap();
        for s in strings {
            data.write_u16::<BigEndian>(s.len() as u16).unwrap();
            data.extend_from_slice(s.as_bytes());
        }

        // has_debug_info = 0
        data.push(0);

        // User flags: count = 0
        data.write_u16::<BigEndian>(0).unwrap();

        // Objects: count = 1
        data.write_u16::<BigEndian>(1).unwrap();
        // Object 0: name_idx = 1 ("TestScript")
        data.write_u16::<BigEndian>(1).unwrap();

        // Object Body
        let mut body = Vec::new();
        body.write_u16::<BigEndian>(0).unwrap(); // parent_class = ""
        body.write_u16::<BigEndian>(0).unwrap(); // doc = ""
        body.write_u16::<BigEndian>(0).unwrap(); // user_flags count = 0
        body.write_u16::<BigEndian>(0).unwrap(); // auto_state = ""

        // Variables: count = 0
        body.write_u16::<BigEndian>(0).unwrap();
        // Property groups: count = 0
        body.write_u16::<BigEndian>(0).unwrap();

        // States: count = 1
        body.write_u16::<BigEndian>(1).unwrap();
        // State 0: name_idx = 0 ("")
        body.write_u16::<BigEndian>(0).unwrap();
        // Functions: count = 1
        body.write_u16::<BigEndian>(1).unwrap();

        // Function 0:
        body.write_u16::<BigEndian>(8).unwrap(); // name_idx = 8 ("GetCount")
        body.write_u16::<BigEndian>(9).unwrap(); // return_type_idx = 9 ("Int")
        body.write_u16::<BigEndian>(0).unwrap(); // doc = ""
        body.push(0); // flags = 0
        body.write_u16::<BigEndian>(0).unwrap(); // user_flags = 0
        body.write_u16::<BigEndian>(0).unwrap(); // params = 0
        body.write_u16::<BigEndian>(0).unwrap(); // locals = 0

        // Instructions: count = 1 (Return 42)
        body.write_u16::<BigEndian>(1).unwrap();
        body.push(0x1A); // Opcode 0x1A (Return)
        body.push(3); // Argument: Integer (type = 3) -> 42
        body.write_i32::<BigEndian>(42).unwrap();

        // Write object body size + body
        data.write_u32::<BigEndian>(body.len() as u32).unwrap();
        data.extend_from_slice(&body);

        // Decompile
        let decompiled = decompile_pex(&data).expect("Must parse real Skyrim big-endian PEX");
        assert_eq!(decompiled.game_id, 1);
        assert_eq!(decompiled.source_file_name, "Source/TestScript.psc");
        assert_eq!(decompiled.user_name, "Builder");
        assert_eq!(decompiled.computer_name, "SkyrimDev");
        assert_eq!(decompiled.header.endian, PexEndian::BigEndian);

        let pseudo = emit_pseudocode(&decompiled);
        assert!(pseudo.contains("ScriptName TestScript"));
        assert!(pseudo.contains("Int Function GetCount()"));
        assert!(pseudo.contains("return 42"));
    }

    #[test]
    fn test_decompile_real_starfield_little_endian_pex() {
        let mut data = Vec::new();

        // Magic 0xFA57C0DE in Little Endian -> bytes [0xDE, 0xC0, 0x57, 0xFA]
        data.extend_from_slice(&[0xDE, 0xC0, 0x57, 0xFA]);
        // Major=3, Minor=9, GameID=4 (Starfield), CompileTime=0
        data.push(3);
        data.push(9);
        data.write_u16::<LittleEndian>(4).unwrap();
        data.write_u64::<LittleEndian>(0).unwrap();

        // Source, User, Computer
        for s in &["Source/SF_Guard.psc", "StarUser", "ShipStation"] {
            data.write_u16::<LittleEndian>(s.len() as u16).unwrap();
            data.extend_from_slice(s.as_bytes());
        }

        // String Table:
        let strings = ["", "GuardTest", "TestFunc", "None", "myGuard", "resVar"];
        data.write_u16::<LittleEndian>(strings.len() as u16).unwrap();
        for s in strings {
            data.write_u16::<LittleEndian>(s.len() as u16).unwrap();
            data.extend_from_slice(s.as_bytes());
        }

        // has_debug_info = 0
        data.push(0);

        // User flags
        data.write_u16::<LittleEndian>(0).unwrap();

        // Objects count = 1
        data.write_u16::<LittleEndian>(1).unwrap();
        data.write_u16::<LittleEndian>(1).unwrap(); // name = "GuardTest"

        let mut body = Vec::new();
        body.write_u16::<LittleEndian>(0).unwrap(); // parent
        body.write_u16::<LittleEndian>(0).unwrap(); // doc
        body.write_u16::<LittleEndian>(0).unwrap(); // user_flags
        body.write_u16::<LittleEndian>(0).unwrap(); // auto_state
        body.write_u16::<LittleEndian>(0).unwrap(); // variables
        body.write_u16::<LittleEndian>(0).unwrap(); // guards
        body.write_u16::<LittleEndian>(0).unwrap(); // property_groups

        // States count = 1
        body.write_u16::<LittleEndian>(1).unwrap();
        body.write_u16::<LittleEndian>(0).unwrap(); // State name = ""
        body.write_u16::<LittleEndian>(1).unwrap(); // Functions count = 1

        // Function 0: TestFunc
        body.write_u16::<LittleEndian>(2).unwrap(); // name = "TestFunc"
        body.write_u16::<LittleEndian>(3).unwrap(); // return_type = "None"
        body.write_u16::<LittleEndian>(0).unwrap(); // doc = ""
        body.push(0); // flags
        body.write_u16::<LittleEndian>(0).unwrap(); // user_flags
        body.write_u16::<LittleEndian>(0).unwrap(); // params
        body.write_u16::<LittleEndian>(0).unwrap(); // locals

        // Instructions count = 4:
        body.write_u16::<LittleEndian>(4).unwrap();

        // Inst 1: GuardLock
        body.push(0x30);
        body.push(3); // Integer type
        body.write_i32::<LittleEndian>(1).unwrap(); // count = 1
        body.push(1); // Ident type
        body.write_u16::<LittleEndian>(4).unwrap(); // "myGuard"

        // Inst 2: GuardTryLock
        body.push(0x32);
        body.push(1); // Ident type
        body.write_u16::<LittleEndian>(5).unwrap(); // "resVar"
        body.push(3); // Integer type
        body.write_i32::<LittleEndian>(1).unwrap(); // count = 1
        body.push(1); // Ident type
        body.write_u16::<LittleEndian>(4).unwrap(); // "myGuard"

        // Inst 3: GuardUnlock
        body.push(0x31);
        body.push(3); // Integer type
        body.write_i32::<LittleEndian>(1).unwrap(); // count = 1
        body.push(1); // Ident type
        body.write_u16::<LittleEndian>(4).unwrap(); // "myGuard"

        // Inst 4: Return
        body.push(0x1A);
        body.push(0); // None type

        // Write object body
        data.write_u32::<LittleEndian>(body.len() as u32).unwrap();
        data.extend_from_slice(&body);

        let decompiled = decompile_pex(&data).expect("Must parse Starfield PEX");
        assert_eq!(decompiled.game_id, 4);
        assert_eq!(decompiled.header.endian, PexEndian::LittleEndian);
        assert_eq!(decompiled.source_file_name, "Source/SF_Guard.psc");

        let pseudo = emit_pseudocode(&decompiled);
        assert!(pseudo.contains("GuardLock(myGuard)"));
        assert!(pseudo.contains("resVar = GuardTryLock(myGuard)"));
        assert!(pseudo.contains("GuardUnlock(myGuard)"));
    }
}
