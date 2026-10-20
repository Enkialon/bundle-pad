use std::path::Path;

use super::paths::{friendly, is_markdown_extension, is_textbundle_name};
use crate::app::classify::{ClassifiedPath, PathClassifier};
use crate::app::error::{AppError, AppResult};

/// 按路径区分 Markdown 文件和 TextBundle 目录。
#[derive(Debug, Default, Clone, Copy)]
pub struct FsClassify;

impl PathClassifier for FsClassify {
    fn classify(&self, path: &Path) -> AppResult<ClassifiedPath> {
        if !path.exists() {
            return Err(AppError::msg("文档路径不存在"));
        }
        if path.is_dir() {
            if !is_textbundle_name(path) {
                return Err(AppError::msg("请选择以 .textbundle 结尾的文档包目录"));
            }
            let canonical = friendly(path)?;
            return Ok(ClassifiedPath {
                kind: "textbundle".to_string(),
                path: canonical.to_string_lossy().into_owned(),
            });
        }
        let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("").to_ascii_lowercase();
        if extension == "textpack" {
            return Err(AppError::msg("暂不支持 TextPack 压缩包"));
        }
        if extension == "textbundle" {
            return Err(AppError::msg("TextBundle 是目录包，不能当作普通文件打开"));
        }
        if is_markdown_extension(path) {
            let canonical = friendly(path)?;
            return Ok(ClassifiedPath {
                kind: "markdown".to_string(),
                path: canonical.to_string_lossy().into_owned(),
            });
        }
        Err(AppError::msg("不支持的文件类型"))
    }
}
