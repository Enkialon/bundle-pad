use std::path::Path;

use super::asset::FsAsset;
use super::classify::FsClassify;
use super::markdown::FsMarkdown;
use super::paths::{is_markdown_extension, is_textbundle_name};
use super::textbundle::FsTextBundle;
use crate::app::asset::{AssetPayload, AssetStore, SavedAsset};
use crate::app::classify::PathClassifier;
use crate::app::document::{DocumentStore, OpenedDocument};
use crate::app::error::{AppError, AppResult};
use crate::app::markdown::{MarkdownDoc, OpenedMarkdown};
use crate::app::stamp::FileStamp;
use crate::app::textbundle::{OpenedTextBundle, TextBundleDoc};

/// 按路径把文档操作分发给 Markdown 或 TextBundle 实现。
#[derive(Debug, Default, Clone, Copy)]
pub struct FsDocument;

enum Kind {
    Markdown,
    TextBundle,
}

impl DocumentStore for FsDocument {
    fn open(&self, path: &Path) -> AppResult<OpenedDocument> {
        let classified = FsClassify.classify(path)?;
        let canonical = Path::new(&classified.path);
        match classified.kind.as_str() {
            "markdown" => Ok(from_markdown(FsMarkdown.open(canonical)?)),
            "textbundle" => Ok(from_textbundle(FsTextBundle.open(canonical)?)),
            _ => Err(AppError::msg("不支持的文件类型")),
        }
    }

    fn save(
        &self,
        path: &Path,
        content: &str,
        has_bom: bool,
        text_file_name: Option<&str>,
        expected_sha256: Option<&str>,
        overwrite: bool,
    ) -> AppResult<FileStamp> {
        match kind_for_storage(path)? {
            Kind::Markdown => FsMarkdown.save(path, content, has_bom, expected_sha256, overwrite),
            Kind::TextBundle => FsTextBundle.save(path, content, has_bom, text_file_name.unwrap_or(""), expected_sha256, overwrite),
        }
    }

    fn read_asset(&self, document_path: &Path, reference: &str) -> AppResult<AssetPayload> {
        let classified = FsClassify.classify(document_path)?;
        let canonical = Path::new(&classified.path);
        match classified.kind.as_str() {
            "markdown" => FsAsset.read_markdown_asset(canonical, reference),
            "textbundle" => FsAsset.read_bundle_asset(canonical, reference),
            _ => Err(AppError::msg("不支持的文件类型")),
        }
    }

    fn write_asset(&self, document_path: &Path, data_base64: &str) -> AppResult<SavedAsset> {
        match kind_for_storage(document_path)? {
            Kind::Markdown => FsAsset.save_markdown_asset(document_path, data_base64),
            Kind::TextBundle => FsAsset.save_bundle_asset(document_path, data_base64),
        }
    }

    fn import_local_image(&self, document_path: &Path, source_path: &Path) -> AppResult<SavedAsset> {
        let classified = FsClassify.classify(document_path)?;
        match classified.kind.as_str() {
            "markdown" => FsAsset.import_local_image(Path::new(&classified.path), "markdown", source_path),
            "textbundle" => FsAsset.import_local_image(Path::new(&classified.path), "textbundle", source_path),
            _ => Err(AppError::msg("不支持的文件类型")),
        }
    }
}

/// 保存和写图片时，目标 Markdown 文件可能还不存在，因此按扩展名区分，而不是要求路径已经落盘。
fn kind_for_storage(path: &Path) -> AppResult<Kind> {
    if is_markdown_extension(path) {
        return Ok(Kind::Markdown);
    }
    if is_textbundle_name(path) {
        return Ok(Kind::TextBundle);
    }
    if path.is_dir() {
        return Err(AppError::msg("请选择以 .textbundle 结尾的文档包目录"));
    }
    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("").to_ascii_lowercase();
    if extension == "textpack" {
        return Err(AppError::msg("暂不支持 TextPack 压缩包"));
    }
    Err(AppError::msg("只能保存为 .md 或 .markdown 文件"))
}

fn from_markdown(opened: OpenedMarkdown) -> OpenedDocument {
    OpenedDocument {
        kind: "markdown".to_string(),
        path: opened.path,
        display_name: opened.display_name,
        content: opened.content,
        has_bom: opened.has_bom,
        stamp: opened.stamp,
        text_file_name: None,
        version: None,
    }
}

fn from_textbundle(opened: OpenedTextBundle) -> OpenedDocument {
    OpenedDocument {
        kind: "textbundle".to_string(),
        path: opened.path,
        display_name: opened.display_name,
        content: opened.content,
        has_bom: opened.has_bom,
        stamp: opened.stamp,
        text_file_name: Some(opened.text_file_name),
        version: Some(opened.version),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::document::DocumentStore;
    use std::fs;

    #[test]
    fn open_reports_kind_for_markdown_and_textbundle() {
        let dir = tempfile::tempdir().unwrap();
        let markdown = dir.path().join("note.md");
        fs::write(&markdown, "hello").unwrap();
        let opened = FsDocument.open(&markdown).unwrap();
        assert_eq!(opened.kind, "markdown");
        assert_eq!(opened.content, "hello");
        assert!(opened.text_file_name.is_none());

        let root = dir.path().join("demo.textbundle");
        fs::create_dir_all(root.join("assets")).unwrap();
        fs::write(root.join("info.json"), r#"{"version":2}"#).unwrap();
        fs::write(root.join("text.md"), "正文").unwrap();
        let bundle = FsDocument.open(&root).unwrap();
        assert_eq!(bundle.kind, "textbundle");
        assert_eq!(bundle.text_file_name.as_deref(), Some("text.md"));
        assert_eq!(bundle.version, Some(2));
    }
}
