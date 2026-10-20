use std::path::Path;

use super::encoding::{encode_utf8, read_utf8};
use super::paths::{commit_bytes, display_name, friendly, is_markdown_extension};
use super::stamp::stamp_file;
use crate::app::error::{AppError, AppResult};
use crate::app::markdown::{MarkdownDoc, OpenedMarkdown};
use crate::app::stamp::FileStamp;

/// 文件系统上的 Markdown 文档实现。
#[derive(Debug, Default, Clone, Copy)]
pub struct FsMarkdown;

impl MarkdownDoc for FsMarkdown {
    fn open(&self, path: &Path) -> AppResult<OpenedMarkdown> {
        if !path.is_file() {
            return Err(AppError::msg("文档路径不存在"));
        }
        if !is_markdown_extension(path) {
            return Err(AppError::msg("只能打开 .md 或 .markdown 文件"));
        }
        let decoded = read_utf8(path)?;
        let canonical = friendly(path)?;
        let stamp = stamp_file(&canonical)?;
        Ok(OpenedMarkdown {
            display_name: display_name(&canonical),
            path: canonical.to_string_lossy().into_owned(),
            content: decoded.content,
            has_bom: decoded.has_bom,
            stamp,
        })
    }

    fn save(&self, path: &Path, content: &str, has_bom: bool, expected_sha256: Option<&str>, overwrite: bool) -> AppResult<FileStamp> {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::asset::AssetStore;
    use crate::app::markdown::MarkdownDoc;
    use crate::infrastructure::asset::FsAsset;
    use base64::Engine;
    use std::fs;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\nfake";

    #[test]
    fn saves_new_image_beside_markdown_and_keeps_bom() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("笔记.md");
        fs::write(&path, b"\xEF\xBB\xBFhello").unwrap();
        let opened = FsMarkdown.open(&path).unwrap();
        assert!(opened.has_bom);
        assert_eq!(opened.content, "hello");

        let encoded = base64::engine::general_purpose::STANDARD.encode(PNG);
        let saved = FsAsset.save_markdown_asset(Path::new(&opened.path), &encoded).unwrap();
        assert!(saved.relative_path.starts_with("笔记.assets/image-"));
        assert!(saved.relative_path.ends_with(".png"));
        assert!(!saved.relative_path.contains('\\'));

        let stamp = FsMarkdown.save(Path::new(&opened.path), "hello!", true, Some(&opened.stamp.sha256), false).unwrap();
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
        let opened = FsMarkdown.open(&path).unwrap();
        fs::write(&path, "external").unwrap();
        let error = FsMarkdown.save(Path::new(&opened.path), "local", false, Some(&opened.stamp.sha256), false).unwrap_err();
        assert_eq!(error.code, "external_change");
        assert_eq!(fs::read_to_string(&path).unwrap(), "external");
    }

    #[test]
    fn readonly_file_is_left_intact() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.md");
        fs::write(&path, "original").unwrap();
        let opened = FsMarkdown.open(&path).unwrap();
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
        let error = FsMarkdown.save(Path::new(&opened.path), "changed", false, Some(&opened.stamp.sha256), false).unwrap_err();
        assert!(error.message.contains("权限") || error.message.contains("占用") || error.message.contains("失败"));
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_readonly(false);
        fs::set_permissions(&path, permissions).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "original");
    }
}
