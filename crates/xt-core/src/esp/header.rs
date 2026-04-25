use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Read, Result};

/// 通用头部 (8 bytes)
#[derive(Clone, Debug)]
pub struct GenericHeader {
    pub name: [u8; 4],
    pub dsize: u32,
}

impl GenericHeader {
    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        let mut name = [0u8; 4];
        reader.read_exact(&mut name)?;
        let dsize = reader.read_u32::<LittleEndian>()?;
        Ok(Self { name, dsize })
    }

    pub fn is_grup(&self) -> bool {
        &self.name == b"GRUP"
    }

    pub fn is_tes4(&self) -> bool {
        &self.name == b"TES4"
    }
}

/// 记录头部数据 (16 bytes, TES5)
#[derive(Clone, Debug)]
pub struct RecordHeaderData {
    pub flags: u32,
    pub form_id: u32,
    pub version: u32,
    pub f_version: u16,
    pub v_info: u16,
}

impl RecordHeaderData {
    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(Self {
            flags: reader.read_u32::<LittleEndian>()?,
            form_id: reader.read_u32::<LittleEndian>()?,
            version: reader.read_u32::<LittleEndian>()?,
            f_version: reader.read_u16::<LittleEndian>()?,
            v_info: reader.read_u16::<LittleEndian>()?,
        })
    }

    pub fn is_compressed(&self) -> bool {
        (self.flags & 0x00040000) != 0
    }
}

/// 组头部 (16 bytes, TES5)
#[derive(Clone, Debug)]
pub struct GrupHeader {
    pub s_ident: [u8; 4],
    pub s_type: u32,
    pub s_tstamp: u16,
    pub param1: u16,
    pub param2: u16,
    pub param3: u16,
}

impl GrupHeader {
    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        let mut s_ident = [0u8; 4];
        reader.read_exact(&mut s_ident)?;
        Ok(Self {
            s_ident,
            s_type: reader.read_u32::<LittleEndian>()?,
            s_tstamp: reader.read_u16::<LittleEndian>()?,
            param1: reader.read_u16::<LittleEndian>()?,
            param2: reader.read_u16::<LittleEndian>()?,
            param3: reader.read_u16::<LittleEndian>()?,
        })
    }
}

/// 字段头部 (6 bytes)
#[derive(Clone, Debug)]
pub struct FieldHeader {
    pub name: [u8; 4],
    pub dsize: u16,
}

impl FieldHeader {
    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        let mut name = [0u8; 4];
        reader.read_exact(&mut name)?;
        let dsize = reader.read_u16::<LittleEndian>()?;
        Ok(Self { name, dsize })
    }

    pub fn is_xxxx(&self) -> bool {
        &self.name == b"XXXX"
    }
}
