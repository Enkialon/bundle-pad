use base64::Engine;
use std::path::Path;

use super::image_sniff::{extension_for_mime, sniff_for_display, sniff_image, MAX_ASSET_BYTES};
use super::paths::{
    ensure_assets_dir, markdown_asset_dir, read_limited, resolve_bundle_asset, resolve_markdown_image, resolve_staged_file,
    staging_images_dir, write_unique_image,
};
use super::textbundle::require_bundle;
use crate::app::asset::{AssetPayload, AssetStore, SavedAsset};
use crate::app::error::{AppError, AppResult};

/// 文件系统上的图片存储实现。
#[derive(Debug, Default, Clone, Copy)]
pub struct FsAsset;

impl AssetStore for FsAsset {
    fn save_markdown_asset(&self, doc_path: &Path, data_base64: &str) -> AppResult<SavedAsset> {
        let bytes = decode_bytes(data_base64)?;
        if doc_path.as_os_str().is_empty() {
            return store_staged(&staging_images_dir()?, &bytes, None);
        }
        let (directory, dir_name) = markdown_asset_dir(doc_path)?;
        let mime = sniff_image(&bytes).ok_or_else(|| AppError::msg("无法根据文件内容识别图片格式"))?;
        store_image(&directory, &bytes, mime, &dir_name)
    }

    fn save_bundle_asset(&self, bundle_path: &Path, data_base64: &str) -> AppResult<SavedAsset> {
        let bundle = require_bundle(bundle_path)?;
        let directory = ensure_assets_dir(&bundle)?;
        let (bytes, mime) = decode_image(data_base64)?;
        store_image(&directory, &bytes, &mime, "assets")
    }

    fn read_markdown_asset(&self, doc_path: &Path, reference: &str) -> AppResult<AssetPayload> {
        if doc_path.as_os_str().is_empty() {
            let path = resolve_staged_file(&staging_images_dir()?, reference)?;
            return payload_for_file(&path, reference);
        }
        if !doc_path.is_file() {
            return Err(AppError::msg("文档路径不存在"));
        }
        let path = resolve_markdown_image(doc_path, reference)?;
        payload_for_display(&path, reference)
    }

    fn read_bundle_asset(&self, bundle_path: &Path, relative_path: &str) -> AppResult<AssetPayload> {
        let bundle = require_bundle(bundle_path)?;
        let path = resolve_bundle_asset(&bundle, relative_path)?;
        if !path.is_file() {
            return Err(AppError::asset_missing(relative_path));
        }
        payload_for_display(&path, relative_path)
    }

    fn import_local_image(&self, document_path: &Path, document_kind: &str, source_path: &Path) -> AppResult<SavedAsset> {
        if document_path.as_os_str().is_empty() {
            let bytes = read_limited(source_path)?;
            return store_staged(&staging_images_dir()?, &bytes, extension_hint(source_path));
        }
        match document_kind {
            "markdown" => import_markdown(document_path, source_path),
            "textbundle" => import_bundle(document_path, source_path),
            _ => Err(AppError::msg("请先保存文档，再插入图片。")),
        }
    }
}

fn import_markdown(doc_path: &Path, source_path: &Path) -> AppResult<SavedAsset> {
    let bytes = read_limited(source_path)?;
    let mime = sniff_image(&bytes).ok_or_else(|| AppError::msg("无法根据文件内容识别图片格式"))?;
    let (directory, dir_name) = markdown_asset_dir(doc_path)?;
    store_image(&directory, &bytes, mime, &dir_name)
}

fn import_bundle(bundle_path: &Path, source_path: &Path) -> AppResult<SavedAsset> {
    let bundle = require_bundle(bundle_path)?;
    let bytes = read_limited(source_path)?;
    let mime = sniff_image(&bytes).ok_or_else(|| AppError::msg("无法根据文件内容识别图片格式"))?;
    let directory = ensure_assets_dir(&bundle)?;
    store_image(&directory, &bytes, mime, "assets")
}

fn decode_bytes(data_base64: &str) -> AppResult<Vec<u8>> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data_base64.trim())
        .map_err(|_| AppError::msg("图片数据无法识别"))?;
    if bytes.is_empty() {
        return Err(AppError::msg("图片文件是空的"));
    }
    if bytes.len() as u64 > MAX_ASSET_BYTES {
        return Err(AppError::msg("图片文件过大"));
    }
    Ok(bytes)
}

pub(crate) fn decode_image(data_base64: &str) -> AppResult<(Vec<u8>, String)> {
    let bytes = decode_bytes(data_base64)?;
    let mime = sniff_image(&bytes).ok_or_else(|| AppError::msg("无法根据文件内容识别图片格式"))?;
    Ok((bytes, mime.to_string()))
}

fn store_image(directory: &Path, bytes: &[u8], mime: &str, relative_dir: &str) -> AppResult<SavedAsset> {
    let extension = extension_for_mime(mime).ok_or_else(|| AppError::msg("无法根据文件内容识别图片格式"))?;
    let name = write_unique_image(directory, bytes, extension)?;
    Ok(SavedAsset {
        relative_path: format!("{relative_dir}/{name}"),
        mime_type: mime.to_string(),
    })
}

fn store_staged(directory: &Path, bytes: &[u8], extension_hint: Option<String>) -> AppResult<SavedAsset> {
    let prefix = dunce::simplified(directory).to_string_lossy().replace('\\', "/");
    if let Some(mime) = sniff_image(bytes) {
        return store_image(directory, bytes, mime, &prefix);
    }
    let extension = extension_hint.unwrap_or_else(|| "bin".to_string());
    let name = write_unique_image(directory, bytes, &extension)?;
    Ok(SavedAsset {
        relative_path: format!("{prefix}/{name}"),
        mime_type: "application/octet-stream".to_string(),
    })
}

fn extension_hint(path: &Path) -> Option<String> {
    let extension = path.extension()?.to_str()?;
    if extension.is_empty() || extension.len() > 10 || !extension.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
        return None;
    }
    Some(extension.to_ascii_lowercase())
}

fn payload_for_file(path: &Path, reference: &str) -> AppResult<AssetPayload> {
    let bytes = read_limited(path)?;
    let mime = sniff_for_display(&bytes, reference).unwrap_or_else(|| "application/octet-stream".to_string());
    Ok(AssetPayload {
        mime_type: mime,
        data_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
    })
}

fn payload_for_display(path: &Path, reference: &str) -> AppResult<AssetPayload> {
    let payload = payload_for_file(path, reference)?;
    if payload.mime_type == "application/octet-stream" {
        return Err(AppError::msg("文件不是受支持的图片"));
    }
    Ok(payload)
}
