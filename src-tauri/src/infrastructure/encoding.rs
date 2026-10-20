use std::path::Path;

use crate::app::error::{from_io, AppError, AppResult};

#[derive(Debug)]
pub struct DecodedText {
    pub content: String,
    pub has_bom: bool,
}

pub fn read_utf8(path: &Path) -> AppResult<DecodedText> {
    let bytes = std::fs::read(path).map_err(from_io)?;
    decode_utf8(&bytes)
}

pub fn decode_utf8(bytes: &[u8]) -> AppResult<DecodedText> {
    let (has_bom, rest) = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        (true, &bytes[3..])
    } else {
        (false, bytes)
    };
    let content = String::from_utf8(rest.to_vec()).map_err(|_| AppError::msg("文件不是有效的 UTF-8，已停止打开以避免丢失数据"))?;
    Ok(DecodedText { content, has_bom })
}

pub fn encode_utf8(content: &str, has_bom: bool) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(content.len() + 3);
    if has_bom {
        bytes.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    }
    bytes.extend_from_slice(content.as_bytes());
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn preserves_bom_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.md");
        fs::write(&path, b"\xEF\xBB\xBF# \xE4\xBD\xA0\xE5\xA5\xBD").unwrap();
        let decoded = read_utf8(&path).unwrap();
        assert!(decoded.has_bom);
        assert_eq!(decoded.content, "# 你好");
        assert_eq!(encode_utf8(&decoded.content, true)[0..3], [0xEF, 0xBB, 0xBF]);
    }

    #[test]
    fn rejects_invalid_utf8() {
        let error = decode_utf8(&[0x80]).unwrap_err();
        assert!(error.message.contains("UTF-8"));
    }
}
