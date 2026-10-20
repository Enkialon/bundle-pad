use std::path::Path;

use crate::encoding::{encode_utf8, read_utf8};
use crate::error::{AppError, AppResult};
use crate::models::{AssetPayload, OpenedMarkdown, SavedAsset};
use crate::paths::{
    commit_bytes, display_name, friendly, is_markdown_extension, markdown_asset_dir, read_limited, resolve_markdown_image,
};
use crate::assets::{decode_image, payload_for_display, store_image};

pub fn open(path: &Path) -> AppResult<OpenedMarkdown> {
    if !path.is_file() {
        return Err(AppError::msg("文档路径不存在"));
    }
    if !is_markdown_extension(path) {
        return Err(AppError::msg("只能打开 .md 或 .markdown 文件"));
    }
    let decoded = read_utf8(path)?;
    let canonical = friendly(path)?;
    let stamp = crate::stamp::stamp_file(&canonical)?;
    Ok(OpenedMarkdown {
        display_name: display_name(&canonical),
        path: canonical.to_string_lossy().into_owned(),
        content: decoded.content,
        has_bom: decoded.has_bom,
        stamp,
    })
}

pub fn save(
    path: &Path,
    content: &str,
    has_bom: bool,
    expected_sha256: Option<&str>,
    overwrite: bool,
) -> AppResult<crate::stamp::FileStamp> {
    if !is_markdown_extension(path) {
        return Err(AppError::msg("只能保存为 .md 或 .markdown 文件"));
    }
    if expected_sha256.is_none() {
        let parent = path.parent().filter(|parent| parent.is_dir()).ok_or_else(|| AppError::msg("目标文件夹不存在"))?;
        let _ = parent;
    }
    let bytes = encode_utf8(content, has_bom);
    let stamp = commit_bytes(path, &bytes, expected_sha256, overwrite)?;
    Ok(stamp)
}

pub fn save_asset(doc_path: &Path, data_base64: &str) -> AppResult<SavedAsset> {
    let (directory, dir_name) = markdown_asset_dir(doc_path)?;
    let (bytes, mime) = decode_image(data_base64)?;
    store_image(&directory, &bytes, &mime, &dir_name)
}

pub fn import_file(doc_path: &Path, source_path: &Path) -> AppResult<SavedAsset> {
    let bytes = read_limited(source_path)?;
    let mime = crate::image_sniff::sniff_image(&bytes).ok_or_else(|| AppError::msg("无法根据文件内容识别图片格式"))?;
    let (directory, dir_name) = markdown_asset_dir(doc_path)?;
    store_image(&directory, &bytes, mime, &dir_name)
}

pub fn read_asset(doc_path: &Path, reference: &str) -> AppResult<AssetPayload> {
    if !doc_path.is_file() {
        return Err(AppError::msg("文档路径不存在"));
    }
    let path = resolve_markdown_image(doc_path, reference)?;
    payload_for_display(&path, reference)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use std::fs;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\nfake";

    #[test]
    fn saves_new_image_beside_markdown_and_keeps_bom() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("笔记.md");
        fs::write(&path, b"\xEF\xBB\xBFhello").unwrap();
        let opened = open(&path).unwrap();
        assert!(opened.has_bom);
        assert_eq!(opened.content, "hello");

        let encoded = base64::engine::general_purpose::STANDARD.encode(PNG);
        let saved = save_asset(Path::new(&opened.path), &encoded).unwrap();
        assert!(saved.relative_path.starts_with("笔记.assets/image-"));
        assert!(saved.relative_path.ends_with(".png"));
        assert!(!saved.relative_path.contains('\\'));

        let stamp = save(Path::new(&opened.path), "hello!", true, Some(&opened.stamp.sha256), false).unwrap();
        let bytes = fs::read(&opened.path).unwrap();
        assert_eq!(&bytes[0..3], &[0xEF, 0xBB, 0xBF]);
        assert!(String::from_utf8_lossy(&bytes).contains("hello!"));
        assert_ne!(stamp.sha256, opened.stamp.sha256);
    }

    #[test]
    fn external_change_does_not_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.md");
        fs::write(&path, "original").unwrap();
        let opened = open(&path).unwrap();
        fs::write(&path, "external").unwrap();
        let error = save(Path::new(&opened.path), "local", false, Some(&opened.stamp.sha256), false).unwrap_err();
        assert_eq!(error.code, "external_change");
        assert_eq!(fs::read_to_string(&path).unwrap(), "external");
    }

    #[test]
    fn readonly_file_is_left_intact() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.md");
        fs::write(&path, "original").unwrap();
        let opened = open(&path).unwrap();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&path, permissions).unwrap();
        struct Unlock<'a>(&'a std::path::Path);
        impl Drop for Unlock<'_> {
            fn drop(&mut self) {
                if let Ok(metadata) = fs::metadata(self.0) {
                    let mut permissions = metadata.permissions();
                    permissions.set_readonly(false);
                    let _ = fs::set_permissions(self.0, permissions);
                }
            }
        }
        let _unlock = Unlock(Path::new(&opened.path));
        let error = save(Path::new(&opened.path), "changed", false, Some(&opened.stamp.sha256), false).unwrap_err();
        assert!(error.message.contains("权限") || error.message.contains("占用") || error.message.contains("失败"));
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_readonly(false);
        fs::set_permissions(&path, permissions).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "original");
    }
}
