//! PEX 反编译器 — 将 PEX 二进制文件解析为结构化类型，并输出类似于 Papyrus 的伪代码。

use byteorder::{LittleEndian, ReadBytesExt};
use std::fmt::Write;
use std::io::{self, Cursor, Read};

use super::types::PexStringEntry;

// ── 结构化类型 ──────────────────────────────────────────────────────

/// 解码后的 PEX 指令
#[derive(Clone, Debug)]
pub struct Instruction {
    pub opcode: Opcode,
    pub args: Vec<u16>,
}

/// 所有已知的 Papyrus 操作码
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    Cmplt = 0x0F,
    CmpEq = 0x10,
    CmpLte = 0x11,
    Cmpgt = 0x12,
    Cmpgte = 0x13,
    Cmpneq = 0x14,
    Jump = 0x15,
    Jz = 0x16,
    Jnz = 0x17,
    Callmethod = 0x18,
    Callparent = 0x19,
    Callstatic = 0x1A,
    Return = 0x1B,
    Strcat = 0x1C,
    Propget = 0x1D,
    Propset = 0x1E,
    ArrayCreate = 0x1F,
    ArrayLength = 0x20,
    ArrayGetElement = 0x21,
    ArraySetElement = 0x22,
    ArrayFindElement = 0x23,
    ArrayRfindElement = 0x24,
    ArrayAddElement = 0x25,
    ArrayInsert = 0x26,
    ArrayRemoveLast = 0x27,
    ArrayRemoveIndex = 0x28,
    ArrayClear = 0x29,
    ArrayRemovelast = 0x2A,
    Invalid = 0x2B,
    IntToFloat = 0x2C,
    FloatToInt = 0x2D,
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
            0x0F => Self::Cmplt,
            0x10 => Self::CmpEq,
            0x11 => Self::CmpLte,
            0x12 => Self::Cmpgt,
            0x13 => Self::Cmpgte,
            0x14 => Self::Cmpneq,
            0x15 => Self::Jump,
            0x16 => Self::Jz,
            0x17 => Self::Jnz,
            0x18 => Self::Callmethod,
            0x19 => Self::Callparent,
            0x1A => Self::Callstatic,
            0x1B => Self::Return,
            0x1C => Self::Strcat,
            0x1D => Self::Propget,
            0x1E => Self::Propset,
            0x1F => Self::ArrayCreate,
            0x20 => Self::ArrayLength,
            0x21 => Self::ArrayGetElement,
            0x22 => Self::ArraySetElement,
            0x23 => Self::ArrayFindElement,
            0x24 => Self::ArrayRfindElement,
            0x25 => Self::ArrayAddElement,
            0x26 => Self::ArrayInsert,
            0x27 => Self::ArrayRemoveLast,
            0x28 => Self::ArrayRemoveIndex,
            0x29 => Self::ArrayClear,
            0x2A => Self::ArrayRemovelast,
            0x2B => Self::Invalid,
            0x2C => Self::IntToFloat,
            0x2D => Self::FloatToInt,
            _ => Self::Nop,
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
            Self::Cmplt => "cmplt",
            Self::CmpEq => "cmpeq",
            Self::CmpLte => "cmplte",
            Self::Cmpgt => "cmpgt",
            Self::Cmpgte => "cmpgte",
            Self::Cmpneq => "cmpneq",
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
            Self::ArrayGetElement => "array_get",
            Self::ArraySetElement => "array_set",
            Self::ArrayFindElement => "array_find",
            Self::ArrayRfindElement => "array_rfind",
            Self::ArrayAddElement => "array_add",
            Self::ArrayInsert => "array_insert",
            Self::ArrayRemoveLast => "array_removelast",
            Self::ArrayRemoveIndex => "array_removeindex",
            Self::ArrayClear => "array_clear",
            Self::ArrayRemovelast => "array_removelast",
            Self::Invalid => "invalid",
            Self::IntToFloat => "int_to_float",
            Self::FloatToInt => "float_to_int",
        }
    }

    /// 此操作码接受的 u16 参数数量（基于 PEX 二进制格式）。
    pub fn arg_count(self) -> usize {
        match self {
            // 0 参数指令
            Self::Nop
            | Self::Iadd
            | Self::Fadd
            | Self::Isub
            | Self::Fsub
            | Self::Imul
            | Self::Fmul
            | Self::Idiv
            | Self::Fdiv
            | Self::Imod
            | Self::Not
            | Self::Ineg
            | Self::Fneg
            | Self::Assign
            | Self::Return
            | Self::Propset
            | Self::ArraySetElement
            | Self::ArrayClear
            | Self::FloatToInt => 0,

            // 1 参数指令
            Self::Cmplt
            | Self::CmpEq
            | Self::CmpLte
            | Self::Cmpgt
            | Self::Cmpgte
            | Self::Cmpneq
            | Self::Strcat
            | Self::Cast
            | Self::Callstatic
            | Self::Propget
            | Self::ArrayCreate
            | Self::ArrayLength
            | Self::ArrayGetElement
            | Self::ArrayAddElement
            | Self::ArrayRemoveLast
            | Self::ArrayRemovelast
            | Self::IntToFloat => 1,

            // 2 参数指令
            Self::Jump
            | Self::Jnz
            | Self::Callmethod
            | Self::Callparent
            | Self::Invalid
            | Self::Jz
            | Self::ArrayFindElement
            | Self::ArrayRfindElement
            | Self::ArrayInsert
            | Self::ArrayRemoveIndex => 2,
        }
    }
}

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

/// 完全反编译的 PEX 脚本
#[derive(Clone, Debug)]
pub struct DecompiledPex {
    pub objects: Vec<PexObject>,
    pub string_table: Vec<PexStringEntry>,
}

// ── 解析器 ────────────────────────────────────────────────────────────────

type StrTab = Vec<PexStringEntry>;

fn lookup(st: &StrTab, idx: u16) -> String {
    st.get(idx as usize)
        .map(|e| e.text.clone())
        .unwrap_or_default()
}

fn parse_var_value(cur: &mut Cursor<&[u8]>, st: &StrTab) -> io::Result<VarValue> {
    if cur.position() >= cur.get_ref().len() as u64 {
        return Ok(VarValue::None);
    }
    let val_type = cur.read_u8()?;
    match val_type {
        0 => Ok(VarValue::None),
        1 => Ok(VarValue::Bool(cur.read_u8()? != 0)),
        2 => Ok(VarValue::Integer(cur.read_u32::<LittleEndian>()?)),
        3 => Ok(VarValue::Float(cur.read_f32::<LittleEndian>()?)),
        4 => Ok(VarValue::Bool(cur.read_u8()? != 0)),
        5 => {
            let idx = cur.read_u16::<LittleEndian>()?;
            Ok(VarValue::String(lookup(st, idx)))
        }
        6 | 7 | 8 => {
            let count = cur.read_u16::<LittleEndian>()? as usize;
            let mut arr = Vec::with_capacity(count);
            for _ in 0..count {
                arr.push(parse_var_value(cur, st)?);
            }
            Ok(VarValue::Array(arr))
        }
        11 => {
            let count = cur.read_u16::<LittleEndian>()? as usize;
            let byte_count = (count + 7) / 8;
            let pos = cur.position();
            cur.set_position(pos + byte_count as u64);
            Ok(VarValue::None)
        }
        12 => {
            let count = cur.read_u16::<LittleEndian>()? as usize;
            let mut arr = Vec::with_capacity(count);
            for _ in 0..count {
                arr.push(VarValue::Integer(cur.read_u32::<LittleEndian>()?));
            }
            Ok(VarValue::Array(arr))
        }
        13 => {
            let count = cur.read_u16::<LittleEndian>()? as usize;
            let mut arr = Vec::with_capacity(count);
            for _ in 0..count {
                arr.push(VarValue::Float(cur.read_f32::<LittleEndian>()?));
            }
            Ok(VarValue::Array(arr))
        }
        14 => {
            let count = cur.read_u16::<LittleEndian>()? as usize;
            let mut arr = Vec::with_capacity(count);
            for _ in 0..count {
                let idx = cur.read_u16::<LittleEndian>()?;
                arr.push(VarValue::String(lookup(st, idx)));
            }
            Ok(VarValue::Array(arr))
        }
        _ => Ok(VarValue::None),
    }
}

fn parse_instruction(cur: &mut Cursor<&[u8]>) -> io::Result<Instruction> {
    let opcode_byte = cur.read_u8()?;
    let opcode = Opcode::from_u8(opcode_byte);
    let arg_count = opcode.arg_count();
    let mut args = Vec::with_capacity(arg_count);
    for _ in 0..arg_count {
        args.push(cur.read_u16::<LittleEndian>()?);
    }
    Ok(Instruction { opcode, args })
}

/// 将 PEX 二进制文件完全反编译为结构化类型。
pub fn decompile_pex(data: &[u8]) -> io::Result<DecompiledPex> {
    let mut cur = Cursor::new(data);

    // Magic
    let magic = cur.read_u32::<LittleEndian>()?;
    if magic != 0xFA57C0DE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid PEX magic: 0x{:08X}", magic),
        ));
    }

    // Header
    let _major = cur.read_u8()?;
    let _minor = cur.read_u8()?;
    let _game_id = cur.read_u16::<LittleEndian>()?;
    let _compile_time = cur.read_u64::<LittleEndian>()?;

    // 字符串表
    let st_count = cur.read_u16::<LittleEndian>()? as usize;
    let mut string_table = Vec::with_capacity(st_count);
    for i in 0..st_count {
        let len = cur.read_u16::<LittleEndian>()? as usize;
        let mut bytes = vec![0u8; len];
        cur.read_exact(&mut bytes)?;
        string_table.push(PexStringEntry {
            index: i as u16,
            text: String::from_utf8_lossy(&bytes).into_owned(),
        });
    }

    // 调试信息（跳过 — 反编译不需要它）
    let _debug_mod_time = cur.read_u64::<LittleEndian>()?;
    let debug_count = cur.read_u16::<LittleEndian>()? as usize;
    for _ in 0..debug_count {
        let len = cur.read_u16::<LittleEndian>()? as usize;
        let pos = cur.position();
        cur.set_position(pos + len as u64);
    }

    // 用户标志（跳过文件头级别）
    let uf_count = cur.read_u16::<LittleEndian>()? as usize;
    for _ in 0..uf_count {
        let _n = cur.read_u16::<LittleEndian>()?;
        let _f = cur.read_u8()?;
    }

    // 对象
    let obj_count = cur.read_u16::<LittleEndian>()? as usize;
    let st = &string_table;
    let mut objects = Vec::with_capacity(obj_count);

    for _ in 0..obj_count {
        let name_idx = cur.read_u16::<LittleEndian>()?;
        let obj_name = lookup(st, name_idx);
        let body_size = cur.read_u32::<LittleEndian>()? as usize;
        let mut body = vec![0u8; body_size];
        cur.read_exact(&mut body)?;
        let obj = parse_object_body_full(&body, &obj_name, st)?;
        objects.push(obj);
    }

    Ok(DecompiledPex {
        objects,
        string_table,
    })
}

fn parse_object_body_full(body: &[u8], obj_name: &str, st: &StrTab) -> io::Result<PexObject> {
    let mut cur = Cursor::new(body);

    let parent_idx = cur.read_u16::<LittleEndian>()?;
    let parent_class = lookup(st, parent_idx);

    let doc_idx = cur.read_u16::<LittleEndian>()?;
    let doc = lookup(st, doc_idx);

    // 用户标志
    let uf_count = cur.read_u16::<LittleEndian>()? as usize;
    let mut user_flags = Vec::with_capacity(uf_count);
    for _ in 0..uf_count {
        let name_idx = cur.read_u16::<LittleEndian>()?;
        let flag = cur.read_u8()?;
        user_flags.push((lookup(st, name_idx), flag));
    }

    let auto_state_idx = cur.read_u16::<LittleEndian>()?;
    let auto_state_name = lookup(st, auto_state_idx);

    // 变量
    let var_count = cur.read_u16::<LittleEndian>()? as usize;
    let mut variables = Vec::with_capacity(var_count);
    for _ in 0..var_count {
        let name = lookup(st, cur.read_u16::<LittleEndian>()?);
        let type_name = lookup(st, cur.read_u16::<LittleEndian>()?);
        let flags = cur.read_u32::<LittleEndian>()?;
        let doc = lookup(st, cur.read_u16::<LittleEndian>()?);
        let user_flags = cur.read_u32::<LittleEndian>()?;
        let default_value = parse_var_value(&mut cur, st)?;
        variables.push(PexVariable {
            name,
            type_name,
            flags,
            doc,
            user_flags,
            default_value,
        });
    }

    // Guards
    let guard_count = cur.read_u16::<LittleEndian>()? as usize;
    let mut guards = Vec::with_capacity(guard_count);
    for _ in 0..guard_count {
        let name = lookup(st, cur.read_u16::<LittleEndian>()?);
        let sc = cur.read_u32::<LittleEndian>()? as usize;
        let mut user_flags = Vec::with_capacity(sc);
        for _ in 0..sc {
            user_flags.push(cur.read_u16::<LittleEndian>()? as u32);
        }
        guards.push(PexGuard { name, user_flags });
    }

    // 属性组
    let pg_count = cur.read_u16::<LittleEndian>()? as usize;
    let mut property_groups = Vec::with_capacity(pg_count);
    for _ in 0..pg_count {
        let name = lookup(st, cur.read_u16::<LittleEndian>()?);
        let doc = lookup(st, cur.read_u16::<LittleEndian>()?);
        let flags = cur.read_u32::<LittleEndian>()?;

        let prop_count = cur.read_u16::<LittleEndian>()? as usize;
        let mut properties = Vec::with_capacity(prop_count);
        for _ in 0..prop_count {
            let prop = parse_property(&mut cur, st)?;
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
    let state_count = cur.read_u16::<LittleEndian>()? as usize;
    let mut states = Vec::with_capacity(state_count);
    for _ in 0..state_count {
        let name = lookup(st, cur.read_u16::<LittleEndian>()?);
        let func_count = cur.read_u16::<LittleEndian>()? as usize;
        let mut functions = Vec::with_capacity(func_count);
        for _ in 0..func_count {
            let func = parse_function(&mut cur, st)?;
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

fn parse_property(cur: &mut Cursor<&[u8]>, st: &StrTab) -> io::Result<PexProperty> {
    let name = lookup(st, cur.read_u16::<LittleEndian>()?);
    let type_name = lookup(st, cur.read_u16::<LittleEndian>()?);
    let doc = lookup(st, cur.read_u16::<LittleEndian>()?);
    let flags = cur.read_u32::<LittleEndian>()?;
    let user_flags = cur.read_u8()?;
    let has_auto = cur.read_u8()?;
    let auto_var = if has_auto != 0 {
        Some(cur.read_u16::<LittleEndian>()?)
    } else {
        None
    };
    let default_value = parse_var_value(cur, st)?;
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

fn parse_function(cur: &mut Cursor<&[u8]>, st: &StrTab) -> io::Result<PexFunction> {
    let name = lookup(st, cur.read_u16::<LittleEndian>()?);
    let return_type = lookup(st, cur.read_u16::<LittleEndian>()?);
    let doc = lookup(st, cur.read_u16::<LittleEndian>()?);
    let flags = cur.read_u8()?;

    let fuf_count = cur.read_u16::<LittleEndian>()? as usize;
    let mut user_flags = Vec::with_capacity(fuf_count);
    for _ in 0..fuf_count {
        let un = lookup(st, cur.read_u16::<LittleEndian>()?);
        let uf = cur.read_u8()?;
        user_flags.push((un, uf));
    }

    let param_count = cur.read_u16::<LittleEndian>()? as usize;
    let mut params = Vec::with_capacity(param_count);
    for _ in 0..param_count {
        let pn = lookup(st, cur.read_u16::<LittleEndian>()?);
        let pt = lookup(st, cur.read_u16::<LittleEndian>()?);
        params.push(PexParam {
            name: pn,
            type_name: pt,
        });
    }

    let local_count = cur.read_u16::<LittleEndian>()? as usize;
    let mut locals = Vec::with_capacity(local_count);
    for _ in 0..local_count {
        let ln = lookup(st, cur.read_u16::<LittleEndian>()?);
        let lt = lookup(st, cur.read_u16::<LittleEndian>()?);
        locals.push(PexLocal {
            name: ln,
            type_name: lt,
        });
    }

    let inst_count = cur.read_u16::<LittleEndian>()? as usize;
    let mut instructions = Vec::with_capacity(inst_count);
    for _ in 0..inst_count {
        instructions.push(parse_instruction(cur)?);
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

/// 从反编译的 PEX 发射类似于 Papyrus 的伪代码。
pub fn emit_pseudocode(pex: &DecompiledPex) -> String {
    let mut out = String::with_capacity(4096);

    for obj in &pex.objects {
        emit_object(&mut out, obj, &pex.string_table);
    }

    out
}

fn emit_object(out: &mut String, obj: &PexObject, st: &StrTab) {
    // 脚本头
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
                let _ = write!(out, " = {}", f);
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

    // 独立属性（不在组中）
    // 组中的属性已在上方发射

    // 状态
    for state in &obj.states {
        if state.name.is_empty() {
            // 默认状态 — 直接发射函数
            for func in &state.functions {
                emit_function(out, func, st);
                out.push('\n');
            }
        } else {
            let _ = writeln!(out, "State {}", state.name);
            for func in &state.functions {
                emit_function(out, func, st);
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
            let _ = write!(out, " = {}", f);
        }
        VarValue::String(s) => {
            let _ = write!(out, " = \"{}\"", s);
        }
        VarValue::Array(_) => {}
    }
    out.push('\n');
    let _ = writeln!(out, "EndProperty");
}

fn emit_function(out: &mut String, func: &PexFunction, st: &StrTab) {
    if !func.doc.is_empty() {
        let _ = writeln!(out, "    ; {}", func.doc);
    }

    // 签名
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

    // 局部变量
    for local in &func.locals {
        let _ = writeln!(out, "        {} {}", local.type_name, local.name);
    }
    if !func.locals.is_empty() {
        out.push('\n');
    }

    // 指令
    for inst in &func.instructions {
        emit_instruction(out, inst, st);
    }

    let _ = writeln!(out, "    EndFunction");
}

fn emit_instruction(out: &mut String, inst: &Instruction, st: &StrTab) {
    match inst.opcode {
        Opcode::Nop => {
            let _ = writeln!(out, "        ; nop");
        }
        Opcode::Return => {
            let _ = writeln!(out, "        return");
        }
        Opcode::Assign => {
            if inst.args.len() >= 2 {
                let dest = arg_name(inst.args[0], st);
                let src = arg_name(inst.args[1], st);
                let _ = writeln!(out, "        {} = {}", dest, src);
            }
        }
        Opcode::Iadd | Opcode::Fadd => {
            if inst.args.len() >= 3 {
                let dest = arg_name(inst.args[0], st);
                let a = arg_name(inst.args[1], st);
                let b = arg_name(inst.args[2], st);
                let op = if inst.opcode == Opcode::Iadd {
                    "+"
                } else {
                    "+"
                };
                let _ = writeln!(out, "        {} = {} {} {}", dest, a, op, b);
            }
        }
        Opcode::Isub | Opcode::Fsub => {
            if inst.args.len() >= 3 {
                let dest = arg_name(inst.args[0], st);
                let a = arg_name(inst.args[1], st);
                let b = arg_name(inst.args[2], st);
                let _ = writeln!(out, "        {} = {} - {}", dest, a, b);
            }
        }
        Opcode::Imul | Opcode::Fmul => {
            if inst.args.len() >= 3 {
                let dest = arg_name(inst.args[0], st);
                let a = arg_name(inst.args[1], st);
                let b = arg_name(inst.args[2], st);
                let _ = writeln!(out, "        {} = {} * {}", dest, a, b);
            }
        }
        Opcode::Idiv | Opcode::Fdiv => {
            if inst.args.len() >= 3 {
                let dest = arg_name(inst.args[0], st);
                let a = arg_name(inst.args[1], st);
                let b = arg_name(inst.args[2], st);
                let _ = writeln!(out, "        {} = {} / {}", dest, a, b);
            }
        }
        Opcode::Imod => {
            if inst.args.len() >= 3 {
                let dest = arg_name(inst.args[0], st);
                let a = arg_name(inst.args[1], st);
                let b = arg_name(inst.args[2], st);
                let _ = writeln!(out, "        {} = {} % {}", dest, a, b);
            }
        }
        Opcode::CmpEq => {
            if inst.args.len() >= 3 {
                let dest = arg_name(inst.args[0], st);
                let a = arg_name(inst.args[1], st);
                let b = arg_name(inst.args[2], st);
                let _ = writeln!(out, "        {} = {} == {}", dest, a, b);
            }
        }
        Opcode::Cmpneq => {
            if inst.args.len() >= 3 {
                let dest = arg_name(inst.args[0], st);
                let a = arg_name(inst.args[1], st);
                let b = arg_name(inst.args[2], st);
                let _ = writeln!(out, "        {} = {} != {}", dest, a, b);
            }
        }
        Opcode::Cmplt => {
            if inst.args.len() >= 3 {
                let dest = arg_name(inst.args[0], st);
                let a = arg_name(inst.args[1], st);
                let b = arg_name(inst.args[2], st);
                let _ = writeln!(out, "        {} = {} < {}", dest, a, b);
            }
        }
        Opcode::CmpLte => {
            if inst.args.len() >= 3 {
                let dest = arg_name(inst.args[0], st);
                let a = arg_name(inst.args[1], st);
                let b = arg_name(inst.args[2], st);
                let _ = writeln!(out, "        {} = {} <= {}", dest, a, b);
            }
        }
        Opcode::Cmpgt => {
            if inst.args.len() >= 3 {
                let dest = arg_name(inst.args[0], st);
                let a = arg_name(inst.args[1], st);
                let b = arg_name(inst.args[2], st);
                let _ = writeln!(out, "        {} = {} > {}", dest, a, b);
            }
        }
        Opcode::Cmpgte => {
            if inst.args.len() >= 3 {
                let dest = arg_name(inst.args[0], st);
                let a = arg_name(inst.args[1], st);
                let b = arg_name(inst.args[2], st);
                let _ = writeln!(out, "        {} = {} >= {}", dest, a, b);
            }
        }
        Opcode::Not => {
            if inst.args.len() >= 2 {
                let dest = arg_name(inst.args[0], st);
                let src = arg_name(inst.args[1], st);
                let _ = writeln!(out, "        {} = !{}", dest, src);
            }
        }
        Opcode::Ineg => {
            if inst.args.len() >= 2 {
                let dest = arg_name(inst.args[0], st);
                let src = arg_name(inst.args[1], st);
                let _ = writeln!(out, "        {} = -{}", dest, src);
            }
        }
        Opcode::Fneg => {
            if inst.args.len() >= 2 {
                let dest = arg_name(inst.args[0], st);
                let src = arg_name(inst.args[1], st);
                let _ = writeln!(out, "        {} = -{}", dest, src);
            }
        }
        Opcode::Cast => {
            if inst.args.len() >= 2 {
                let dest = arg_name(inst.args[0], st);
                let src = arg_name(inst.args[1], st);
                let _ = writeln!(out, "        {} = ({}) {}", dest, "...", src);
            }
        }
        Opcode::Jump => {
            if !inst.args.is_empty() {
                let _ = writeln!(out, "        jump {}", inst.args[0]);
            }
        }
        Opcode::Jz => {
            if inst.args.len() >= 2 {
                let cond = arg_name(inst.args[0], st);
                let _ = writeln!(out, "        if !{}: jump {}", cond, inst.args[1]);
            }
        }
        Opcode::Jnz => {
            if inst.args.len() >= 2 {
                let cond = arg_name(inst.args[0], st);
                let _ = writeln!(out, "        if {}: jump {}", cond, inst.args[1]);
            }
        }
        Opcode::Callmethod => {
            if inst.args.len() >= 2 {
                let func_name = arg_name(inst.args[0], st);
                let obj = arg_name(inst.args[1], st);
                let _ = writeln!(out, "        {}.{}(...)", obj, func_name);
            }
        }
        Opcode::Callparent => {
            if !inst.args.is_empty() {
                let func_name = arg_name(inst.args[0], st);
                let _ = writeln!(out, "        parent.{}(...)", func_name);
            }
        }
        Opcode::Callstatic => {
            if inst.args.len() >= 2 {
                let func_name = arg_name(inst.args[0], st);
                let script = arg_name(inst.args[1], st);
                let _ = writeln!(out, "        {}.{}(...)", script, func_name);
            }
        }
        Opcode::Strcat => {
            if inst.args.len() >= 3 {
                let dest = arg_name(inst.args[0], st);
                let a = arg_name(inst.args[1], st);
                let b = arg_name(inst.args[2], st);
                let _ = writeln!(out, "        {} = {} + {}", dest, a, b);
            }
        }
        Opcode::Propget => {
            if inst.args.len() >= 2 {
                let prop = arg_name(inst.args[0], st);
                let obj = arg_name(inst.args[1], st);
                let _ = writeln!(out, "        {} = {}.{}", obj, obj, prop);
            }
        }
        Opcode::Propset => {
            if inst.args.len() >= 2 {
                let prop = arg_name(inst.args[0], st);
                let val = arg_name(inst.args[1], st);
                let _ = writeln!(out, "        {} = {}", prop, val);
            }
        }
        Opcode::ArrayCreate => {
            if inst.args.len() >= 2 {
                let arr = arg_name(inst.args[0], st);
                let size = arg_name(inst.args[1], st);
                let _ = writeln!(out, "        {} = new [{}]", arr, size);
            }
        }
        Opcode::ArrayLength => {
            if inst.args.len() >= 2 {
                let dest = arg_name(inst.args[0], st);
                let arr = arg_name(inst.args[1], st);
                let _ = writeln!(out, "        {} = {}.length", dest, arr);
            }
        }
        Opcode::ArrayGetElement => {
            if inst.args.len() >= 3 {
                let dest = arg_name(inst.args[0], st);
                let arr = arg_name(inst.args[1], st);
                let idx = arg_name(inst.args[2], st);
                let _ = writeln!(out, "        {} = {}[{}]", dest, arr, idx);
            }
        }
        Opcode::ArraySetElement => {
            if inst.args.len() >= 3 {
                let arr = arg_name(inst.args[0], st);
                let idx = arg_name(inst.args[1], st);
                let val = arg_name(inst.args[2], st);
                let _ = writeln!(out, "        {}[{}] = {}", arr, idx, val);
            }
        }
        Opcode::ArrayFindElement => {
            if inst.args.len() >= 4 {
                let dest = arg_name(inst.args[0], st);
                let arr = arg_name(inst.args[1], st);
                let val = arg_name(inst.args[2], st);
                let start = arg_name(inst.args[3], st);
                let _ = writeln!(out, "        {} = {}.find({}, {})", dest, arr, val, start);
            }
        }
        Opcode::ArrayRfindElement => {
            if inst.args.len() >= 4 {
                let dest = arg_name(inst.args[0], st);
                let arr = arg_name(inst.args[1], st);
                let val = arg_name(inst.args[2], st);
                let start = arg_name(inst.args[3], st);
                let _ = writeln!(out, "        {} = {}.rfind({}, {})", dest, arr, val, start);
            }
        }
        Opcode::ArrayAddElement => {
            if inst.args.len() >= 2 {
                let arr = arg_name(inst.args[0], st);
                let val = arg_name(inst.args[1], st);
                let _ = writeln!(out, "        {}.add({})", arr, val);
            }
        }
        Opcode::ArrayInsert => {
            if inst.args.len() >= 3 {
                let arr = arg_name(inst.args[0], st);
                let idx = arg_name(inst.args[1], st);
                let val = arg_name(inst.args[2], st);
                let _ = writeln!(out, "        {}.insert({}, {})", arr, idx, val);
            }
        }
        Opcode::ArrayRemoveLast => {
            if !inst.args.is_empty() {
                let arr = arg_name(inst.args[0], st);
                let _ = writeln!(out, "        {}.removelast()", arr);
            }
        }
        Opcode::ArrayRemoveIndex => {
            if inst.args.len() >= 3 {
                let arr = arg_name(inst.args[0], st);
                let idx = arg_name(inst.args[1], st);
                let count = arg_name(inst.args[2], st);
                let _ = writeln!(out, "        {}.remove({}, {})", arr, idx, count);
            }
        }
        Opcode::ArrayClear => {
            if !inst.args.is_empty() {
                let arr = arg_name(inst.args[0], st);
                let _ = writeln!(out, "        {}.clear()", arr);
            }
        }
        Opcode::IntToFloat => {
            if inst.args.len() >= 2 {
                let dest = arg_name(inst.args[0], st);
                let src = arg_name(inst.args[1], st);
                let _ = writeln!(out, "        {} = {} as Float", dest, src);
            }
        }
        Opcode::FloatToInt => {
            if inst.args.len() >= 2 {
                let dest = arg_name(inst.args[0], st);
                let src = arg_name(inst.args[1], st);
                let _ = writeln!(out, "        {} = {} as Int", dest, src);
            }
        }
        Opcode::ArrayRemovelast => {
            if !inst.args.is_empty() {
                let arr = arg_name(inst.args[0], st);
                let _ = writeln!(out, "        {}.removelast()", arr);
            }
        }
        Opcode::Invalid => {
            let _ = writeln!(out, "        ; invalid({:?})", inst.args);
        }
    }
}

fn arg_name(arg: u16, st: &StrTab) -> String {
    // PEX 中的参数通常是编码为字符串表引用的变量/临时变量索引
    // 我们查找字符串表以提高可读性
    lookup(st, arg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opcode_mnemonic() {
        assert_eq!(Opcode::Nop.mnemonic(), "nop");
        assert_eq!(Opcode::Return.mnemonic(), "return");
        assert_eq!(Opcode::Callmethod.mnemonic(), "callmethod");
    }

    #[test]
    fn test_opcode_arg_count() {
        assert_eq!(Opcode::Nop.arg_count(), 0);
        assert_eq!(Opcode::Jump.arg_count(), 2);
        assert_eq!(Opcode::Invalid.arg_count(), 2);
        assert_eq!(Opcode::Cast.arg_count(), 1);
        assert_eq!(Opcode::Return.arg_count(), 0);
        assert_eq!(Opcode::Callmethod.arg_count(), 2);
        assert_eq!(Opcode::Callstatic.arg_count(), 1);
        assert_eq!(Opcode::Jz.arg_count(), 2);
        assert_eq!(Opcode::Jnz.arg_count(), 2);
    }

    #[test]
    fn test_decompile_minimal_pex() {
        // 构造一个包含单个对象和单个函数的最小 PEX
        // String table:
        // 0: ""
        // 1: "TestScript"
        // 2: ""
        // 3: ""
        // 4: ""
        // 5: "Int"
        // 6: "count"
        // 7: ""
        // 8: "GetCount"
        // 9: "Int"

        let strings: Vec<&str> = vec![
            "",
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
        // idx 0:"", 1:"", 2:"TestScript", 3:"", 4:"", 5:"", 6:"Int", 7:"count", 8:"", 9:"GetCount", 10:"Int"

        let mut data = Vec::new();
        // Magic
        data.extend_from_slice(&0xFA57C0DEu32.to_le_bytes());
        // Header
        data.push(3);
        data.push(10);
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&0u64.to_le_bytes());
        // String table
        data.extend_from_slice(&(strings.len() as u16).to_le_bytes());
        for s in &strings {
            let b = s.as_bytes();
            data.extend_from_slice(&(b.len() as u16).to_le_bytes());
            data.extend_from_slice(b);
        }
        // Debug info (empty)
        data.extend_from_slice(&0u64.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        // User flags (empty)
        data.extend_from_slice(&0u16.to_le_bytes());
        // Objects: 1
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&2u16.to_le_bytes()); // name=idx2 "TestScript"

        // 构建对象体
        let mut body = Vec::new();
        // parent class = "" (idx 0)
        body.extend_from_slice(&0u16.to_le_bytes());
        // doc = "" (idx 1)
        body.extend_from_slice(&1u16.to_le_bytes());
        // user flags = 0
        body.extend_from_slice(&0u16.to_le_bytes());
        // auto state = "" (idx 3)
        body.extend_from_slice(&3u16.to_le_bytes());
        // variables = 0
        body.extend_from_slice(&0u16.to_le_bytes());
        // guards = 0
        body.extend_from_slice(&0u16.to_le_bytes());
        // property groups = 0
        body.extend_from_slice(&0u16.to_le_bytes());
        // states = 1
        body.extend_from_slice(&1u16.to_le_bytes());
        // state name = "" (idx 4)
        body.extend_from_slice(&4u16.to_le_bytes());
        // functions = 1
        body.extend_from_slice(&1u16.to_le_bytes());
        // func name = "GetCount" (idx 9)
        body.extend_from_slice(&9u16.to_le_bytes());
        // return type = "Int" (idx 10)
        body.extend_from_slice(&10u16.to_le_bytes());
        // doc = "" (idx 5)
        body.extend_from_slice(&5u16.to_le_bytes());
        // flags = 0
        body.push(0u8);
        // user flags = 0
        body.extend_from_slice(&0u16.to_le_bytes());
        // params = 0
        body.extend_from_slice(&0u16.to_le_bytes());
        // locals = 1 (Int count)
        body.extend_from_slice(&1u16.to_le_bytes());
        body.extend_from_slice(&7u16.to_le_bytes()); // name=idx7 "count"
        body.extend_from_slice(&6u16.to_le_bytes()); // type=idx6 "Int"
                                                     // instructions = 3
        body.extend_from_slice(&3u16.to_le_bytes());
        // inst 0: Jump (0x15), 2 个参数
        body.push(0x15u8);
        body.extend_from_slice(&1u16.to_le_bytes());
        body.extend_from_slice(&2u16.to_le_bytes());
        // inst 1: Cast (0x0E), 1 个参数
        body.push(0x0Eu8);
        body.extend_from_slice(&5u16.to_le_bytes());
        // inst 2: Return (0x1B), 0 个参数
        body.push(0x1Bu8);

        // 写入对象体大小和数据
        data.extend_from_slice(&(body.len() as u32).to_le_bytes());
        data.extend_from_slice(&body);

        // 反编译
        let result = decompile_pex(&data);
        assert!(result.is_ok(), "Decompile failed: {:?}", result.err());
        let decompiled = result.unwrap();

        assert_eq!(decompiled.objects.len(), 1);
        assert_eq!(decompiled.objects[0].name, "TestScript");
        assert_eq!(decompiled.objects[0].states.len(), 1);
        assert_eq!(decompiled.objects[0].states[0].functions.len(), 1);
        let func = &decompiled.objects[0].states[0].functions[0];
        assert_eq!(func.name, "GetCount");
        assert_eq!(func.return_type, "Int");
        assert_eq!(func.locals.len(), 1);
        assert_eq!(func.locals[0].name, "count");
        assert_eq!(func.instructions.len(), 3);
        assert_eq!(func.instructions[0].opcode, Opcode::Jump);
        assert_eq!(func.instructions[1].opcode, Opcode::Cast);
        assert_eq!(func.instructions[2].opcode, Opcode::Return);

        // 发射伪代码并验证关键部分
        let pseudo = emit_pseudocode(&decompiled);
        println!("=== PSEUDOCODE ===\n{}", pseudo);
        assert!(pseudo.contains("ScriptName TestScript"));
        assert!(pseudo.contains("Int Function GetCount()"));
        assert!(pseudo.contains("Int count"));
        assert!(pseudo.contains("return"));
        assert!(pseudo.contains("EndFunction"));
    }

    #[test]
    fn test_decompile_reject_invalid_magic() {
        let data = vec![0u8; 16];
        let result = decompile_pex(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_decompile_empty_object_list() {
        let mut data = Vec::new();
        data.extend_from_slice(&0xFA57C0DEu32.to_le_bytes());
        data.push(3);
        data.push(10);
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&0u64.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes()); // 空字符串表
        data.extend_from_slice(&0u64.to_le_bytes()); // 调试修改时间
        data.extend_from_slice(&0u16.to_le_bytes()); // 调试计数
        data.extend_from_slice(&0u16.to_le_bytes()); // 用户标志
        data.extend_from_slice(&0u16.to_le_bytes()); // 0 个对象

        let result = decompile_pex(&data);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().objects.len(), 0);
    }
}
