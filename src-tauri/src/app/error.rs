use serde::Serialize;
use std::io;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
    pub message: String,
}

impl AppError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn msg(message: impl Into<String>) -> Self {
        Self::new("error", message)
    }

    pub fn external_change() -> Self {
        Self::new("external_change", "文件已被其他程序修改")
    }

    pub fn asset_missing(reference: &str) -> Self {
        Self::new("asset_missing", format!("图片资源缺失：{reference}"))
    }
}

pub type AppResult<T> = Result<T, AppError>;

pub fn from_io(err: io::Error) -> AppError {
    let message = match err.kind() {
        io::ErrorKind::NotFound => "文档路径不存在".to_string(),
        io::ErrorKind::PermissionDenied => "原文件没有写入权限".to_string(),
        _ => match err.raw_os_error() {
            Some(112) | Some(39) => "磁盘空间不足".to_string(),
            Some(32) | Some(33) => "保存时发生文件占用或替换失败".to_string(),
            _ => format!("文件操作失败：{err}"),
        },
    };
    AppError::msg(message)
}
