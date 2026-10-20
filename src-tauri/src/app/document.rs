use std::path::Path;

use super::asset::{AssetPayload, SavedAsset};
use super::error::AppResult;
use super::stamp::FileStamp;

/// 打开后的文档。`kind` 为 `markdown` 或 `textbundle`。
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenedDocument {
    pub kind: String,
    pub path: String,
    pub display_name: String,
    pub content: String,
    pub has_bom: bool,
    pub stamp: FileStamp,
    pub text_file_name: Option<String>,
    pub version: Option<i64>,
}

/// 面向文档的端口。前端不按文件格式拆命令，具体格式由路径决定。
pub trait DocumentStore {
    /// 识别路径并打开文档。目录且以 `.textbundle` 结尾按文档包打开，`.textpack` 按 zip 压缩包打开，`.md` / `.markdown` 按 Markdown 打开。
    fn open(&self, path: &Path) -> AppResult<OpenedDocument>;

    /// 按路径对应的文档类型回写正文。TextBundle 只写 `text.*`；外部修改未确认时拒绝覆盖。
    fn save(
        &self,
        path: &Path,
        content: &str,
        has_bom: bool,
        text_file_name: Option<&str>,
        expected_sha256: Option<&str>,
        overwrite: bool,
    ) -> AppResult<FileStamp>;

    /// 按文档类型读取图片。Markdown 按图片引用解析，TextBundle 只接受包内 `assets/`。
    fn read_asset(&self, document_path: &Path, reference: &str) -> AppResult<AssetPayload>;

    /// 按文档类型写入 Base64 图片。Markdown 落到 `<文档名>.assets/`，TextBundle 落到包内 `assets/`。
    fn write_asset(&self, document_path: &Path, data_base64: &str) -> AppResult<SavedAsset>;

    /// 把本地图片导入当前文档。落盘位置由文档路径决定，不需要前端再传文档类型。
    fn import_local_image(&self, document_path: &Path, source_path: &Path) -> AppResult<SavedAsset>;
}
