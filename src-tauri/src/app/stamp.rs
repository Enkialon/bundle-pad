use serde::Serialize;

/// 打开或保存后的文件戳，用于发现编辑期间的外部修改。
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileStamp {
    pub sha256: String,
    pub len: u64,
    pub modified_ms: u64,
}
