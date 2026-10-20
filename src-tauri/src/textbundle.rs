use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

use crate::assets::{decode_image, payload_for_display, store_image};
use crate::encoding::{encode_utf8, read_utf8};
use crate::error::{from_io, AppError, AppResult};
use crate::image_sniff::{sniff_image, MARKDOWN_UTI};
use crate::models::{AssetPayload, OpenedTextBundle, SavedAsset};
use crate::paths::{
    commit_bytes, display_name, ensure_assets_dir, friendly, is_markdown_extension, is_textbundle_name, read_limited,
    resolve_bundle_asset,
};

struct BundleFiles {
    root: PathBuf,
    text_path: PathBuf,
    text_file_name: String,
    version: i64,
}

pub fn open(path: &Path) -> AppResult<OpenedTextBundle> {
    let bundle = inspect(path)?;
    let decoded = read_utf8(&bundle.text_path)?;
    let canonical = friendly(&bundle.root)?;
    let text_path = canonical.join(&bundle.text_file_name);
    let stamp = crate::stamp::stamp_file(&text_path)?;
    Ok(OpenedTextBundle {
        path: canonical.to_string_lossy().into_owned(),
        display_name: display_name(&canonical),
        content: decoded.content,
        has_bom: decoded.has_bom,
        text_file_name: bundle.text_file_name,
        version: bundle.version,
        stamp,
    })
}

pub fn save(
    path: &Path,
    content: &str,
    has_bom: bool,
    text_file_name: &str,
    expected_sha256: Option<&str>,
    overwrite: bool,
) -> AppResult<crate::stamp::FileStamp> {
    let bundle = inspect(path)?;
    if bundle.text_file_name != text_file_name {
        return Err(AppError::msg("TextBundle 正文文件已变化，请重新打开文档"));
    }
    let info_before = fs::read(bundle.root.join("info.json")).map_err(from_io)?;
    let bytes = encode_utf8(content, has_bom);
    let stamp = commit_bytes(&bundle.text_path, &bytes, expected_sha256, overwrite)?;
    let info_after = fs::read(bundle.root.join("info.json")).map_err(from_io)?;
    if info_before != info_after {
        return Err(AppError::msg("保存时意外改变了 info.json"));
    }
    Ok(stamp)
}

pub fn save_asset(bundle_path: &Path, data_base64: &str) -> AppResult<SavedAsset> {
    let bundle = inspect(bundle_path)?;
    let directory = ensure_assets_dir(&bundle.root)?;
    let (bytes, mime) = decode_image(data_base64)?;
    store_image(&directory, &bytes, &mime, "assets")
}

pub fn import_file(bundle_path: &Path, source_path: &Path) -> AppResult<SavedAsset> {
    let bundle = inspect(bundle_path)?;
    let bytes = read_limited(source_path)?;
    let mime = sniff_image(&bytes).ok_or_else(|| AppError::msg("无法根据文件内容识别图片格式"))?;
    let directory = ensure_assets_dir(&bundle.root)?;
    store_image(&directory, &bytes, mime, "assets")
}

pub fn read_asset(bundle_path: &Path, reference: &str) -> AppResult<AssetPayload> {
    let bundle = inspect(bundle_path)?;
    let path = resolve_bundle_asset(&bundle.root, reference)?;
    if !path.is_file() {
        return Err(AppError::asset_missing(reference));
    }
    payload_for_display(&path, reference)
}

pub fn classify(path: &Path) -> AppResult<crate::models::ClassifiedPath> {
    if !path.exists() {
        return Err(AppError::msg("文档路径不存在"));
    }
    if path.is_dir() {
        if !is_textbundle_name(path) {
            return Err(AppError::msg("请选择以 .textbundle 结尾的文档包目录"));
        }
        let canonical = friendly(path)?;
        return Ok(crate::models::ClassifiedPath {
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
        return Ok(crate::models::ClassifiedPath {
            kind: "markdown".to_string(),
            path: canonical.to_string_lossy().into_owned(),
        });
    }
    Err(AppError::msg("不支持的文件类型"))
}

fn inspect(path: &Path) -> AppResult<BundleFiles> {
    if !path.is_dir() {
        return Err(AppError::msg("文档路径不存在或不是 TextBundle 目录"));
    }
    if !is_textbundle_name(path) {
        return Err(AppError::msg("请选择以 .textbundle 结尾的文档包目录"));
    }
    let info_path = path.join("info.json");
    if !info_path.is_file() {
        return Err(AppError::msg("TextBundle 缺少 info.json"));
    }
    let info_bytes = fs::read(&info_path).map_err(from_io)?;
    let info = parse_info(&info_bytes)?;
    let version = info.get("version").and_then(Value::as_i64).ok_or_else(|| AppError::msg("info.json 缺少有效的 version"))?;
    if version != 1 && version != 2 {
        return Err(AppError::msg(format!("不支持的 TextBundle 版本：{version}")));
    }
    match info.get("type") {
        Some(Value::String(kind)) if kind != MARKDOWN_UTI => {
            return Err(AppError::msg(format!(
                "文档类型不受支持：{kind}。第一版只支持 Markdown（{MARKDOWN_UTI}）。"
            )));
        }
        Some(Value::String(_)) | None => {}
        Some(_) => return Err(AppError::msg("info.json 中的 type 无效")),
    }

    let text_files = list_text_files(path)?;
    if text_files.is_empty() {
        return Err(AppError::msg("TextBundle 缺少正文文件（text.*）"));
    }
    if text_files.len() > 1 {
        let names = text_files
            .iter()
            .map(|file| file.file_name().unwrap_or_default().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("、");
        return Err(AppError::msg(format!("TextBundle 存在多个候选正文文件：{names}")));
    }
    let text_path = text_files.into_iter().next().unwrap();
    let text_file_name = text_path.file_name().unwrap().to_string_lossy().into_owned();
    Ok(BundleFiles {
        root: path.to_path_buf(),
        text_path,
        text_file_name,
        version,
    })
}

fn parse_info(bytes: &[u8]) -> AppResult<Value> {
    let json = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    let value: Value = serde_json::from_slice(json).map_err(|_| AppError::msg("info.json 无法解析"))?;
    if !value.is_object() {
        return Err(AppError::msg("info.json 必须是 JSON 对象"));
    }
    Ok(value)
}

fn list_text_files(root: &Path) -> AppResult<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(root).map_err(from_io)? {
        let entry = entry.map_err(from_io)?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let lower = name.to_ascii_lowercase();
        if !lower.starts_with("text.") {
            continue;
        }
        let extension = lower.trim_start_matches("text.");
        if extension.is_empty() || extension.contains('.') {
            continue;
        }
        if entry.path().is_file() {
            files.push(entry.path());
        }
    }
    files.sort();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use std::fs;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\nasset";

    fn write_bundle(root: &Path, info: &str, text_name: &str, text: &str) {
        fs::create_dir_all(root.join("assets")).unwrap();
        fs::write(root.join("info.json"), info).unwrap();
        fs::write(root.join(text_name), text).unwrap();
        fs::write(root.join("assets").join("keep.png"), PNG).unwrap();
    }

    #[test]
    fn opens_v1_without_type_and_saves_without_touching_metadata_or_assets() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("示例.textbundle");
        let info = "{\n  \"version\": 1,\n  \"creatorIdentifier\": \"com.example.keep\",\n  \"com.example.keep\": {\"custom\": true}\n}\n";
        write_bundle(&root, info, "text.md", "原文 ![图](assets/keep.png)\n");
        let opened = open(&root).unwrap();
        assert_eq!(opened.version, 1);
        assert_eq!(opened.text_file_name, "text.md");
        assert!(opened.content.contains("assets/keep.png"));

        let asset = read_asset(Path::new(&opened.path), "assets/keep.png").unwrap();
        assert_eq!(asset.mime_type, "image/png");
        let missing = read_asset(Path::new(&opened.path), "assets/missing.png").unwrap_err();
        assert_eq!(missing.code, "asset_missing");
        let escaped = read_asset(Path::new(&opened.path), "assets/../info.json").unwrap_err();
        assert_ne!(escaped.code, "ok");
        assert!(!escaped.message.contains("creatorIdentifier"));

        let encoded = base64::engine::general_purpose::STANDARD.encode(PNG);
        let saved_asset = save_asset(Path::new(&opened.path), &encoded).unwrap();
        assert!(saved_asset.relative_path.starts_with("assets/image-"));
        let new_markdown = format!("改过 ![图]({})\n", saved_asset.relative_path);
        save(
            Path::new(&opened.path),
            &new_markdown,
            false,
            &opened.text_file_name,
            Some(&opened.stamp.sha256),
            false,
        )
        .unwrap();

        assert_eq!(fs::read_to_string(root.join("info.json")).unwrap(), info);
        assert_eq!(fs::read(root.join("assets").join("keep.png")).unwrap(), PNG);
        assert_eq!(fs::read_to_string(root.join("text.md")).unwrap(), new_markdown);
        assert!(!fs::read_to_string(root.join("text.md")).unwrap().contains("blob:"));
        assert!(!root.join("text.markdown").exists());

        let reopened = open(&root).unwrap();
        assert_eq!(reopened.content, new_markdown);
        read_asset(Path::new(&reopened.path), &saved_asset.relative_path).unwrap();
    }

    #[test]
    fn rejects_unsupported_type_multiple_texts_and_invalid_json_without_rewriting() {
        let dir = tempfile::tempdir().unwrap();
        let bad_type = dir.path().join("bad.textbundle");
        write_bundle(&bad_type, r#"{"version":2,"type":"com.example.fountain"}"#, "text.markdown", "x");
        let before = fs::read(bad_type.join("text.markdown")).unwrap();
        assert!(open(&bad_type).unwrap_err().message.contains("不受支持"));
        assert_eq!(fs::read(bad_type.join("text.markdown")).unwrap(), before);

        let multi = dir.path().join("multi.textbundle");
        write_bundle(&multi, r#"{"version":2,"type":"net.daringfireball.markdown"}"#, "text.md", "a");
        fs::write(multi.join("text.markdown"), "b").unwrap();
        assert!(open(&multi).unwrap_err().message.contains("多个候选正文"));

        let broken = dir.path().join("broken.textbundle");
        write_bundle(&broken, "{", "text.markdown", "a");
        assert!(open(&broken).unwrap_err().message.contains("无法解析"));
        assert_eq!(fs::read_to_string(broken.join("text.markdown")).unwrap(), "a");

        let plain = dir.path().join("not-a-bundle");
        fs::create_dir_all(&plain).unwrap();
        assert!(open(&plain).unwrap_err().message.contains(".textbundle"));
    }

    #[test]
    fn v2_without_type_defaults_to_markdown() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("v2.textbundle");
        write_bundle(&root, r#"{"version":2,"sourceURL":"file:///keep"}"#, "text.markdown", "正文");
        let opened = open(&root).unwrap();
        assert_eq!(opened.version, 2);
        assert_eq!(opened.text_file_name, "text.markdown");
        assert_eq!(opened.content, "正文");
    }

    #[test]
    fn external_edit_blocks_bundle_save() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("edit.textbundle");
        write_bundle(&root, r#"{"version":2}"#, "text.markdown", "原文");
        let opened = open(&root).unwrap();
        fs::write(root.join("text.markdown"), "别人改的").unwrap();
        let error = save(Path::new(&opened.path), "我的修改", false, "text.markdown", Some(&opened.stamp.sha256), false).unwrap_err();
        assert_eq!(error.code, "external_change");
        assert_eq!(fs::read_to_string(root.join("text.markdown")).unwrap(), "别人改的");
    }
}
