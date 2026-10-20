use serde::Serialize;

use crate::stamp::FileStamp;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenedMarkdown {
    pub path: String,
    pub display_name: String,
    pub content: String,
    pub has_bom: bool,
    pub stamp: FileStamp,
}

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedAsset {
    pub relative_path: String,
    pub mime_type: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetPayload {
    pub mime_type: String,
    pub data_base64: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassifiedPath {
    pub kind: String,
    pub path: String,
}
