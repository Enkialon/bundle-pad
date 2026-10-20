use std::path::Path;

use super::error::AppResult;
use super::stamp::FileStamp;

/// 已打开的 TextBundle 目录包。
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenedTextBundle {
    pub path: String,
    pub display_name: String,
    pub content: String,
    pub has_bom: bool,
    pub text_file_name: String,
    pub version: i64,
    pub stamp: FileStamp,
}

/// TextBundle 目录包的打开与保存端口。只处理已经存在的包。
pub trait TextBundleDoc {
    /// 打开已有的 `.textbundle` 目录，读取唯一的 `text.*` 正文，不修改 `info.json`。
    fn open(&self, path: &Path) -> AppResult<OpenedTextBundle>;

    /// 只回写正文文件。正文文件名变化、外部修改未确认，或写入过程改动了 `info.json` 时失败。
    fn save(
        &self,
        path: &Path,
        content: &str,
        has_bom: bool,
        text_file_name: &str,
        expected_sha256: Option<&str>,
        overwrite: bool,
    ) -> AppResult<FileStamp>;
}
