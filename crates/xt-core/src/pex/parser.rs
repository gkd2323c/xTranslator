//! PEX binary parser — extracts translatable strings from compiled Papyrus scripts.

use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Cursor, Read, Result};

use super::types::*;

const PEX_MAGIC: u32 = 0xFA57C0DE;

pub fn parse_pex<R: Read>(reader: &mut R) -> Result<PexScript> {
    let magic = reader.read_u32::<LittleEndian>()?;
    if magic != PEX_MAGIC {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Invalid PEX magic: 0x{:08X}", magic),
        ));
    }

    let major_version = reader.read_u8()?;
    let minor_version = reader.read_u8()?;
    let game_id = reader.read_u16::<LittleEndian>()?;
    let compile_time = reader.read_u64::<LittleEndian>()?;

    let header = PexHeader {
        major_version,
        minor_version,
        game_id,
        compile_time,
    };

    // String table
    let st_count = reader.read_u16::<LittleEndian>()? as usize;
    let mut string_table = Vec::with_capacity(st_count);
    for i in 0..st_count {
        let len = reader.read_u16::<LittleEndian>()? as usize;
        let mut bytes = vec![0u8; len];
        reader.read_exact(&mut bytes)?;
        string_table.push(PexStringEntry {
            index: i as u16,
            text: String::from_utf8_lossy(&bytes).into_owned(),
        });
    }

    // Debug info — capture raw bytes for preservation during recompile
    let debug_mod_time = reader.read_u64::<LittleEndian>()?;
    let debug_count = reader.read_u16::<LittleEndian>()? as usize;
    let mut debug_info_raw = Vec::with_capacity(8 + 2 + debug_count * 512);
    debug_info_raw.extend_from_slice(&debug_mod_time.to_le_bytes());
    debug_info_raw.extend_from_slice(&(debug_count as u16).to_le_bytes());
    for _ in 0..debug_count {
        let len = reader.read_u16::<LittleEndian>()? as usize;
        let mut buf = vec![0u8; len];
        reader.read_exact(&mut buf)?;
        debug_info_raw.extend_from_slice(&(len as u16).to_le_bytes());
        debug_info_raw.extend_from_slice(&buf);
    }

    // User flags — capture raw bytes for preservation during recompile
    let uf_count = reader.read_u16::<LittleEndian>()? as usize;
    let mut user_flags_raw = Vec::with_capacity(2 + uf_count * 3);
    user_flags_raw.extend_from_slice(&(uf_count as u16).to_le_bytes());
    for _ in 0..uf_count {
        let n = reader.read_u16::<LittleEndian>()?;
        let f = reader.read_u8()?;
        user_flags_raw.extend_from_slice(&n.to_le_bytes());
        user_flags_raw.push(f);
    }

    // Objects — parse for strings AND preserve raw body bytes
    let object_count = reader.read_u16::<LittleEndian>()? as usize;
    let mut translatable = Vec::new();
    let mut object_bodies_raw = Vec::with_capacity(object_count);
    let st = &string_table;

    for _ in 0..object_count {
        let name_idx = reader.read_u16::<LittleEndian>()?;
        let obj_name = if (name_idx as usize) < st.len() {
            st[name_idx as usize].text.clone()
        } else {
            String::new()
        };

        let body_size = reader.read_u32::<LittleEndian>()? as usize;
        let mut body_bytes = vec![0u8; body_size];
        reader.read_exact(&mut body_bytes)?;

        // Preserve raw body bytes for recompile
        object_bodies_raw.push(body_bytes.clone());

        // Parse body for translatable strings
        let mut cur = Cursor::new(&body_bytes[..]);
        parse_object_body(&mut cur, &obj_name, st, &mut translatable)?;
    }

    Ok(PexScript {
        header,
        string_table,
        translatable,
        debug_info_raw,
        user_flags_raw,
        object_bodies_raw,
    })
}

type StrTab = Vec<PexStringEntry>;

fn lookup_str(st: &StrTab, idx: u16) -> String {
    st.get(idx as usize)
        .map(|e| e.text.clone())
        .unwrap_or_default()
}

fn parse_object_body(
    cur: &mut Cursor<&[u8]>,
    obj_name: &str,
    st: &StrTab,
    out: &mut Vec<PexTranslatableString>,
) -> Result<()> {
    let _parent = cur.read_u16::<LittleEndian>()?;

    let doc_idx = cur.read_u16::<LittleEndian>()?;
    let doc_str = lookup_str(st, doc_idx);
    if !doc_str.is_empty() {
        out.push(PexTranslatableString {
            translation: String::new(),
            object_name: obj_name.to_string(),
            state_name: String::new(),
            function_name: String::new(),
            string_type: "DebugString".to_string(),
            source_text: doc_str,
        });
    }

    let uf_count = cur.read_u16::<LittleEndian>()? as usize;
    for _ in 0..uf_count {
        let _n = cur.read_u16::<LittleEndian>()?;
        let _f = cur.read_u8()?;
    }

    let auto_state_idx = cur.read_u16::<LittleEndian>()?;
    let auto_state = lookup_str(st, auto_state_idx);

    // Variables
    let var_count = cur.read_u16::<LittleEndian>()? as usize;
    for _ in 0..var_count {
        let vn_idx = cur.read_u16::<LittleEndian>()?;
        let vn = lookup_str(st, vn_idx);
        let _vt_idx = cur.read_u16::<LittleEndian>()?;
        let _vf = cur.read_u32::<LittleEndian>()?;
        let vd_idx = cur.read_u16::<LittleEndian>()?;
        let vd = lookup_str(st, vd_idx);
        if !vd.is_empty() {
            out.push(PexTranslatableString {
                translation: String::new(),
                object_name: obj_name.to_string(),
                state_name: auto_state.clone(),
                function_name: vn.clone(),
                string_type: "DebugString".to_string(),
                source_text: vd,
            });
        } else if !vn.is_empty() && !vd.is_empty() {
            out.push(PexTranslatableString {
                translation: String::new(),
                object_name: obj_name.to_string(),
                state_name: auto_state.clone(),
                function_name: vn,
                string_type: "PropertyName".to_string(),
                source_text: vd,
            });
        }
        let _vuf = cur.read_u32::<LittleEndian>()?;
        skip_var_value(cur, st)?;
    }

    // Guards
    let guard_count = cur.read_u16::<LittleEndian>()? as usize;
    for _ in 0..guard_count {
        let _gn = cur.read_u16::<LittleEndian>()?;
        let sc = cur.read_u32::<LittleEndian>()? as usize;
        for _ in 0..sc {
            let _s = cur.read_u16::<LittleEndian>()?;
        }
    }

    // Property groups
    let pg_count = cur.read_u16::<LittleEndian>()? as usize;
    for _ in 0..pg_count {
        let _pgn = cur.read_u16::<LittleEndian>()?;
        let pgd_idx = cur.read_u16::<LittleEndian>()?;
        let pgd = lookup_str(st, pgd_idx);
        if !pgd.is_empty() {
            out.push(PexTranslatableString {
                translation: String::new(),
                object_name: obj_name.to_string(),
                state_name: String::new(),
                function_name: String::new(),
                string_type: "DebugString".to_string(),
                source_text: pgd,
            });
        }
        let _pgf = cur.read_u32::<LittleEndian>()?;

        let prop_count = cur.read_u16::<LittleEndian>()? as usize;
        for _ in 0..prop_count {
            let pn_idx = cur.read_u16::<LittleEndian>()?;
            let pn = lookup_str(st, pn_idx);
            let _pt_idx = cur.read_u16::<LittleEndian>()?;
            let pd_idx = cur.read_u16::<LittleEndian>()?;
            let pd = lookup_str(st, pd_idx);
            if !pd.is_empty() {
                out.push(PexTranslatableString {
                    translation: String::new(),
                    object_name: obj_name.to_string(),
                    state_name: String::new(),
                    function_name: pn.clone(),
                    string_type: "DebugString".to_string(),
                    source_text: pd,
                });
            } else if !pn.is_empty() && (pn.contains(' ') || pn.chars().count() > 3) {
                out.push(PexTranslatableString {
                    translation: String::new(),
                    object_name: obj_name.to_string(),
                    state_name: String::new(),
                    function_name: String::new(),
                    string_type: "PropertyName".to_string(),
                    source_text: pn,
                });
            }
            let _pf1 = cur.read_u32::<LittleEndian>()?;
            let _pf2 = cur.read_u8()?;
            let has_auto = cur.read_u8()?;
            if has_auto != 0 {
                let _av = cur.read_u16::<LittleEndian>()?;
            }
            skip_var_value(cur, st)?;
        }
    }

    // States
    let state_count = cur.read_u16::<LittleEndian>()? as usize;
    for _ in 0..state_count {
        let sn_idx = cur.read_u16::<LittleEndian>()?;
        let sn = lookup_str(st, sn_idx);

        let func_count = cur.read_u16::<LittleEndian>()? as usize;
        for _ in 0..func_count {
            let fn_idx = cur.read_u16::<LittleEndian>()?;
            let fn_name = lookup_str(st, fn_idx);
            let _fr_idx = cur.read_u16::<LittleEndian>()?;
            let fd_idx = cur.read_u16::<LittleEndian>()?;
            let fd = lookup_str(st, fd_idx);
            if !fd.is_empty() {
                out.push(PexTranslatableString {
                    translation: String::new(),
                    object_name: obj_name.to_string(),
                    state_name: sn.clone(),
                    function_name: fn_name,
                    string_type: "DebugString".to_string(),
                    source_text: fd,
                });
            }
            let _ff = cur.read_u8()?;
            let fuf_count = cur.read_u16::<LittleEndian>()? as usize;
            for _ in 0..fuf_count {
                let _un = cur.read_u16::<LittleEndian>()?;
                let _uf = cur.read_u8()?;
            }

            let param_count = cur.read_u16::<LittleEndian>()? as usize;
            for _ in 0..param_count {
                let _pn = cur.read_u16::<LittleEndian>()?;
                let _pt = cur.read_u16::<LittleEndian>()?;
            }

            let local_count = cur.read_u16::<LittleEndian>()? as usize;
            for _ in 0..local_count {
                let _ln = cur.read_u16::<LittleEndian>()?;
                let _lt = cur.read_u16::<LittleEndian>()?;
            }

            let inst_count = cur.read_u16::<LittleEndian>()? as usize;
            for _ in 0..inst_count {
                skip_instruction(cur)?;
            }
        }
    }

    Ok(())
}

fn skip_var_value(cur: &mut Cursor<&[u8]>, st: &StrTab) -> Result<()> {
    if cur.position() >= cur.get_ref().len() as u64 {
        return Ok(());
    }
    let val_type = cur.read_u8()?;
    match val_type {
        0 => {}
        1 => {
            let _ = cur.read_u8()?;
        }
        2 => {
            let _ = cur.read_u32::<LittleEndian>()?;
        }
        3 => {
            let _ = cur.read_f32::<LittleEndian>()?;
        }
        4 => {
            let _ = cur.read_u8()?;
        }
        5 => {
            let _idx = cur.read_u16::<LittleEndian>()?;
        }
        6 | 7 | 8 => {
            let count = cur.read_u16::<LittleEndian>()? as usize;
            for _ in 0..count {
                skip_var_value(cur, st)?;
            }
        }
        11 => {
            let count = cur.read_u16::<LittleEndian>()? as usize;
            let byte_count = (count + 7) / 8;
            let pos = cur.position();
            cur.set_position(pos + byte_count as u64);
        }
        12 => {
            let count = cur.read_u16::<LittleEndian>()? as usize;
            for _ in 0..count {
                let _ = cur.read_u32::<LittleEndian>()?;
            }
        }
        13 => {
            let count = cur.read_u16::<LittleEndian>()? as usize;
            for _ in 0..count {
                let _ = cur.read_f32::<LittleEndian>()?;
            }
        }
        14 => {
            let count = cur.read_u16::<LittleEndian>()? as usize;
            for _ in 0..count {
                let _idx = cur.read_u16::<LittleEndian>()?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn skip_instruction(cur: &mut Cursor<&[u8]>) -> Result<()> {
    let opcode = cur.read_u8()?;
    let args: usize = match opcode {
        0x00..=0x0D
        | 0x0F
        | 0x12
        | 0x16
        | 0x1B
        | 0x1E
        | 0x22
        | 0x23
        | 0x24
        | 0x26
        | 0x28..=0x2A
        | 0x2C => 0,
        0x0E | 0x10 | 0x11 | 0x13 | 0x14 | 0x1A | 0x1C | 0x1D | 0x1F | 0x20 | 0x21 | 0x25
        | 0x27 => 1,
        0x15 | 0x17 | 0x18 | 0x19 | 0x2B | 0x2D => 2,
        _ => 1,
    };
    for _ in 0..args {
        let _arg = cur.read_u16::<LittleEndian>()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn build_minimal_pex() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&0xFA57C0DEu32.to_le_bytes());
        data.push(3);
        data.push(10);
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&0u64.to_le_bytes());

        data.extend_from_slice(&3u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        let s1 = b"TestObject";
        data.extend_from_slice(&(s1.len() as u16).to_le_bytes());
        data.extend_from_slice(s1);
        let s2 = b"A doc string for translation";
        data.extend_from_slice(&(s2.len() as u16).to_le_bytes());
        data.extend_from_slice(s2);

        data.extend_from_slice(&0u64.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());

        let mut body = Vec::new();
        body.extend_from_slice(&0u16.to_le_bytes());
        body.extend_from_slice(&2u16.to_le_bytes());
        body.extend_from_slice(&0u16.to_le_bytes());
        body.extend_from_slice(&0u16.to_le_bytes());
        body.extend_from_slice(&0u16.to_le_bytes());
        body.extend_from_slice(&0u16.to_le_bytes());
        body.extend_from_slice(&0u16.to_le_bytes());
        body.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&(body.len() as u32).to_le_bytes());
        data.extend_from_slice(&body);

        data
    }

    #[test]
    fn test_parse_minimal_pex() {
        let data = build_minimal_pex();
        let mut cursor = Cursor::new(&data[..]);
        let script = parse_pex(&mut cursor).unwrap();
        assert_eq!(script.header.major_version, 3);
        assert_eq!(script.string_table.len(), 3);
        assert_eq!(script.translatable.len(), 1);
        assert_eq!(script.translatable[0].object_name, "TestObject");
        assert_eq!(
            script.translatable[0].source_text,
            "A doc string for translation"
        );
    }

    #[test]
    fn test_reject_invalid_magic() {
        let data = vec![0u8; 8];
        let mut cursor = Cursor::new(&data[..]);
        assert!(parse_pex(&mut cursor).is_err());
    }

    #[test]
    fn test_empty_string_table() {
        let mut data = Vec::new();
        data.extend_from_slice(&0xFA57C0DEu32.to_le_bytes());
        data.push(3);
        data.push(10);
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&0u64.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u64.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());

        let mut cursor = Cursor::new(&data[..]);
        let script = parse_pex(&mut cursor).unwrap();
        assert_eq!(script.string_table.len(), 0);
        assert_eq!(script.translatable.len(), 0);
    }
}
