//! PEX 反编译器 — 将 PEX 二进制文件解析为结构化类型，并输出与 Delphi xTranslator 完全等价的 Papyrus 伪代码。
//! 严格支持真实 Bethesda PEX 规范与大小端模式（Skyrim Big-Endian 与 FO4/Starfield Little-Endian）。
//! Object Body 解析严格对齐 Delphi `checkObjectData` / `checkVariables` / `checkFunction` / `checkProperty` 顺序。

use byteorder::{BigEndian, ByteOrder, LittleEndian};
use std::fmt::Write;
use std::io::{self, Cursor, Read};

use super::parser::{detect_endian, PexReader};
use super::types::{PexEndian, PexHeader, PexStringEntry};

// ── PexValue / Opcode / Instruction ──────────────────────────────────

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

#[derive(Clone, Debug, PartialEq)]
pub struct Instruction {
    pub opcode: Opcode,
    pub raw_opcode: u8,
    pub args: Vec<PexValue>,
}

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
    GetAllMatchingStruct = 0x2F,
    GuardLock = 0x30,
    GuardUnlock = 0x31,
    GuardTryLock = 0x32,
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
    pub fn fixed_arg_count(self) -> usize {
        match self {
            Self::Nop => 0,
            Self::Iadd
            | Self::Fadd
            | Self::Isub
            | Self::Fsub
            | Self::Imul
            | Self::Fmul
            | Self::Idiv
            | Self::Fdiv
            | Self::Imod => 3,
            Self::Not | Self::Ineg | Self::Fneg | Self::Assign | Self::Cast => 2,
            Self::CmpEq | Self::CmpLt | Self::CmpLte | Self::CmpGt | Self::CmpGte => 3,
            Self::Jump => 1,
            Self::Jz | Self::Jnz => 2,
            Self::Callmethod => 4,
            Self::Callparent => 3,
            Self::Callstatic => 4,
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
            Self::GuardLock | Self::GuardUnlock => 1,
            Self::GuardTryLock => 2,
            Self::Unknown(_) => 0,
        }
    }
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
    pub fn is_supported_in_game(self, game_id: u16) -> bool {
        let raw = self.to_u8();
        match game_id {
            1 => raw <= 0x23,     // Skyrim
            2 | 3 => raw <= 0x2E, // FO4 / FO76
            _ => raw <= 0x32,     // Starfield
        }
    }
}

// ── AST 类型 ────────────────────────────────────────────────────────

/// 变量默认值（仅在变量/属性声明中使用）
#[derive(Clone, Debug)]
pub enum VarValue {
    None,
    Bool(bool),
    Integer(u32),
    Float(f32),
    String(String),
    Array(Vec<VarValue>),
}

/// 变量定义（严格对齐 Delphi `checkVariables`）
#[derive(Clone, Debug)]
pub struct PexVariable {
    pub name: String,
    pub type_name: String,
    pub flags: u32,
    pub default_value: VarValue,
    // [LE] group: u8
    // [struct] docType: String
}

/// 函数参数（严格对齐 Delphi `checkVariabletype`）
#[derive(Clone, Debug)]
pub struct PexParam {
    pub name: String,
    pub type_name: String,
}

/// 局部变量
#[derive(Clone, Debug)]
pub struct PexLocal {
    pub name: String,
    pub type_name: String,
}

/// 函数定义（严格对齐 Delphi `checkFunction`）
#[derive(Clone, Debug)]
pub struct PexFunction {
    pub name: String,
    pub return_type: String,
    pub doc: String,
    pub uflags: u32,
    pub flags: u8,
    pub params: Vec<PexParam>,
    pub locals: Vec<PexLocal>,
    pub instructions: Vec<Instruction>,
}

/// 属性定义（严格对齐 Delphi `checkProperty`）
#[derive(Clone, Debug)]
pub struct PexProperty {
    pub name: String,
    pub type_name: String,
    pub doc: String,
    pub uflags: u32,
    pub flag: u8,
    pub auto_var_name: Option<String>,
    pub read_handler: Option<Box<PexFunction>>,
    pub write_handler: Option<Box<PexFunction>>,
}

/// 状态定义
#[derive(Clone, Debug)]
pub struct PexState {
    pub name: String,
    pub functions: Vec<PexFunction>,
}

/// 完全解析的对象（严格对齐 Delphi `checkObjectData`）
#[derive(Clone, Debug)]
pub struct PexObject {
    pub name: String,
    pub parent_class: String,
    pub doc: String,
    pub user_flags: u32,
    pub auto_state_name: String,
    pub variables: Vec<PexVariable>,
    pub guards: Vec<String>, // Starfield only: guard names
    pub properties: Vec<PexProperty>,
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
        1 | 2 => {
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

/// 变量解析（严格对齐 Delphi `checkVariables`）
fn parse_variable<R: Read>(
    r: &mut PexReader<R>,
    st: &StrTab,
    use_doc_string: bool,
) -> io::Result<PexVariable> {
    let name = lookup(st, r.read_u16()?);
    let type_name = lookup(st, r.read_u16()?);
    let flags = r.read_u32()?;
    let default_value = parse_var_value(r, st)?;
    // [LE] group byte
    if r.endian == PexEndian::LittleEndian {
        let _group = r.read_u8()?;
        // [struct] docType
        if use_doc_string {
            let _doc_type = r.read_u16()?;
        }
    }
    Ok(PexVariable {
        name,
        type_name,
        flags,
        default_value,
    })
}

/// 属性解析（严格对齐 Delphi `checkProperty`）
fn parse_property<R: Read>(r: &mut PexReader<R>, st: &StrTab) -> io::Result<PexProperty> {
    let name = lookup(st, r.read_u16()?);
    let type_name = lookup(st, r.read_u16()?);
    let doc = lookup(st, r.read_u16()?);
    let uflags = r.read_u32()?;
    let flag = r.read_u8()?;
    let mut auto_var_name = None;
    let mut read_handler = None;
    let mut write_handler = None;
    // auto var: flag bit 2 (0x04)
    if flag & 0x04 != 0 {
        auto_var_name = Some(lookup(st, r.read_u16()?));
    }
    // read handler: flag bit 0 (0x01)
    if flag & 0x01 != 0 {
        read_handler = Some(Box::new(parse_function_no_name(r, st)?));
    }
    // write handler: flag bit 1 (0x02)
    if flag & 0x02 != 0 {
        write_handler = Some(Box::new(parse_function_no_name(r, st)?));
    }
    Ok(PexProperty {
        name,
        type_name,
        doc,
        uflags,
        flag,
        auto_var_name,
        read_handler,
        write_handler,
    })
}

/// 函数解析（无函数名，对应 Delphi `checkFunction(getFirst=false)`）
fn parse_function_no_name<R: Read>(r: &mut PexReader<R>, st: &StrTab) -> io::Result<PexFunction> {
    let return_type = lookup(st, r.read_u16()?);
    let doc = lookup(st, r.read_u16()?);
    let uflags = r.read_u32()?;
    let flags = r.read_u8()?;
    let params = read_param_list(r, st)?;
    let locals = read_local_list(r, st)?;
    let instructions = read_instruction_list(r, st)?;
    Ok(PexFunction {
        name: String::new(),
        return_type,
        doc,
        uflags,
        flags,
        params,
        locals,
        instructions,
    })
}

/// 函数解析（含函数名，对应 Delphi `checkFunction(getFirst=true)`）
fn parse_function<R: Read>(r: &mut PexReader<R>, st: &StrTab) -> io::Result<PexFunction> {
    let name = lookup(st, r.read_u16()?);
    let return_type = lookup(st, r.read_u16()?);
    let doc = lookup(st, r.read_u16()?);
    let uflags = r.read_u32()?;
    let flags = r.read_u8()?;
    let params = read_param_list(r, st)?;
    let locals = read_local_list(r, st)?;
    let instructions = read_instruction_list(r, st)?;
    Ok(PexFunction {
        name,
        return_type,
        doc,
        uflags,
        flags,
        params,
        locals,
        instructions,
    })
}

fn read_param_list<R: Read>(r: &mut PexReader<R>, st: &StrTab) -> io::Result<Vec<PexParam>> {
    let count = r.read_u16()? as usize;
    let mut params = Vec::with_capacity(count);
    for _ in 0..count {
        let pn = lookup(st, r.read_u16()?);
        let pt = lookup(st, r.read_u16()?);
        params.push(PexParam {
            name: pn,
            type_name: pt,
        });
    }
    Ok(params)
}

fn read_local_list<R: Read>(r: &mut PexReader<R>, st: &StrTab) -> io::Result<Vec<PexLocal>> {
    let count = r.read_u16()? as usize;
    let mut locals = Vec::with_capacity(count);
    for _ in 0..count {
        let ln = lookup(st, r.read_u16()?);
        let lt = lookup(st, r.read_u16()?);
        locals.push(PexLocal {
            name: ln,
            type_name: lt,
        });
    }
    Ok(locals)
}

fn read_instruction_list<R: Read>(
    r: &mut PexReader<R>,
    st: &StrTab,
) -> io::Result<Vec<Instruction>> {
    let count = r.read_u16()? as usize;
    let mut instructions = Vec::with_capacity(count);
    for _ in 0..count {
        instructions.push(parse_instruction(r, st)?);
    }
    Ok(instructions)
}

/// 对象体解析（严格对齐 Delphi `checkObjectData`）
fn parse_object_body_full<R: Read>(
    r: &mut PexReader<R>,
    obj_name: &str,
    st: &StrTab,
    game_id: u16,
) -> io::Result<PexObject> {
    let parent_class = lookup(st, r.read_u16()?);
    let doc = lookup(st, r.read_u16()?);
    // [LE] uConst (u8)
    if r.endian == PexEndian::LittleEndian {
        let _ = r.read_u8()?;
    }
    let user_flags = r.read_u32()?;
    let auto_state_name = lookup(st, r.read_u16()?);
    // [LE] structs
    if r.endian == PexEndian::LittleEndian {
        let struct_count = r.read_u16().unwrap_or(0) as usize;
        for _ in 0..struct_count {
            let _struct_name = r.read_u16()?;
            let var_count = r.read_u16().unwrap_or(0) as usize;
            for _ in 0..var_count {
                parse_variable(r, st, true)?;
            }
        }
    }
    // Variables
    let var_count = r.read_u16().unwrap_or(0) as usize;
    let mut variables = Vec::with_capacity(var_count);
    for _ in 0..var_count {
        variables.push(parse_variable(r, st, false)?);
    }
    // [Starfield only, game_id == 4] Guards
    let mut guards = Vec::new();
    if game_id == 4 {
        let guard_count = r.read_u16().unwrap_or(0) as usize;
        for _ in 0..guard_count {
            guards.push(lookup(st, r.read_u16()?));
        }
    }
    // Properties
    let prop_count = r.read_u16().unwrap_or(0) as usize;
    let mut properties = Vec::with_capacity(prop_count);
    for _ in 0..prop_count {
        properties.push(parse_property(r, st)?);
    }
    // States
    let state_count = r.read_u16().unwrap_or(0) as usize;
    let mut states = Vec::with_capacity(state_count);
    for _ in 0..state_count {
        let name = lookup(st, r.read_u16()?);
        let func_count = r.read_u16()? as usize;
        let mut functions = Vec::with_capacity(func_count);
        for _ in 0..func_count {
            functions.push(parse_function(r, st)?);
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
        properties,
        states,
    })
}

/// 主入口：将 PEX 二进制文件反编译为结构化类型
pub fn decompile_pex(data: &[u8]) -> io::Result<DecompiledPex> {
    if data.len() < 16 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "PEX file too short",
        ));
    }

    let endian = detect_endian(data).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid PEX magic: {:?}", &data[0..4]),
        )
    })?;

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

    let st_count = r.read_u16()? as usize;
    let mut string_table = Vec::with_capacity(st_count);
    for i in 0..st_count {
        string_table.push(PexStringEntry {
            index: i as u16,
            text: r.read_string()?,
        });
    }

    // Debug info
    let has_debug = r.read_u8().unwrap_or(0);
    if has_debug == 1 {
        let _mod_time = r.read_u64()?;
        let func_count = r.read_u16()? as usize;
        for _ in 0..func_count {
            let _on = r.read_u16()?;
            let _sn = r.read_u16()?;
            let _fn = r.read_u16()?;
            let _ft = r.read_u8()?;
            let lc = r.read_u16()? as usize;
            let mut buf = vec![0u8; lc * 2];
            r.read_exact(&mut buf)?;
        }
        if endian == PexEndian::LittleEndian {
            let gc = r.read_u16().unwrap_or(0) as usize;
            for _ in 0..gc {
                let _oi = r.read_u16()?;
                let _si = r.read_u16()?;
                let _fi = r.read_u16()?;
                let _gt = r.read_u32()?;
                let pc = r.read_u16()? as usize;
                let mut buf = vec![0u8; pc * 2];
                r.read_exact(&mut buf)?;
            }
            let sc = r.read_u16().unwrap_or(0) as usize;
            for _ in 0..sc {
                let _oi = r.read_u16()?;
                let _si = r.read_u16()?;
                let c = r.read_u16()? as usize;
                let mut buf = vec![0u8; c * 2];
                r.read_exact(&mut buf)?;
            }
        }
    }

    // User flags
    let uf_count = r.read_u16().unwrap_or(0) as usize;
    for _ in 0..uf_count {
        let _n = r.read_u16()?;
        let _f = r.read_u8()?;
    }

    // Objects
    let obj_count = r.read_u16().unwrap_or(0) as usize;
    let st = &string_table;
    let mut objects = Vec::with_capacity(obj_count);
    for _ in 0..obj_count {
        let obj_name = lookup(st, r.read_u16()?);
        let raw_size = r.read_u32()? as usize;
        let body_size = raw_size.saturating_sub(4); // Delphi: readValue_Int - 4
        let mut body = vec![0u8; body_size];
        r.read_exact(&mut body)?;
        let mut bc = Cursor::new(&body[..]);
        let mut br = PexReader::new(&mut bc, endian);
        objects.push(parse_object_body_full(&mut br, &obj_name, st, game_id)?);
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

// ── 伪代码发射器 ────────────────────────────────────────────────────

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
    for var in &obj.variables {
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
    for prop in &obj.properties {
        emit_property(out, prop);
    }
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

fn emit_instruction(out: &mut String, inst: &Instruction, locals: &[PexLocal]) {
    let strtmp = match inst.opcode {
        Opcode::Nop => "none".to_string(),
        Opcode::Iadd | Opcode::Fadd => format!(
            "{} = {} + {}",
            get_arg(inst, 0),
            get_arg(inst, 1),
            get_arg(inst, 2)
        ),
        Opcode::Isub | Opcode::Fsub => format!(
            "{} = {} - {}",
            get_arg(inst, 0),
            get_arg(inst, 1),
            get_arg(inst, 2)
        ),
        Opcode::Imul | Opcode::Fmul => format!(
            "{} = {} * {}",
            get_arg(inst, 0),
            get_arg(inst, 1),
            get_arg(inst, 2)
        ),
        Opcode::Idiv | Opcode::Fdiv => format!(
            "{} = {} / {}",
            get_arg(inst, 0),
            get_arg(inst, 1),
            get_arg(inst, 2)
        ),
        Opcode::Imod => format!(
            "{} = {} mod {}",
            get_arg(inst, 0),
            get_arg(inst, 1),
            get_arg(inst, 2)
        ),
        Opcode::Not => format!("{} = not {}", get_arg(inst, 0), get_arg(inst, 1)),
        Opcode::Ineg | Opcode::Fneg => format!("{} = -{}", get_arg(inst, 0), get_arg(inst, 1)),
        Opcode::Assign => format!("{} = {}", get_arg(inst, 0), get_arg(inst, 1)),
        Opcode::Cast => {
            let t = get_var_type(&get_arg(inst, 0), locals);
            format!("{} = {} as {}", get_arg(inst, 0), get_arg(inst, 1), t)
        }
        Opcode::CmpEq => format!(
            "{} = {} == {}",
            get_arg(inst, 0),
            get_arg(inst, 1),
            get_arg(inst, 2)
        ),
        Opcode::CmpLt => format!(
            "{} = {} < {}",
            get_arg(inst, 0),
            get_arg(inst, 1),
            get_arg(inst, 2)
        ),
        Opcode::CmpLte => format!(
            "{} = {} <= {}",
            get_arg(inst, 0),
            get_arg(inst, 1),
            get_arg(inst, 2)
        ),
        Opcode::CmpGt => format!(
            "{} = {} > {}",
            get_arg(inst, 0),
            get_arg(inst, 1),
            get_arg(inst, 2)
        ),
        Opcode::CmpGte => format!(
            "{} = {} >= {}",
            get_arg(inst, 0),
            get_arg(inst, 1),
            get_arg(inst, 2)
        ),
        Opcode::Jump => format!("jump {}", get_arg(inst, 0)),
        Opcode::Jz => format!("if {} then jump {}", get_arg(inst, 0), get_arg(inst, 1)),
        Opcode::Jnz => format!("if not {} then jump {}", get_arg(inst, 0), get_arg(inst, 1)),
        Opcode::Callmethod => format!(
            "{}{}.{}({})",
            set_method_result(&get_arg(inst, 2)),
            get_arg(inst, 1),
            get_arg(inst, 0),
            get_method_args(inst, 4)
        ),
        Opcode::Callparent => format!(
            "{}parent.{}({})",
            set_method_result(&get_arg(inst, 1)),
            get_arg(inst, 0),
            get_method_args(inst, 3)
        ),
        Opcode::Callstatic => format!(
            "{}{}.{}({})",
            set_method_result(&get_arg(inst, 2)),
            get_arg(inst, 0),
            get_arg(inst, 1),
            get_method_args(inst, 4)
        ),
        Opcode::Return => format!("return {}", get_arg(inst, 0)),
        Opcode::Strcat => format!(
            "{} = {} + {}",
            get_arg(inst, 0),
            get_arg(inst, 1),
            get_arg(inst, 2)
        ),
        Opcode::Propget => format!(
            "{} = {}.{}",
            get_arg(inst, 2),
            get_arg(inst, 1),
            get_arg(inst, 0)
        ),
        Opcode::Propset => format!(
            "{}.{} = {}",
            get_arg(inst, 1),
            get_arg(inst, 0),
            get_arg(inst, 2)
        ),
        Opcode::ArrayCreate => {
            let t = get_var_type(&get_arg(inst, 0), locals);
            format!(
                "{} = new {}",
                get_arg(inst, 0),
                include_new_array(&t, &get_arg(inst, 1))
            )
        }
        Opcode::ArrayLength => format!("{} = {}.length", get_arg(inst, 0), get_arg(inst, 1)),
        Opcode::ArrayGetElement => format!(
            "{} = {}[{}]",
            get_arg(inst, 0),
            get_arg(inst, 1),
            get_arg(inst, 2)
        ),
        Opcode::ArraySetElement => format!(
            "{}[{}] = {}",
            get_arg(inst, 0),
            get_arg(inst, 1),
            get_arg(inst, 2)
        ),
        Opcode::ArrayFindElement => format!(
            "{} = {}.find({}, {})",
            get_arg(inst, 1),
            get_arg(inst, 0),
            get_arg(inst, 2),
            get_arg(inst, 3)
        ),
        Opcode::ArrayRfindElement => format!(
            "{} = {}.rfind({}, {})",
            get_arg(inst, 1),
            get_arg(inst, 0),
            get_arg(inst, 2),
            get_arg(inst, 3)
        ),
        Opcode::Is => format!(
            "{} = {} is {}",
            get_arg(inst, 0),
            get_arg(inst, 1),
            get_arg(inst, 2)
        ),
        Opcode::StructCreate => format!("new {}", get_arg(inst, 0)),
        Opcode::StructGet => format!(
            "{} = {}.{}",
            get_arg(inst, 0),
            get_arg(inst, 1),
            get_arg(inst, 2)
        ),
        Opcode::StructSet => format!(
            "{}.{} = {}",
            get_arg(inst, 0),
            get_arg(inst, 1),
            get_arg(inst, 2)
        ),
        Opcode::StructFind => format!(
            "{} = {}.findstruct({}, {}, {})",
            get_arg(inst, 1),
            get_arg(inst, 0),
            get_arg(inst, 2),
            get_arg(inst, 3),
            get_arg(inst, 4)
        ),
        Opcode::StructRFind => format!(
            "{} = {}.rfindstruct({}, {}, {})",
            get_arg(inst, 1),
            get_arg(inst, 0),
            get_arg(inst, 2),
            get_arg(inst, 3),
            get_arg(inst, 4)
        ),
        Opcode::ArrayAdd => format!(
            "{}.Add({}, {})",
            get_arg(inst, 0),
            get_arg(inst, 1),
            get_arg(inst, 2)
        ),
        Opcode::ArrayInsert => format!(
            "{}.Insert({}, {})",
            get_arg(inst, 0),
            get_arg(inst, 1),
            get_arg(inst, 2)
        ),
        Opcode::ArrayRemoveLast => format!("{}.RemoveLast", get_arg(inst, 0)),
        Opcode::ArrayRemove => format!(
            "{}.Remove({}, {})",
            get_arg(inst, 0),
            get_arg(inst, 1),
            get_arg(inst, 2)
        ),
        Opcode::ArrayClear => format!("{}.Clear", get_arg(inst, 0)),
        Opcode::GetAllMatchingStruct => format!(
            "{}.GetAllMatchingStruct({}, {}, {}, {})",
            get_arg(inst, 0),
            get_arg(inst, 2),
            get_arg(inst, 3),
            get_arg(inst, 4),
            get_arg(inst, 5)
        ),
        Opcode::GuardLock => format!("GuardLock({})", get_arg(inst, 1)),
        Opcode::GuardUnlock => format!("GuardUnlock({})", get_arg(inst, 1)),
        Opcode::GuardTryLock => {
            format!("{} = GuardTryLock({})", get_arg(inst, 0), get_arg(inst, 2))
        }
        Opcode::Unknown(raw) => format!("unknown OpCode: {:02x}", raw),
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
        assert!(Opcode::CmpEq.is_supported_in_game(1));
        assert!(!Opcode::Is.is_supported_in_game(1));
        assert!(Opcode::Is.is_supported_in_game(2));
        assert!(Opcode::ArrayClear.is_supported_in_game(2));
        assert!(Opcode::Is.is_supported_in_game(3));
        assert!(!Opcode::GuardLock.is_supported_in_game(3));
        assert!(Opcode::GuardLock.is_supported_in_game(4));
    }

    /// 构造一个最小真实 Skyrim PEX（Big-Endian，含完整 Object Body 结构）
    fn build_minimal_skyrim_pex() -> Vec<u8> {
        let mut data = Vec::new();
        data.write_u32::<BigEndian>(PEX_MAGIC_BIG).unwrap();
        data.push(3);
        data.push(9);
        data.write_u16::<BigEndian>(1).unwrap(); // GameID = 1 (Skyrim)
        data.write_u64::<BigEndian>(12345678).unwrap();
        for s in &["Source/Test.psc", "user", "machine"] {
            data.write_u16::<BigEndian>(s.len() as u16).unwrap();
            data.extend_from_slice(s.as_bytes());
        }
        let strings = &["", "TestScript", "GetCount", "Int", "result", "None"];
        data.write_u16::<BigEndian>(strings.len() as u16).unwrap();
        for s in strings {
            data.write_u16::<BigEndian>(s.len() as u16).unwrap();
            data.extend_from_slice(s.as_bytes());
        }
        data.push(0); // has_debug_info = 0
        data.write_u16::<BigEndian>(0).unwrap(); // user_flags
        data.write_u16::<BigEndian>(1).unwrap(); // objects count = 1
        data.write_u16::<BigEndian>(1).unwrap(); // name = "TestScript"

        let mut body = Vec::new();
        body.write_u16::<BigEndian>(0).unwrap(); // parentClass = ""
        body.write_u16::<BigEndian>(0).unwrap(); // docString = ""
        body.write_u32::<BigEndian>(0).unwrap(); // userFlags = 0
        body.write_u16::<BigEndian>(0).unwrap(); // autoStateName = ""
                                                 // variables = 0 (no structs for Skyrim/BigEndian)
        body.write_u16::<BigEndian>(0).unwrap();
        // properties = 0
        body.write_u16::<BigEndian>(0).unwrap();
        // states = 1
        body.write_u16::<BigEndian>(1).unwrap();
        body.write_u16::<BigEndian>(0).unwrap(); // state name = ""
        body.write_u16::<BigEndian>(1).unwrap(); // functions count = 1
                                                 // Function 0: "GetCount"
        body.write_u16::<BigEndian>(2).unwrap(); // name = "GetCount"
        body.write_u16::<BigEndian>(3).unwrap(); // return_type = "Int"
        body.write_u16::<BigEndian>(0).unwrap(); // doc = ""
        body.write_u32::<BigEndian>(0).unwrap(); // uFlags = 0
        body.push(0); // flags = 0
        body.write_u16::<BigEndian>(0).unwrap(); // params = 0
        body.write_u16::<BigEndian>(0).unwrap(); // locals = 0
                                                 // 1 instruction: Return 42
        body.write_u16::<BigEndian>(1).unwrap();
        body.push(0x1A); // Return
        body.push(3); // Integer type
        body.write_i32::<BigEndian>(42).unwrap();

        // size = body + 4 (size field itself)
        data.write_u32::<BigEndian>((body.len() as u32) + 4)
            .unwrap();
        data.extend_from_slice(&body);
        data
    }

    #[test]
    fn test_decompile_real_skyrim_big_endian_pex() {
        let data = build_minimal_skyrim_pex();
        let decompiled = decompile_pex(&data).expect("Must parse real Skyrim big-endian PEX");
        assert_eq!(decompiled.game_id, 1);
        assert_eq!(decompiled.source_file_name, "Source/Test.psc");
        assert_eq!(decompiled.header.endian, PexEndian::BigEndian);
        assert_eq!(decompiled.objects.len(), 1);
        let obj = &decompiled.objects[0];
        assert_eq!(obj.name, "TestScript");
        assert_eq!(obj.states.len(), 1);
        let func = &obj.states[0].functions[0];
        assert_eq!(func.name, "GetCount");
        assert_eq!(func.return_type, "Int");
        assert_eq!(func.instructions.len(), 1);
        assert_eq!(func.instructions[0].opcode, Opcode::Return);
        let pseudo = emit_pseudocode(&decompiled);
        assert!(pseudo.contains("ScriptName TestScript"));
        assert!(pseudo.contains("Int Function GetCount()"));
        assert!(pseudo.contains("return 42"));
    }

    #[test]
    fn test_decompile_real_starfield_little_endian_pex() {
        let mut data = Vec::new();
        data.extend_from_slice(&[0xDE, 0xC0, 0x57, 0xFA]); // Magic (Little Endian)
        data.push(3);
        data.push(9);
        data.write_u16::<LittleEndian>(4).unwrap(); // GameID = 4 (Starfield)
        data.write_u64::<LittleEndian>(0).unwrap();
        for s in &["Source/SF_Guard.psc", "StarUser", "ShipStation"] {
            data.write_u16::<LittleEndian>(s.len() as u16).unwrap();
            data.extend_from_slice(s.as_bytes());
        }
        let strings = &["", "GuardTest", "TestFunc", "None", "myGuard", "resVar"];
        data.write_u16::<LittleEndian>(strings.len() as u16)
            .unwrap();
        for s in strings {
            data.write_u16::<LittleEndian>(s.len() as u16).unwrap();
            data.extend_from_slice(s.as_bytes());
        }
        data.push(0); // has_debug_info = 0
        data.write_u16::<LittleEndian>(0).unwrap(); // user_flags
        data.write_u16::<LittleEndian>(1).unwrap(); // objects count = 1
        data.write_u16::<LittleEndian>(1).unwrap(); // name = "GuardTest"

        let mut body = Vec::new();
        body.write_u16::<LittleEndian>(0).unwrap(); // parentClass
        body.write_u16::<LittleEndian>(0).unwrap(); // docString
        body.push(0); // uConst (LE)
        body.write_u32::<LittleEndian>(0).unwrap(); // userFlags
        body.write_u16::<LittleEndian>(0).unwrap(); // autoStateName
                                                    // structs = 0 (LE)
        body.write_u16::<LittleEndian>(0).unwrap();
        // variables = 0
        body.write_u16::<LittleEndian>(0).unwrap();
        // Guards (Starfield only, game_id==4)
        body.write_u16::<LittleEndian>(1).unwrap(); // guard count = 1
        body.write_u16::<LittleEndian>(4).unwrap(); // "myGuard"
                                                    // properties = 0
        body.write_u16::<LittleEndian>(0).unwrap();
        // states = 1
        body.write_u16::<LittleEndian>(1).unwrap();
        body.write_u16::<LittleEndian>(0).unwrap(); // state name = ""
        body.write_u16::<LittleEndian>(1).unwrap(); // functions count = 1
                                                    // Function 0: TestFunc
        body.write_u16::<LittleEndian>(2).unwrap(); // name = "TestFunc"
        body.write_u16::<LittleEndian>(3).unwrap(); // return_type = "None"
        body.write_u16::<LittleEndian>(0).unwrap(); // doc = ""
        body.write_u32::<LittleEndian>(0).unwrap(); // uFlags = 0
        body.push(0); // flags = 0
        body.write_u16::<LittleEndian>(0).unwrap(); // params = 0
        body.write_u16::<LittleEndian>(0).unwrap(); // locals = 0

        // Instructions: GuardLock, GuardTryLock, GuardUnlock, Return
        body.write_u16::<LittleEndian>(4).unwrap();
        body.push(0x30); // GuardLock
        body.push(3);
        body.write_i32::<LittleEndian>(1).unwrap(); // count = 1
        body.push(1);
        body.write_u16::<LittleEndian>(4).unwrap(); // "myGuard"
        body.push(0x32); // GuardTryLock
        body.push(1);
        body.write_u16::<LittleEndian>(5).unwrap(); // "resVar"
        body.push(3);
        body.write_i32::<LittleEndian>(1).unwrap(); // count = 1
        body.push(1);
        body.write_u16::<LittleEndian>(4).unwrap(); // "myGuard"
        body.push(0x31); // GuardUnlock
        body.push(3);
        body.write_i32::<LittleEndian>(1).unwrap(); // count = 1
        body.push(1);
        body.write_u16::<LittleEndian>(4).unwrap(); // "myGuard"
        body.push(0x1A); // Return
        body.push(0); // None

        data.write_u32::<LittleEndian>((body.len() as u32) + 4)
            .unwrap();
        data.extend_from_slice(&body);

        let decompiled = decompile_pex(&data).expect("Must parse Starfield PEX");
        assert_eq!(decompiled.game_id, 4);
        assert_eq!(decompiled.header.endian, PexEndian::LittleEndian);
        assert_eq!(decompiled.source_file_name, "Source/SF_Guard.psc");
        assert_eq!(decompiled.objects[0].guards, vec!["myGuard"]);
        let pseudo = emit_pseudocode(&decompiled);
        assert!(pseudo.contains("GuardLock(myGuard)"));
        assert!(pseudo.contains("resVar = GuardTryLock(myGuard)"));
        assert!(pseudo.contains("GuardUnlock(myGuard)"));
    }
}
