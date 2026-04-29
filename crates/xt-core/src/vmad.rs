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
                        let str_start = pos;
                        if let Ok(arr) = Self::read_string_array(&self.buffer, &mut pos) {
                            let str_end = pos;
                            // StringArray 将多个字符串连接存储
                            result.push(VmadString {
                                script_name: script_name.clone(),
                                prop_name: prop_name.clone(),
                                value: arr.join("\x1F"), // 用 ASCII 单元分隔符连接
                                offset: str_start,
                                length: str_end - str_start,
                            });
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

    /// 写回翻译到 VMAD buffer（保持二进制结构不变）
    ///
    /// 注意：此方法仅支持简单的字符串替换，长度变化时需要重写整个 VMAD 结构。
    pub fn write_back(&mut self, offset: usize, new_value: &str) -> Result<(), VmadError> {
        if offset >= self.buffer.len() {
            return Err(VmadError::WriteBackFailed(format!(
                "Offset {} out of bounds (buffer size {})",
                offset,
                self.buffer.len()
            )));
        }

        // 将新值编码为 UTF-8
        let new_bytes = new_value.as_bytes();

        // 读取原有长度
        let old_length = if offset + 4 <= self.buffer.len() {
            u32::from_le_bytes([
                self.buffer[offset],
                self.buffer[offset + 1],
                self.buffer[offset + 2],
                self.buffer[offset + 3],
            ]) as usize
        } else {
            return Err(VmadError::WriteBackFailed(
                "Cannot read string length at offset".to_string(),
            ));
        };

        if new_bytes.len() != old_length {
            // 长度不匹配时返回错误（后续可实现完整重写）
            return Err(VmadError::WriteBackFailed(format!(
                "Length mismatch: new={}, old={}. Variable-length write-back not yet implemented.",
                new_bytes.len(),
                old_length
            )));
        }

        // 原地替换
        let data_start = offset + 4; // 4 bytes for length prefix
        let data_end = data_start + old_length;
        if data_end > self.buffer.len() {
            return Err(VmadError::WriteBackFailed(
                "String data extends beyond buffer".to_string(),
            ));
        }

        self.buffer[data_start..data_end].copy_from_slice(new_bytes);

        Ok(())
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
        let s = String::from_utf8(data[*pos..*pos + len].to_vec())
            .map_err(|e| VmadError::InvalidData(e.to_string()))?;
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
}
