use base64::Engine;
use std::path::Path;

use crate::error::{AppError, AppResult};
use crate::image_sniff::{extension_for_mime, sniff_for_display, sniff_image, MAX_ASSET_BYTES};
use crate::models::{AssetPayload, SavedAsset};
use crate::paths::{read_limited, write_unique_image};

pub fn decode_image(data_base64: &str) -> AppResult<(Vec<u8>, String)> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data_base64.trim())
        .map_err(|_| AppError::msg("图片数据无法识别"))?;
    if bytes.is_empty() {
        return Err(AppError::msg("图片文件是空的"));
    }
    if bytes.len() as u64 > MAX_ASSET_BYTES {
        return Err(AppError::msg("图片文件过大"));
    }
    let mime = sniff_image(&bytes).ok_or_else(|| AppError::msg("无法根据文件内容识别图片格式"))?;
    Ok((bytes, mime.to_string()))
}

pub fn store_image(directory: &Path, bytes: &[u8], mime: &str, relative_dir: &str) -> AppResult<SavedAsset> {
    let extension = extension_for_mime(mime).ok_or_else(|| AppError::msg("无法根据文件内容识别图片格式"))?;
    let name = write_unique_image(directory, bytes, extension)?;
    Ok(SavedAsset {
        relative_path: format!("{relative_dir}/{name}"),
        mime_type: mime.to_string(),
    })
}

pub fn payload_for_display(path: &Path, reference: &str) -> AppResult<AssetPayload> {
    let bytes = read_limited(path)?;
    let mime = sniff_for_display(&bytes, reference).ok_or_else(|| AppError::msg("文件不是受支持的图片"))?;
    Ok(AssetPayload {
        mime_type: mime,
        data_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
    })
}
