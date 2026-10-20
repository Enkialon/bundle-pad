use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::app::error::{from_io, AppResult};
use crate::app::stamp::FileStamp;

pub fn stamp_bytes(bytes: &[u8], modified_ms: u64) -> FileStamp {
    FileStamp {
        sha256: sha256_hex(bytes),
        len: bytes.len() as u64,
        modified_ms,
    }
}

pub fn stamp_file(path: &Path) -> AppResult<FileStamp> {
    let bytes = fs::read(path).map_err(from_io)?;
    let modified = fs::metadata(path).map_err(from_io)?.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    let modified_ms = modified.duration_since(UNIX_EPOCH).map(|duration| duration.as_millis() as u64).unwrap_or(0);
    Ok(stamp_bytes(&bytes, modified_ms))
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_changes_when_bytes_change() {
        assert_ne!(sha256_hex(b"a"), sha256_hex(b"b"));
        assert_eq!(sha256_hex(b"a"), sha256_hex(b"a"));
    }
}
