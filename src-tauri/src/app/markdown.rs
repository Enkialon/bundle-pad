use std::path::Path;

use super::error::AppResult;
use super::stamp::FileStamp;

/// 已打开的 Markdown 文件。
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenedMarkdown {
    pub path: String,
    pub display_name: String,
    pub content: String,
    pub has_bom: bool,
    pub stamp: FileStamp,
}

/// Markdown 文件的打开与保存端口。
pub trait MarkdownDoc {
    /// 打开已存在的 `.md` 或 `.markdown` 文件，按 UTF-8 读取正文并保留 BOM 标记。
    fn open(&self, path: &Path) -> AppResult<OpenedMarkdown>;

    /// 把正文写回 Markdown 文件。磁盘内容的 SHA-256 与 `expected_sha256` 不一致且 `overwrite` 为 false 时，拒绝覆盖。
    fn save(
        &self,
        path: &Path,
        content: &str,
        has_bom: bool,
        expected_sha256: Option<&str>,
        overwrite: bool,
    ) -> AppResult<FileStamp>;
}
