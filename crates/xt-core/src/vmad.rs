//! VMAD (Virtual Machine Address Data) 解码器
//!
//! VMAD 是 Bethesda 用于在 ESP/ESM 文件中嵌入脚本属性的二进制格式。
//! 本模块解析 VMAD 二进制数据，提取可翻译的字符串属性。
//!
//! ## VMAD 二进制格式
//!
//! ```text
//! Header: version(i16) + objType(i16) + scriptCount(i16)
//! Scripts: scriptName(len+bytes) + propCount(i16) + properties[]
//! Property: name(len+bytes) + type(u8) + status(u8) + value
//! Types: 1=Null, 2=Object, 3=String, 4=Int, 5=Float, 6=Bool,
//!        7=Variable, 11=Struct, 12=StringArray, 13=IntArray,
//!        14=FloatArray, 15=BoolArray, 17=ArrayStruct(FO4)
//! Fragments: version 1-5=TES5, 6=FO4
//! ```


/// VMAD 字符串属性
#[derive(Clone, Debug)]
pub struct VmadString {
    /// 脚本名称
    pub script_name: String,
    /// 属性名称
    pub prop_name: String,
    /// 字符串值
    pub value: String,
    /// 在 VMAD buffer 中的字节偏移量（用于写回）
    pub offset: usize,
    /// 字符串长度（字节数）
    pub length: usize,
}

/// VMAD 属性类型
#[derive(Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum VmadPropType {
    Null = 1,
    Object = 2,
    String = 3,
    Int = 4,
    Float = 5,
    Bool = 6,
    Variable = 7,
    Struct = 11,
    StringArray = 12,
    IntArray = 13,
    FloatArray = 14,
    BoolArray = 15,
    ArrayStruct = 17,
}

impl From<u8> for VmadPropType {
    fn from(v: u8) -> Self {
        match v {
            1 => VmadPropType::Null,
            2 => VmadPropType::Object,
            3 => VmadPropType::String,
            4 => VmadPropType::Int,
            5 => VmadPropType::Float,
            6 => VmadPropType::Bool,
            7 => VmadPropType::Variable,
            11 => VmadPropType::Struct,
            12 => VmadPropType::StringArray,
            13 => VmadPropType::IntArray,
            14 => VmadPropType::FloatArray,
            15 => VmadPropType::BoolArray,
            17 => VmadPropType::ArrayStruct,
            _ => VmadPropType::Null,
        }
    }
}

/// VMAD 解码错误
#[derive(Debug)]
pub enum VmadError {
    Eof,
    InvalidData(String),
    WriteBackFailed(String),
}

impl std::fmt::Display for VmadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VmadError::Eof => write!(f, "Unexpected end of VMAD data"),
            VmadError::InvalidData(s) => write!(f, "Invalid VMAD data: {}", s),
            VmadError::WriteBackFailed(s) => write!(f, "VMAD write-back failed: {}", s),
        }
    }
}

impl std::error::Error for VmadError {}

/// VMAD 解码器
pub struct VmadDecoder {
    buffer: Vec<u8>,
    version: i16,
}

impl VmadDecoder {
    /// 创建新的 VMAD 解码器
    ///
    /// * `buffer` - VMAD 字段的完整数据（不包含字段头）
    /// * `version` - VMAD 版本（从父记录头部获取）
    pub fn new(buffer: &[u8], version: i16) -> Self {
        Self {
            buffer: buffer.to_vec(),
            version,
        }
    }

    /// 解码 VMAD buffer，提取所有字符串类型属性
    pub fn decode(&self) -> Vec<VmadString> {
        let mut result = Vec::new();
        let mut pos = 0usize;

        // 读取 Header
        let (_, _obj_type, script_count) = match Self::read_header(&self.buffer, &mut pos) {
            Ok(v) => v,
            Err(_) => return result,
        };

        // 遍历每个脚本
        for _ in 0..script_count {
            let script_name = match Self::read_length_prefixed_string(&self.buffer, &mut pos) {
                Ok(s) => s,
                Err(_) => break,
            };

            let prop_count = match Self::read_u16(&self.buffer, &mut pos) {
                Ok(p) => p,
                Err(_) => break,
            };

            // 遍历每个属性
            for _ in 0..prop_count {
                let _prop_name_start = pos;
                let prop_name = match Self::read_length_prefixed_string(&self.buffer, &mut pos) {
                    Ok(p) => p,
                    Err(_) => break,
                };

                let prop_type_byte = match Self::read_u8(&self.buffer, &mut pos) {
                    Ok(b) => b,
                    Err(_) => break,
                };
                let prop_type = VmadPropType::from(prop_type_byte);

                let _status = match Self::read_u8(&self.buffer, &mut pos) {
                    Ok(s) => s,
                    Err(_) => break,
                };

                // 根据类型读取值
                match prop_type {
                    VmadPropType::String => {
                        let str_start = pos; // start of u32 length prefix
                        if let Ok(len) = Self::read_u32(&self.buffer, &mut pos) {
                            let len = len as usize;
                            let str_end = (pos + len).min(self.buffer.len());
                            let value = String::from_utf8(self.buffer[pos..str_end].to_vec())
                                .unwrap_or_default();
                            pos = str_end;
                            result.push(VmadString {
                                script_name: script_name.clone(),
                                prop_name: prop_name.clone(),
                                value,
                                offset: str_start,
                                length: 4 + len, // u32 length prefix + string data
                            });
                        }
                    }
                    VmadPropType::StringArray => {
                        let arr_start = pos;
                        if let Ok(arr) = Self::read_string_array(&self.buffer, &mut pos) {
                            // StringArray 中每个字符串都有独立的偏移量，用于写回
                            // 计算每个字符串的偏移量：跳过 count(u32) 后，每个字符串有 len(u8/u32) + data
                            let mut item_offset = arr_start + 4; // 跳过 count(u32)
                            for s in &arr {
                                // 长度前缀是 u32 (4 bytes)，参见 read_length_prefixed_string
                                // 但实际 read_length_prefixed_string 用的是 u8 长度前缀
                                // 这里需要匹配实际格式
                                result.push(VmadString {
                                    script_name: script_name.clone(),
                                    prop_name: prop_name.clone(),
                                    value: s.clone(),
                                    offset: item_offset, // 指向 len 前缀的位置
                                    length: 1 + s.len(), // u8 len prefix + string data
                                });
                                item_offset += 1 + s.len(); // 移动到下一个字符串
                            }
                        }
                    }
                    VmadPropType::Null => {
                        // 无数据
                    }
                    VmadPropType::Int => {
                        let _ = Self::read_i32(&self.buffer, &mut pos);
                    }
                    VmadPropType::Float => {
                        let _ = Self::read_f32(&self.buffer, &mut pos);
                    }
                    VmadPropType::Bool => {
                        let _ = Self::read_u8(&self.buffer, &mut pos);
                    }
                    VmadPropType::Object => {
                        let _ = Self::read_u32(&self.buffer, &mut pos); // formid
                    }
                    VmadPropType::Variable => {
                        let _ = Self::read_u8(&self.buffer, &mut pos); // type hint
                        let _ = Self::read_u8(&self.buffer, &mut pos); // flags
                    }
                    VmadPropType::Struct => {
                        // Struct 是复杂类型，需要根据具体结构解析
                        let _ = Self::skip_struct_data(&self.buffer, &mut pos);
                    }
                    VmadPropType::IntArray => {
                        if let Ok(count) = Self::read_u32(&self.buffer, &mut pos) {
                            pos += (count as usize) * 4;
                        }
                    }
                    VmadPropType::FloatArray => {
                        if let Ok(count) = Self::read_u32(&self.buffer, &mut pos) {
                            pos += (count as usize) * 4;
                        }
                    }
                    VmadPropType::BoolArray => {
                        if let Ok(count) = Self::read_u32(&self.buffer, &mut pos) {
                            pos += count as usize;
                        }
                    }
                    VmadPropType::ArrayStruct => {
                        if let Ok(count) = Self::read_u32(&self.buffer, &mut pos) {
                            for _ in 0..count {
                                let _ = Self::skip_struct_data(&self.buffer, &mut pos);
                            }
                        }
                    }
                }
            }
        }

        result
    }

    /// 写回翻译到 VMAD buffer（支持变长字符串替换）
    ///
    /// 当新字符串长度与原字符串相同时，原地替换（最快）。
    /// 当长度不同时，重新构建整个 VMAD buffer 以保持结构正确。
    ///
    /// * `offset` - 字符串长度前缀在 buffer 中的偏移量（由 `decode()` 返回的 `VmadString.offset`）
    /// * `new_value` - 新的字符串值
    pub fn write_back(&mut self, offset: usize, new_value: &str) -> Result<(), VmadError> {
        if offset + 4 > self.buffer.len() {
            return Err(VmadError::WriteBackFailed(format!(
                "Offset {} out of bounds (buffer size {})",
                offset,
                self.buffer.len()
            )));
        }

        let old_length = u32::from_le_bytes([
            self.buffer[offset],
            self.buffer[offset + 1],
            self.buffer[offset + 2],
            self.buffer[offset + 3],
        ]) as usize;

        let new_bytes = new_value.as_bytes();

        if new_bytes.len() == old_length {
            // 原地替换（长度相同，最快路径）
            let data_start = offset + 4;
            let data_end = data_start + old_length;
            if data_end > self.buffer.len() {
                return Err(VmadError::WriteBackFailed(
                    "String data extends beyond buffer".to_string(),
                ));
            }
            self.buffer[data_start..data_end].copy_from_slice(new_bytes);
            Ok(())
        } else {
            // 变长替换：重新构建整个 buffer
            self.write_back_rebuild(offset, new_bytes)
        }
    }

    /// 变长字符串写回：重新构建 VMAD buffer
    ///
    /// 策略：重新序列化整个 VMAD 结构，在目标偏移量处替换字符串。
    fn write_back_rebuild(&mut self, target_offset: usize, new_bytes: &[u8]) -> Result<(), VmadError> {
        let mut pos = 0usize;

        // 读取 header
        let (version, obj_type, script_count) = Self::read_header(&self.buffer, &mut pos)?;

        let mut out = Vec::with_capacity(self.buffer.len() + new_bytes.len());
        // 写入 header
        out.extend_from_slice(&version.to_le_bytes());
        out.extend_from_slice(&obj_type.to_le_bytes());
        out.extend_from_slice(&script_count.to_le_bytes());

        for _ in 0..script_count {
            // 读取并写入 script name
            let script_name_len = Self::read_u8(&self.buffer, &mut pos)? as usize;
            Self::ensure_remaining(&self.buffer, pos, script_name_len)?;
            out.push(script_name_len as u8);
            out.extend_from_slice(&self.buffer[pos..pos + script_name_len]);
            pos += script_name_len;

            // 读取并写入 prop count
            let prop_count = Self::read_u16(&self.buffer, &mut pos)?;
            out.extend_from_slice(&prop_count.to_le_bytes());

            for _ in 0..prop_count {
                // 读取并写入 prop name
                let prop_name_len = Self::read_u8(&self.buffer, &mut pos)? as usize;
                Self::ensure_remaining(&self.buffer, pos, prop_name_len)?;
                out.push(prop_name_len as u8);
                out.extend_from_slice(&self.buffer[pos..pos + prop_name_len]);
                pos += prop_name_len;

                // 读取并写入 type + status
                let prop_type_byte = Self::read_u8(&self.buffer, &mut pos)?;
                let status = Self::read_u8(&self.buffer, &mut pos)?;
                out.push(prop_type_byte);
                out.push(status);

                let prop_type = VmadPropType::from(prop_type_byte);

                match prop_type {
                    VmadPropType::String => {
                        let str_len = Self::read_u32(&self.buffer, &mut pos)? as usize;
                        Self::ensure_remaining(&self.buffer, pos, str_len)?;

                        // 检查是否是目标字符串
                        let str_prefix_offset = pos - 4; // 回退到 u32 长度前缀的位置
                        if str_prefix_offset == target_offset {
                            // 替换为新字符串
                            out.extend_from_slice(&(new_bytes.len() as u32).to_le_bytes());
                            out.extend_from_slice(new_bytes);
                        } else {
                            // 保持原样
                            out.extend_from_slice(&(str_len as u32).to_le_bytes());
                            out.extend_from_slice(&self.buffer[pos..pos + str_len]);
                        }
                        pos += str_len;
                    }
                    VmadPropType::StringArray => {
                        let count = Self::read_u32(&self.buffer, &mut pos)? as usize;
                        out.extend_from_slice(&(count as u32).to_le_bytes());

                        for _ in 0..count {
                            let str_len = Self::read_u32(&self.buffer, &mut pos)? as usize;
                            Self::ensure_remaining(&self.buffer, pos, str_len)?;

                            // 检查是否是目标字符串（StringArray 中的每个字符串都有独立的长度前缀）
                            let str_prefix_offset = pos - 4;
                            if str_prefix_offset == target_offset {
                                out.extend_from_slice(&(new_bytes.len() as u32).to_le_bytes());
                                out.extend_from_slice(new_bytes);
                            } else {
                                out.extend_from_slice(&(str_len as u32).to_le_bytes());
                                out.extend_from_slice(&self.buffer[pos..pos + str_len]);
                            }
                            pos += str_len;
                        }
                    }
                    VmadPropType::Null => {
                        // 无数据
                    }
                    VmadPropType::Int => {
                        Self::ensure_remaining(&self.buffer, pos, 4)?;
                        out.extend_from_slice(&self.buffer[pos..pos + 4]);
                        pos += 4;
                    }
                    VmadPropType::Float => {
                        Self::ensure_remaining(&self.buffer, pos, 4)?;
                        out.extend_from_slice(&self.buffer[pos..pos + 4]);
                        pos += 4;
                    }
                    VmadPropType::Bool => {
                        let v = Self::read_u8(&self.buffer, &mut pos)?;
                        out.push(v);
                    }
                    VmadPropType::Object => {
                        Self::ensure_remaining(&self.buffer, pos, 4)?;
                        out.extend_from_slice(&self.buffer[pos..pos + 4]);
                        pos += 4;
                    }
                    VmadPropType::Variable => {
                        Self::ensure_remaining(&self.buffer, pos, 2)?;
                        out.extend_from_slice(&self.buffer[pos..pos + 2]);
                        pos += 2;
                    }
                    VmadPropType::Struct => {
                        self.rebuild_struct_data(&mut out, &mut pos)?;
                    }
                    VmadPropType::IntArray => {
                        let count = Self::read_u32(&self.buffer, &mut pos)? as usize;
                        let byte_count = count * 4;
                        Self::ensure_remaining(&self.buffer, pos, byte_count)?;
                        out.extend_from_slice(&(count as u32).to_le_bytes());
                        out.extend_from_slice(&self.buffer[pos..pos + byte_count]);
                        pos += byte_count;
                    }
                    VmadPropType::FloatArray => {
                        let count = Self::read_u32(&self.buffer, &mut pos)? as usize;
                        let byte_count = count * 4;
                        Self::ensure_remaining(&self.buffer, pos, byte_count)?;
                        out.extend_from_slice(&(count as u32).to_le_bytes());
                        out.extend_from_slice(&self.buffer[pos..pos + byte_count]);
                        pos += byte_count;
                    }
                    VmadPropType::BoolArray => {
                        let count = Self::read_u32(&self.buffer, &mut pos)? as usize;
                        Self::ensure_remaining(&self.buffer, pos, count)?;
                        out.extend_from_slice(&(count as u32).to_le_bytes());
                        out.extend_from_slice(&self.buffer[pos..pos + count]);
                        pos += count;
                    }
                    VmadPropType::ArrayStruct => {
                        let count = Self::read_u32(&self.buffer, &mut pos)? as usize;
                        out.extend_from_slice(&(count as u32).to_le_bytes());
                        for _ in 0..count {
                            self.rebuild_struct_data(&mut out, &mut pos)?;
                        }
                    }
                }
            }
        }

        self.buffer = out;
        Ok(())
    }

    /// 辅助：重建 struct 数据（count + elements）
    fn rebuild_struct_data(&self, out: &mut Vec<u8>, pos: &mut usize) -> Result<(), VmadError> {
        let count = Self::read_u32(&self.buffer, pos)? as usize;
        let byte_count = count * 12; // 每个 element: type(u32) + size(u32) + offset(u32)
        Self::ensure_remaining(&self.buffer, *pos, byte_count)?;
        out.extend_from_slice(&(count as u32).to_le_bytes());
        out.extend_from_slice(&self.buffer[*pos..*pos + byte_count]);
        *pos += byte_count;
        Ok(())
    }

    fn ensure_remaining(data: &[u8], pos: usize, needed: usize) -> Result<(), VmadError> {
        if pos + needed > data.len() {
            Err(VmadError::Eof)
        } else {
            Ok(())
        }
    }

    /// 获取原始 buffer
    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }

    /// 获取 VMAD 版本
    pub fn version(&self) -> i16 {
        self.version
    }

    // === 私有辅助方法 ===

    fn read_header(data: &[u8], pos: &mut usize) -> Result<(i16, i16, u16), VmadError> {
        let version = Self::read_i16(data, pos)?;
        let obj_type = Self::read_i16(data, pos)?;
        let script_count = Self::read_u16(data, pos)?;
        Ok((version, obj_type, script_count))
    }

    fn read_length_prefixed_string(data: &[u8], pos: &mut usize) -> Result<String, VmadError> {
        let len = Self::read_u8(data, pos)? as usize;
        if *pos + len > data.len() {
            return Err(VmadError::Eof);
        }
        let s = std::str::from_utf8(&data[*pos..*pos + len])
            .map_err(|e| VmadError::InvalidData(e.to_string()))?
            .to_string();
        *pos += len;
        Ok(s)
    }

    fn read_u8(data: &[u8], pos: &mut usize) -> Result<u8, VmadError> {
        if *pos >= data.len() {
            return Err(VmadError::Eof);
        }
        let v = data[*pos];
        *pos += 1;
        Ok(v)
    }

    fn read_u16(data: &[u8], pos: &mut usize) -> Result<u16, VmadError> {
        if *pos + 2 > data.len() {
            return Err(VmadError::Eof);
        }
        let v = u16::from_le_bytes([data[*pos], data[*pos + 1]]);
        *pos += 2;
        Ok(v)
    }

    fn read_u32(data: &[u8], pos: &mut usize) -> Result<u32, VmadError> {
        if *pos + 4 > data.len() {
            return Err(VmadError::Eof);
        }
        let v = u32::from_le_bytes([data[*pos], data[*pos + 1], data[*pos + 2], data[*pos + 3]]);
        *pos += 4;
        Ok(v)
    }

    fn read_i16(data: &[u8], pos: &mut usize) -> Result<i16, VmadError> {
        if *pos + 2 > data.len() {
            return Err(VmadError::Eof);
        }
        let v = i16::from_le_bytes([data[*pos], data[*pos + 1]]);
        *pos += 2;
        Ok(v)
    }

    fn read_i32(data: &[u8], pos: &mut usize) -> Result<i32, VmadError> {
        if *pos + 4 > data.len() {
            return Err(VmadError::Eof);
        }
        let v = i32::from_le_bytes([data[*pos], data[*pos + 1], data[*pos + 2], data[*pos + 3]]);
        *pos += 4;
        Ok(v)
    }

    fn read_f32(data: &[u8], pos: &mut usize) -> Result<f32, VmadError> {
        if *pos + 4 > data.len() {
            return Err(VmadError::Eof);
        }
        let v = f32::from_le_bytes([data[*pos], data[*pos + 1], data[*pos + 2], data[*pos + 3]]);
        *pos += 4;
        Ok(v)
    }

    fn read_string_array(data: &[u8], pos: &mut usize) -> Result<Vec<String>, VmadError> {
        let count = Self::read_u32(data, pos)? as usize;
        let mut arr = Vec::with_capacity(count);
        for _ in 0..count {
            arr.push(Self::read_length_prefixed_string(data, pos)?);
        }
        Ok(arr)
    }

    fn skip_struct_data(data: &[u8], pos: &mut usize) -> Result<(), VmadError> {
        // Struct 格式：count(u32) + elements
        let count = Self::read_u32(data, pos)? as usize;
        for _ in 0..count {
            Self::read_u32(data, pos)?; // type
            Self::read_u32(data, pos)?; // size
            Self::read_u32(data, pos)?; // offset
        }
        Ok(())
    }
}

/// 零分配 VMAD 解码：直接从 &[u8] 提取字符串，不构建 VmadDecoder
///
/// 用于解析时的快速路径，避免 buffer.to_vec() 堆分配。
pub(crate) fn decode_vmad_fast(data: &[u8], _version: i16) -> Vec<VmadString> {
    let mut result = Vec::new();
    let mut pos = 0usize;

    if data.len() < 6 {
        return result;
    }

    // Header: version (i16, 2 bytes), objType (i16, 2 bytes), scriptCount (u16, 2 bytes)
    // version 由调用方在 RecordHeaderData 已读取，此处跳过以对齐 objType
    pos += 2; // skip version

    let _obj_type = if pos + 2 <= data.len() {
        i16::from_le_bytes([data[pos], data[pos + 1]])
    } else {
        return result;
    };
    pos += 2;

    let script_count = if pos + 2 <= data.len() {
        u16::from_le_bytes([data[pos], data[pos + 1]]) as usize
    } else {
        return result;
    };
    pos += 2;

    for _ in 0..script_count {
        // Read script name (u8 length prefix)
        let name_len = if pos < data.len() { data[pos] as usize } else { break };
        pos += 1;
        if pos + name_len > data.len() { break; }
        let script_name = std::str::from_utf8(&data[pos..pos + name_len])
            .unwrap_or("")
            .to_string();
        pos += name_len;

        // Read prop count (u16)
        let prop_count = if pos + 2 <= data.len() {
            u16::from_le_bytes([data[pos], data[pos + 1]]) as usize
        } else {
            break;
        };
        pos += 2;

        for _ in 0..prop_count {
            // Property name (u8 length prefix)
            let pname_len = if pos < data.len() { data[pos] as usize } else { break };
            pos += 1;
            if pos + pname_len > data.len() { break; }
            let prop_name = std::str::from_utf8(&data[pos..pos + pname_len])
                .unwrap_or("")
                .to_string();
            pos += pname_len;

            // Type (u8)
            let prop_type_byte = if pos < data.len() { data[pos] } else { break };
            pos += 1;

            // Status (u8)
            if pos >= data.len() { break; }
            pos += 1;

            // Read value based on type
            match prop_type_byte {
                3 => { // String type
                    if pos + 4 > data.len() { break; }
                    let len = u32::from_le_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]]) as usize;
                    pos += 4;
                    let str_end = (pos + len).min(data.len());
                    let value = String::from_utf8_lossy(&data[pos..str_end]).to_string();
                    pos = str_end;
                    if !value.is_empty() {
                        // 偏移量 = 当前 pos（字符串内容末尾）向后回退到长度前缀的起始位置
                        let offset = pos.saturating_sub(4).saturating_sub(len);
                        result.push(VmadString {
                            script_name: script_name.clone(),
                            prop_name: prop_name.clone(),
                            value,
                            offset,
                            length: len,
                        });
                    }
                }
                1 | 4 | 6 | 7 => { pos += 1; } // Null, Int (u8), Bool, Variable
                2 => { pos += 4; } // Object (u32 formid)
                5 => { pos += 4; } // Float (f32)
                11 => { // Struct
                    if pos + 4 > data.len() { break; }
                    let count = u32::from_le_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]]) as usize;
                    pos += 4 + count * 12; // 3 × u32 per element
                }
                12 | 14 => { // StringArray, FloatArray
                    if pos + 4 > data.len() { break; }
                    let count = u32::from_le_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]]) as usize;
                    pos += 4 + count * 4;
                }
                13 => { // IntArray
                    if pos + 4 > data.len() { break; }
                    let count = u32::from_le_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]]) as usize;
                    pos += 4 + count * 4;
                }
                15 => { // BoolArray
                    if pos + 4 > data.len() { break; }
                    let count = u32::from_le_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]]) as usize;
                    pos += 4 + count;
                }
                17 => { // ArrayStruct (FO4)
                    if pos + 4 > data.len() { break; }
                    let count = u32::from_le_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]]) as usize;
                    pos += 4;
                    for _ in 0..count {
                        pos += 12; // 3 × u32 per struct element
                    }
                }
                _ => {} // Unknown type, skip
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_simple_vmad() {
        // 构造简单的 VMAD 数据:
        // version=5, objType=1, scriptCount=1
        // scriptName="TestScript", propCount=1
        // propName="Text", type=3(String), status=0
        // value="Hello World"
        let mut data = Vec::new();

        // Header
        data.extend_from_slice(&5i16.to_le_bytes()); // version
        data.extend_from_slice(&1i16.to_le_bytes()); // objType
        data.extend_from_slice(&1u16.to_le_bytes()); // scriptCount

        // Script name (1-byte len prefix)
        data.push(10); // len
        data.extend_from_slice(b"TestScript");

        // propCount
        data.extend_from_slice(&1u16.to_le_bytes());

        // Property name (1-byte len prefix)
        data.push(4); // len
        data.extend_from_slice(b"Text");

        // type=3 (String), status=0
        data.push(3u8);
        data.push(0u8);

        // String value (u32 length prefix)
        data.extend_from_slice(&11u32.to_le_bytes()); // length = 11
        data.extend_from_slice(b"Hello World");

        let decoder = VmadDecoder::new(&data, 5);
        let strings = decoder.decode();

        assert_eq!(strings.len(), 1);
        assert_eq!(strings[0].script_name, "TestScript");
        assert_eq!(strings[0].prop_name, "Text");
        assert_eq!(strings[0].value, "Hello World");
    }

    #[test]
    fn test_decode_empty_vmad() {
        let decoder = VmadDecoder::new(&[], 5);
        let strings = decoder.decode();
        assert!(strings.is_empty());
    }

    #[test]
    fn test_decode_multiple_scripts() {
        let mut data = Vec::new();

        // Header
        data.extend_from_slice(&5i16.to_le_bytes());
        data.extend_from_slice(&1i16.to_le_bytes());
        data.extend_from_slice(&2u16.to_le_bytes()); // 2 scripts

        // Script 1
        data.push(4); // script name len
        data.extend_from_slice(b"Scp1"); // 4 bytes
        data.extend_from_slice(&1u16.to_le_bytes()); // 1 prop
        data.push(4); // prop name len
        data.extend_from_slice(b"Prp1"); // 4 bytes
        data.push(3u8); // String type
        data.push(0u8); // status
        data.extend_from_slice(&4u32.to_le_bytes()); // string value len
        data.extend_from_slice(b"Val1"); // 4 bytes

        // Script 2
        data.push(4); // script name len
        data.extend_from_slice(b"Scp2"); // 4 bytes
        data.extend_from_slice(&1u16.to_le_bytes()); // 1 prop
        data.push(4); // prop name len
        data.extend_from_slice(b"Prp2"); // 4 bytes
        data.push(3u8); // String type
        data.push(0u8); // status
        data.extend_from_slice(&4u32.to_le_bytes()); // string value len
        data.extend_from_slice(b"Val2"); // 4 bytes

        let decoder = VmadDecoder::new(&data, 5);
        let strings = decoder.decode();

        assert_eq!(strings.len(), 2);
        assert_eq!(strings[0].script_name, "Scp1");
        assert_eq!(strings[0].value, "Val1");
        assert_eq!(strings[1].script_name, "Scp2");
        assert_eq!(strings[1].value, "Val2");
    }

    #[test]
    fn test_write_back() {
        // VMAD strings use u32 (4-byte) length prefix
        // After type=3, status=0 at positions 18-19:
        // String length (4 bytes) at 19-22 = 2
        // String data (2 bytes) at 23-24 = "Hi"

        // Build minimal VMAD data
        let mut data = Vec::new();
        data.extend_from_slice(&5i16.to_le_bytes()); // version
        data.extend_from_slice(&1i16.to_le_bytes()); // objType
        data.extend_from_slice(&1u16.to_le_bytes()); // scriptCount=1
        data.push(4);
        data.extend_from_slice(b"Test"); // scriptName
        data.extend_from_slice(&1u16.to_le_bytes()); // propCount=1
        data.push(3);
        data.extend_from_slice(b"Prp"); // propName
        data.push(3u8); // type=String
        data.push(0u8); // status
        // String: u32 length = 2 (4 bytes: 02 00 00 00), then "Hi"
        data.extend_from_slice(&2u32.to_le_bytes());
        data.extend_from_slice(b"Hi");

        let mut decoder = VmadDecoder::new(&data, 5);
        let strings = decoder.decode();

        assert_eq!(strings.len(), 1);
        assert_eq!(strings[0].script_name, "Test");
        assert_eq!(strings[0].prop_name, "Prp");
        assert_eq!(strings[0].value, "Hi");

        // write_back at the offset of the u32 length prefix
        decoder.write_back(strings[0].offset, "OK").unwrap();

        let buf = decoder.buffer();
        // String data starts 4 bytes after the length prefix offset
        let str_start = strings[0].offset + 4;
        let result = std::str::from_utf8(&buf[str_start..str_start + 2]).unwrap();
        assert_eq!(result, "OK");
    }

    #[test]
    fn test_write_back_variable_length() {
        // 测试变长字符串替换（新字符串比原字符串长）
        let mut data = Vec::new();
        data.extend_from_slice(&5i16.to_le_bytes()); // version
        data.extend_from_slice(&1i16.to_le_bytes()); // objType
        data.extend_from_slice(&1u16.to_le_bytes()); // scriptCount=1
        data.push(4);
        data.extend_from_slice(b"Test"); // scriptName
        data.extend_from_slice(&1u16.to_le_bytes()); // propCount=1
        data.push(3);
        data.extend_from_slice(b"Prp"); // propName
        data.push(3u8); // type=String
        data.push(0u8); // status
        data.extend_from_slice(&2u32.to_le_bytes()); // string length = 2
        data.extend_from_slice(b"Hi"); // "Hi"

        let mut decoder = VmadDecoder::new(&data, 5);
        let strings = decoder.decode();
        assert_eq!(strings.len(), 1);
        assert_eq!(strings[0].value, "Hi");

        // 替换为更长的字符串
        decoder.write_back(strings[0].offset, "Hello World!").unwrap();

        // 重新解码验证
        let strings2 = decoder.decode();
        assert_eq!(strings2.len(), 1);
        assert_eq!(strings2[0].value, "Hello World!");
        assert_eq!(strings2[0].script_name, "Test");
        assert_eq!(strings2[0].prop_name, "Prp");
    }

    #[test]
    fn test_write_back_variable_length_shorter() {
        // 测试变长字符串替换（新字符串比原字符串短）
        let mut data = Vec::new();
        data.extend_from_slice(&5i16.to_le_bytes());
        data.extend_from_slice(&1i16.to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());
        data.push(4);
        data.extend_from_slice(b"Test");
        data.extend_from_slice(&1u16.to_le_bytes());
        data.push(3);
        data.extend_from_slice(b"Prp");
        data.push(3u8);
        data.push(0u8);
        data.extend_from_slice(&11u32.to_le_bytes()); // length = 11
        data.extend_from_slice(b"Hello World");

        let mut decoder = VmadDecoder::new(&data, 5);
        let strings = decoder.decode();
        assert_eq!(strings[0].value, "Hello World");

        // 替换为更短的字符串
        decoder.write_back(strings[0].offset, "OK").unwrap();

        let strings2 = decoder.decode();
        assert_eq!(strings2.len(), 1);
        assert_eq!(strings2[0].value, "OK");
    }

    #[test]
    fn test_write_back_multiple_scripts() {
        // 测试多脚本场景下的变长替换（只替换第二个脚本的字符串）
        let mut data = Vec::new();
        data.extend_from_slice(&5i16.to_le_bytes());
        data.extend_from_slice(&1i16.to_le_bytes());
        data.extend_from_slice(&2u16.to_le_bytes()); // 2 scripts

        // Script 1: "Scp1" -> prop "P1" = "Val1"
        data.push(4);
        data.extend_from_slice(b"Scp1");
        data.extend_from_slice(&1u16.to_le_bytes());
        data.push(2);
        data.extend_from_slice(b"P1");
        data.push(3u8);
        data.push(0u8);
        data.extend_from_slice(&4u32.to_le_bytes());
        data.extend_from_slice(b"Val1");

        // Script 2: "Scp2" -> prop "P2" = "Val2"
        data.push(4);
        data.extend_from_slice(b"Scp2");
        data.extend_from_slice(&1u16.to_le_bytes());
        data.push(2);
        data.extend_from_slice(b"P2");
        data.push(3u8);
        data.push(0u8);
        data.extend_from_slice(&4u32.to_le_bytes());
        data.extend_from_slice(b"Val2");

        let mut decoder = VmadDecoder::new(&data, 5);
        let strings = decoder.decode();
        assert_eq!(strings.len(), 2);
        assert_eq!(strings[0].value, "Val1");
        assert_eq!(strings[1].value, "Val2");

        // 替换第二个脚本的字符串为更长的值
        decoder.write_back(strings[1].offset, "ReplacedValue").unwrap();

        let strings2 = decoder.decode();
        assert_eq!(strings2.len(), 2);
        assert_eq!(strings2[0].value, "Val1"); // 第一个不变
        assert_eq!(strings2[1].value, "ReplacedValue"); // 第二个被替换
        assert_eq!(strings2[1].script_name, "Scp2");
    }

    #[test]
    fn test_decode_string_array() {
        // 测试 StringArray 类型的解码
        let mut data = Vec::new();
        data.extend_from_slice(&5i16.to_le_bytes()); // version
        data.extend_from_slice(&1i16.to_le_bytes()); // objType
        data.extend_from_slice(&1u16.to_le_bytes()); // scriptCount=1
        data.push(4);
        data.extend_from_slice(b"Test"); // scriptName
        data.extend_from_slice(&1u16.to_le_bytes()); // propCount=1
        data.push(3);
        data.extend_from_slice(b"Arr"); // propName
        data.push(12u8); // type=StringArray
        data.push(0u8); // status
        // StringArray: count(u32) + strings with u8 len prefix
        data.extend_from_slice(&3u32.to_le_bytes()); // count=3
        data.push(3); // len
        data.extend_from_slice(b"Foo"); // "Foo"
        data.push(3); // len
        data.extend_from_slice(b"Bar"); // "Bar"
        data.push(3); // len
        data.extend_from_slice(b"Baz"); // "Baz"

        let decoder = VmadDecoder::new(&data, 5);
        let strings = decoder.decode();

        // StringArray 应该产生 3 个独立的 VmadString 条目
        assert_eq!(strings.len(), 3);
        assert_eq!(strings[0].value, "Foo");
        assert_eq!(strings[1].value, "Bar");
        assert_eq!(strings[2].value, "Baz");
        assert_eq!(strings[0].script_name, "Test");
        assert_eq!(strings[0].prop_name, "Arr");
    }
}
