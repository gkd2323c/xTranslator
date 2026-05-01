//! PEX decompiler — parses PEX binary into structured types and emits Papyrus-like pseudocode.

use byteorder::{LittleEndian, ReadBytesExt};
use std::fmt::Write;
use std::io::{self, Cursor, Read};

use super::types::PexStringEntry;

// ── Structured types ──────────────────────────────────────────────────────

/// Decoded PEX instruction
#[derive(Clone, Debug)]
pub struct Instruction {
    pub opcode: Opcode,
    pub args: Vec<u16>,
}

/// All known Papyrus opcodes
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

    /// Number of u16 arguments this opcode takes
    pub fn arg_count(self) -> usize {
        match self {
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
            | Self::Cmplt
            | Self::CmpEq
            | Self::CmpLte
            | Self::Cmpgt
            | Self::Cmpgte
            | Self::Cmpneq
            | Self::Return
            | Self::Strcat
            | Self::Propset
            | Self::ArraySetElement
            | Self::ArrayFindElement
            | Self::ArrayRfindElement
            | Self::ArrayInsert
            | Self::ArrayRemoveIndex
            | Self::ArrayClear
            | Self::ArrayRemovelast
            | Self::IntToFloat
            | Self::FloatToInt => 0,

            Self::Cast
            | Self::Jump
            | Self::Jz
            | Self::Jnz
            | Self::Callmethod
            | Self::Callparent
            | Self::Callstatic
            | Self::Propget
            | Self::ArrayCreate
            | Self::ArrayLength
            | Self::ArrayGetElement
            | Self::ArrayAddElement
            | Self::ArrayRemoveLast => 1,

            Self::Invalid => 2,
        }
    }
}

/// Variable definition
#[derive(Clone, Debug)]
pub struct PexVariable {
    pub name: String,
    pub type_name: String,
    pub flags: u32,
    pub doc: String,
    pub user_flags: u32,
    pub default_value: VarValue,
}

/// Variable default value
#[derive(Clone, Debug)]
pub enum VarValue {
    None,
    Bool(bool),
    Integer(u32),
    Float(f32),
    String(String),
    Array(Vec<VarValue>),
}

/// Property definition
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

/// Property group
#[derive(Clone, Debug)]
pub struct PexPropertyGroup {
    pub name: String,
    pub doc: String,
    pub flags: u32,
    pub properties: Vec<PexProperty>,
}

/// Function parameter
#[derive(Clone, Debug)]
pub struct PexParam {
    pub name: String,
    pub type_name: String,
}

/// Local variable in a function
#[derive(Clone, Debug)]
pub struct PexLocal {
    pub name: String,
    pub type_name: String,
}

/// Function definition
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

/// State definition
#[derive(Clone, Debug)]
pub struct PexState {
    pub name: String,
    pub functions: Vec<PexFunction>,
}

/// Guard definition
#[derive(Clone, Debug)]
pub struct PexGuard {
    pub name: String,
    pub user_flags: Vec<u32>,
}

/// Fully parsed object
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

/// Fully decompiled PEX script
#[derive(Clone, Debug)]
pub struct DecompiledPex {
    pub objects: Vec<PexObject>,
    pub string_table: Vec<PexStringEntry>,
}

// ── Parser ────────────────────────────────────────────────────────────────

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

/// Fully decompile a PEX binary into structured types.
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

    // String table
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

    // Debug info (skip — we don't need it for decompilation)
    let _debug_mod_time = cur.read_u64::<LittleEndian>()?;
    let debug_count = cur.read_u16::<LittleEndian>()? as usize;
    for _ in 0..debug_count {
        let len = cur.read_u16::<LittleEndian>()? as usize;
        let pos = cur.position();
        cur.set_position(pos + len as u64);
    }

    // User flags (skip header-level)
    let uf_count = cur.read_u16::<LittleEndian>()? as usize;
    for _ in 0..uf_count {
        let _n = cur.read_u16::<LittleEndian>()?;
        let _f = cur.read_u8()?;
    }

    // Objects
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

    // User flags
    let uf_count = cur.read_u16::<LittleEndian>()? as usize;
    let mut user_flags = Vec::with_capacity(uf_count);
    for _ in 0..uf_count {
        let name_idx = cur.read_u16::<LittleEndian>()?;
        let flag = cur.read_u8()?;
        user_flags.push((lookup(st, name_idx), flag));
    }

    let auto_state_idx = cur.read_u16::<LittleEndian>()?;
    let auto_state_name = lookup(st, auto_state_idx);

    // Variables
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

    // Property groups
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

    // States
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

// ── Pseudocode Emitter ────────────────────────────────────────────────────

/// Emit Papyrus-like pseudocode from a decompiled PEX.
pub fn emit_pseudocode(pex: &DecompiledPex) -> String {
    let mut out = String::with_capacity(4096);

    for obj in &pex.objects {
        emit_object(&mut out, obj, &pex.string_table);
    }

    out
}

fn emit_object(out: &mut String, obj: &PexObject, st: &StrTab) {
    // Script header
    let _ = write!(out, "ScriptName {}", obj.name);
    if !obj.parent_class.is_empty() {
        let _ = write!(out, " Extends {}", obj.parent_class);
    }
    out.push('\n');

    if !obj.doc.is_empty() {
        let _ = writeln!(out, "; {}", obj.doc);
        out.push('\n');
    }

    // Variables
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

    // Property groups
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

    // Standalone properties (not in groups)
    // Properties in groups are already emitted above

    // States
    for state in &obj.states {
        if state.name.is_empty() {
            // Default state — emit functions directly
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

    // Signature
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

    // Local variables
    for local in &func.locals {
        let _ = writeln!(out, "        {} {}", local.type_name, local.name);
    }
    if !func.locals.is_empty() {
        out.push('\n');
    }

    // Instructions
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
                let _ = writeln!(
                    out,
                    "        {} = {}.find({}, {})",
                    dest, arr, val, start
                );
            }
        }
        Opcode::ArrayRfindElement => {
            if inst.args.len() >= 4 {
                let dest = arg_name(inst.args[0], st);
                let arr = arg_name(inst.args[1], st);
                let val = arg_name(inst.args[2], st);
                let start = arg_name(inst.args[3], st);
                let _ = writeln!(
                    out,
                    "        {} = {}.rfind({}, {})",
                    dest, arr, val, start
                );
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
            let _ = writeln!(
                out,
                "        ; invalid({:?})",
                inst.args
            );
        }
    }
}

fn arg_name(arg: u16, st: &StrTab) -> String {
    // Arguments in PEX are typically variable/temp indices encoded as string table refs
    // We look up the string table for readability
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
        assert_eq!(Opcode::Jump.arg_count(), 1);
        assert_eq!(Opcode::Invalid.arg_count(), 2);
    }
}
