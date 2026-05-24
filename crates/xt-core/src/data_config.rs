//! Data/\<Game\>/ 配置文件解析
//!
//! 支持的文件：
//! - ctdaFunc.txt     - CTDA 条件函数定义 (ID=FuncName:{Params})
//! - fieldSizeRef.txt - 字段最大长度参考 (REC:FIELD:AuthCR=MaxSize)
//! - DialSubType.txt  - 对话子类型映射 (FormID=SubTypeName)
//! - EmoteDefinition.txt - 表情定义映射 (FormID=EmoteName)

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::str::FromStr;

/// CTDA 函数信息
#[derive(Clone, Debug)]
pub struct CtdaFunc {
    /// 函数名
    pub name: String,
    /// 参数签名
    pub params: String,
}

/// 字段大小信息
#[derive(Clone, Debug)]
pub struct FieldSizeInfo {
    /// 最大字符数
    pub max_size: u32,
    /// 是否允许换行
    pub can_wrap: bool,
}

/// 解析 ctdaFunc.txt
///
/// 格式：`ID=FuncName:{Params}`
/// 示例：
/// ```
/// 000=GetWantBlocking
/// 001=GetDistance:{1}
/// ACAC=ActorCollidewithActor  (0x prefix for hex)
/// ```
pub fn parse_ctda_func(path: &Path) -> HashMap<u32, CtdaFunc> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return HashMap::new(),
    };

    content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let eq_pos = line.find('=')?;
            let id_str = line[..eq_pos].trim();
            let rest = line[eq_pos + 1..].trim();

            // ID 解析：0x 前缀为十六进制，否则为十进制
            // Bethesda ctdaFunc.txt 使用十进制 ID（如 "001", "010", "672"）
            let id = if id_str.starts_with("0x") || id_str.starts_with("0X") {
                u32::from_str_radix(&id_str[2..], 16).ok()
            } else {
                u32::from_str(id_str).ok()
            }?;

            // 分割函数名和参数
            let (name, params) = if let Some(brace_pos) = rest.find(":{") {
                let name = rest[..brace_pos].to_string();
                let params = rest[brace_pos + 1..].to_string(); // 包含大括号
                (name, params)
            } else {
                (rest.to_string(), String::new())
            };

            Some((id, CtdaFunc { name, params }))
        })
        .collect()
}

/// 解析 fieldSizeRef.txt
///
/// 格式：`REC:FIELD:AuthCR=MaxSize`
/// 示例：
/// ```
/// ACTI:FULL:0=47
/// ACTI:RNAM:0=36
/// ALCH:FULL:0=43
/// ```
pub fn parse_field_size_ref(path: &Path) -> HashMap<String, FieldSizeInfo> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return HashMap::new(),
    };

    content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let eq_pos = line.find('=')?;
            let left = line[..eq_pos].trim();
            let right = line[eq_pos + 1..].trim();

            // 解析左侧 REC:FIELD:AuthCR
            let parts: Vec<&str> = left.split(':').collect();
            if parts.len() != 3 {
                return None;
            }

            let rec = parts[0];
            let field = parts[1];
            let auth_cr = parts[2];
            let can_wrap = auth_cr == "1";

            // 解析 MaxSize
            let max_size = u32::from_str(right).ok()?;

            let key = format!("{}:{}", rec.to_uppercase(), field.to_uppercase());
            Some((key, FieldSizeInfo { max_size, can_wrap }))
        })
        .collect()
}

/// 解析 DialSubType.txt
///
/// 格式：`FormID=SubTypeName`
/// 示例：
/// ```
/// 00000001=Anger
/// 00000002=Disgust
/// FFFFFFFF=Undefined
/// ```
pub fn parse_dial_sub_type(path: &Path) -> HashMap<u32, String> {
    parse_simple_hex_mapping(path)
}

/// 解析 EmoteDefinition.txt
///
/// 格式：`FormID=EmoteName`
/// 示例：
/// ```
/// 0018E866=Awed
/// 0018E865=Impressed
/// FFFFFFFF=Undefined
/// ```
pub fn parse_emote_definition(path: &Path) -> HashMap<u32, String> {
    parse_simple_hex_mapping(path)
}

/// 解析简单的 FormID=Name 格式文件
fn parse_simple_hex_mapping(path: &Path) -> HashMap<u32, String> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return HashMap::new(),
    };

    content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let eq_pos = line.find('=')?;
            let id_str = line[..eq_pos].trim();
            let name = line[eq_pos + 1..].trim().to_string();

            // FormID 是十六进制
            let id = u32::from_str_radix(id_str, 16).ok()?;

            Some((id, name))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp_file(content: &str) -> PathBuf {
        let dir = std::env::temp_dir();
        let path = dir.join(format!(
            "xt_test_{}.txt",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_parse_ctda_func() {
        let path = temp_file(
            "# ctda functions\n\
             000=GetWantBlocking\n\
             001=GetDistance:{1}\n\
             002=GetLocked\n\
             0x1F=HexExample\n",
        );
        let result = parse_ctda_func(&path);
        fs::remove_file(&path).ok();
        assert_eq!(result.len(), 4);
        assert_eq!(result.get(&0).unwrap().name, "GetWantBlocking");
        assert_eq!(result.get(&1).unwrap().name, "GetDistance");
        assert_eq!(result.get(&1).unwrap().params, "{1}");
        assert_eq!(result.get(&2).unwrap().params, "");
        assert_eq!(result.get(&0x1F).unwrap().name, "HexExample");
    }

    #[test]
    fn test_parse_ctda_func_decimal_not_hex() {
        // 验证 "010" 被解析为十进制 10，而不是十六进制 16
        let path = temp_file("010=TestFunc\n");
        let result = parse_ctda_func(&path);
        fs::remove_file(&path).ok();
        assert_eq!(result.len(), 1);
        assert_eq!(result.get(&10).unwrap().name, "TestFunc");
        assert!(result.get(&16).is_none()); // 不应该是 0x10 = 16
    }

    #[test]
    fn test_parse_field_size_ref() {
        let path = temp_file(
            "# field sizes\n\
             ACTI:FULL:0=47\n\
             ACTI:RNAM:1=36\n",
        );
        let result = parse_field_size_ref(&path);
        fs::remove_file(&path).ok();
        assert_eq!(result.len(), 2);
        assert_eq!(result.get("ACTI:FULL").unwrap().max_size, 47);
        assert!(!result.get("ACTI:FULL").unwrap().can_wrap);
        assert_eq!(result.get("ACTI:RNAM").unwrap().max_size, 36);
        assert!(result.get("ACTI:RNAM").unwrap().can_wrap);
    }

    #[test]
    fn test_parse_dial_sub_type() {
        let path = temp_file("00000001=Anger\nFFFFFFFF=Undefined\n");
        let result = parse_dial_sub_type(&path);
        fs::remove_file(&path).ok();
        assert_eq!(result.len(), 2);
        assert_eq!(result.get(&0x00000001).unwrap(), "Anger");
        assert_eq!(result.get(&0xFFFFFFFF).unwrap(), "Undefined");
    }

    #[test]
    fn test_parse_emote_definition() {
        let path = temp_file("0018E866=Awed\n0018E865=Impressed\n");
        let result = parse_emote_definition(&path);
        fs::remove_file(&path).ok();
        assert_eq!(result.len(), 2);
        assert_eq!(result.get(&0x0018E866).unwrap(), "Awed");
        assert_eq!(result.get(&0x0018E865).unwrap(), "Impressed");
    }

    #[test]
    fn test_empty_file() {
        let path = temp_file("");
        assert!(parse_ctda_func(&path).is_empty());
        assert!(parse_field_size_ref(&path).is_empty());
        assert!(parse_dial_sub_type(&path).is_empty());
        assert!(parse_emote_definition(&path).is_empty());
        fs::remove_file(&path).ok();
    }
}
