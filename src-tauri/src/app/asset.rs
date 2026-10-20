use std::path::Path;

use super::error::AppResult;

/// 新写入文档的图片位置。
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedAsset {
    pub relative_path: String,
    pub mime_type: String,
}

/// 供编辑器显示的图片内容。
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetPayload {
    pub mime_type: String,
    pub data_base64: String,
}

/// 文档图片的读取、保存和导入端口。
pub trait AssetStore {
    /// 把 Base64 图片写入 Markdown 同级的 `<文档名>.assets/`，正文引用使用正斜杠相对路径。
    fn save_markdown_asset(&self, doc_path: &Path, data_base64: &str) -> AppResult<SavedAsset>;

    /// 把 Base64 图片写入 TextBundle 的 `assets/`。
    fn save_bundle_asset(&self, bundle_path: &Path, data_base64: &str) -> AppResult<SavedAsset>;

    /// 按引用读取 Markdown 旁的图片。远程地址不下载，缺失时保留原始引用对应的错误。
    fn read_markdown_asset(&self, doc_path: &Path, reference: &str) -> AppResult<AssetPayload>;

    /// 按包内相对路径读取 TextBundle 图片。只接受 `assets/` 下的路径，拒绝越出文档包。
    fn read_bundle_asset(&self, bundle_path: &Path, relative_path: &str) -> AppResult<AssetPayload>;

    /// 从本地文件导入图片。`document_kind` 为 `markdown` 或 `textbundle`，按文档类型落到对应目录。
    fn import_local_image(&self, document_path: &Path, document_kind: &str, source_path: &Path) -> AppResult<SavedAsset>;
}
