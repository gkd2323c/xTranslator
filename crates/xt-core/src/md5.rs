//! MD5 哈希工具 — 用于百度/有道翻译 API 签名认证
//!
//! 基于 `md-5` crate（Rust 生态标准实现，广泛审计）。
//! 对外暴露 `md5_hex()` 函数，签名与原有自定义实现一致。

/// 计算输入字符串的 MD5 哈希值，返回 32 位小写十六进制字符串。
pub fn md5_hex(input: &str) -> String {
    use md5::{Digest, Md5};
    let digest = Md5::digest(input.as_bytes());
    format!("{:032x}", digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_md5_empty() {
        assert_eq!(md5_hex(""), "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn test_md5_hello() {
        assert_eq!(md5_hex("hello"), "5d41402abc4b2a76b9719d911017c592");
    }

    #[test]
    fn test_md5_hello_world() {
        assert_eq!(md5_hex("Hello World"), "b10a8db164e0754105b7a99be72e3fe5");
    }

    #[test]
    fn test_md5_length() {
        assert_eq!(md5_hex("test").len(), 32);
    }
}
