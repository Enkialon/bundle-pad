use std::path::Path;

use super::error::AppResult;

/// 路径分类结果。`kind` 为 `markdown` 或 `textbundle`。
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassifiedPath {
    pub kind: String,
    pub path: String,
}

/// 按路径区分 Markdown 文件和 TextBundle 目录的端口。
pub trait PathClassifier {
    /// 判断路径种类并返回规范化路径。目录且以 `.textbundle` 结尾为文档包，`.textpack` 为 zip 压缩包，`.md` / `.markdown` 文件为 Markdown。
    fn classify(&self, path: &Path) -> AppResult<ClassifiedPath>;
}
