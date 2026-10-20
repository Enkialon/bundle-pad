use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{Cursor, Read, Write};
use std::path::{Component, Path};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use serde_json::Value;
use zip::result::ZipError;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use super::atomic_write::atomic_write;
use super::encoding::{decode_utf8, encode_utf8};
use super::image_sniff::{MAX_ASSET_BYTES, extension_for_mime, sniff_for_display, sniff_image};
use super::paths::{display_name, friendly, normalize_reference, read_limited};
use super::stamp::stamp_bytes;
use crate::app::asset::{AssetPayload, SavedAsset};
use crate::app::error::{AppError, AppResult, from_io};
use crate::app::stamp::FileStamp;
use crate::app::textbundle::OpenedTextBundle;

const MARKDOWN_UTI: &str = "net.daringfireball.markdown";
const MAX_ENTRIES: usize = 4096;
const MAX_PACK_BYTES: u64 = 128 * 1024 * 1024;

struct PackEntry {
    name: String,
    bytes: Vec<u8>,
}

struct Pack {
    entries: Vec<PackEntry>,
    prefix: String,
    info_index: usize,
    text_index: usize,
    text_file_name: String,
    version: i64,
}

/// `.textpack` 是 zip 压缩的 TextBundle。正文和图片都写回原压缩包。
#[derive(Debug, Default, Clone, Copy)]
pub struct FsTextPack;

impl FsTextPack {
    pub fn open(&self, path: &Path) -> AppResult<OpenedTextBundle> {
        let pack = load(path)?;
        let text = &pack.entries[pack.text_index].bytes;
        let decoded = decode_utf8(text)?;
        let canonical = friendly(path)?;
        Ok(OpenedTextBundle {
            path: canonical.to_string_lossy().into_owned(),
            display_name: display_name(&canonical),
            content: decoded.content,
            has_bom: decoded.has_bom,
            text_file_name: pack.text_file_name,
            version: pack.version,
            stamp: stamp_bytes(text, modified_ms(path)?),
        })
    }

    pub fn save(
        &self,
        path: &Path,
        content: &str,
        has_bom: bool,
        text_file_name: &str,
        expected_sha256: Option<&str>,
        overwrite: bool,
    ) -> AppResult<FileStamp> {
        let mut pack = load(path)?;
        if pack.text_file_name != text_file_name {
            return Err(AppError::msg("TextBundle 正文文件已变化，请重新打开文档"));
        }
        let current = stamp_bytes(&pack.entries[pack.text_index].bytes, modified_ms(path)?);
        if let Some(expected) = expected_sha256 {
            if current.sha256 != expected && !overwrite {
                return Err(AppError::external_change());
            }
        }
        let info_before = pack.entries[pack.info_index].bytes.clone();
        pack.entries[pack.text_index].bytes = encode_utf8(content, has_bom);
        if pack.entries[pack.info_index].bytes != info_before {
            return Err(AppError::msg("保存时意外改变了 info.json"));
        }
        let text = pack.entries[pack.text_index].bytes.clone();
        store(path, &pack.entries)?;
        Ok(stamp_bytes(&text, modified_ms(path)?))
    }

    pub fn read_asset(&self, path: &Path, reference: &str) -> AppResult<AssetPayload> {
        let pack = load(path)?;
        let relative = asset_relative(reference)?;
        let full = format!("{}{relative}", pack.prefix);
        let Some(index) = entry_index(&pack.entries, &full) else {
            return Err(AppError::asset_missing(reference));
        };
        payload(&pack.entries[index].bytes, reference)
    }

    pub fn write_asset(&self, path: &Path, data_base64: &str) -> AppResult<SavedAsset> {
        let (bytes, mime) = decode_pasted_image(data_base64)?;
        insert_image(path, bytes, &mime)
    }

    pub fn import_local_image(&self, path: &Path, source_path: &Path) -> AppResult<SavedAsset> {
        let bytes = read_limited(source_path)?;
        let mime =
            sniff_image(&bytes).ok_or_else(|| AppError::msg("无法根据文件内容识别图片格式"))?;
        insert_image(path, bytes, mime)
    }
}

fn decode_pasted_image(data_base64: &str) -> AppResult<(Vec<u8>, String)> {
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

fn insert_image(path: &Path, bytes: Vec<u8>, mime: &str) -> AppResult<SavedAsset> {
    let extension =
        extension_for_mime(mime).ok_or_else(|| AppError::msg("无法根据文件内容识别图片格式"))?;
    let mut pack = load(path)?;
    let name = unique_asset_name(&pack, extension)?;
    let relative = format!("assets/{name}");
    pack.entries.push(PackEntry {
        name: format!("{}{relative}", pack.prefix),
        bytes,
    });
    store(path, &pack.entries)?;
    Ok(SavedAsset {
        relative_path: relative,
        mime_type: mime.to_string(),
    })
}

fn load(path: &Path) -> AppResult<Pack> {
    if !path.is_file() {
        return Err(AppError::msg("文档路径不存在"));
    }
    let file = File::open(path).map_err(from_io)?;
    let mut archive = ZipArchive::new(file).map_err(zip_err)?;
    if archive.len() > MAX_ENTRIES {
        return Err(AppError::msg("TextPack 文件过多"));
    }

    let mut entries = Vec::new();
    let mut seen = HashSet::new();
    let mut total = 0_u64;
    for index in 0..archive.len() {
        let mut zip_file = archive.by_index(index).map_err(zip_err)?;
        if zip_file.encrypted() {
            return Err(AppError::msg("TextPack 已加密，无法打开"));
        }
        let raw = zip_file
            .name()
            .map_err(|_| AppError::msg("TextPack 含有无法识别的文件名"))?
            .into_owned();
        if raw_name_is_unsafe(&raw) {
            return Err(AppError::msg("TextPack 含有不安全的路径"));
        }
        if zip_file.is_symlink() {
            return Err(AppError::msg("TextPack 含有符号链接，无法打开"));
        }
        if zip_file.is_dir() {
            continue;
        }
        let enclosed = zip_file
            .enclosed_name()
            .ok_or_else(|| AppError::msg("TextPack 含有不安全的路径"))?;
        let name = zip_rel_name(&enclosed)?;
        if !seen.insert(name.to_ascii_lowercase()) {
            return Err(AppError::msg("TextPack 含有重复文件"));
        }
        if zip_file.size() > MAX_PACK_BYTES
            || total.saturating_add(zip_file.size()) > MAX_PACK_BYTES
        {
            return Err(AppError::msg("TextPack 过大"));
        }
        let budget = MAX_PACK_BYTES - total;
        let mut bytes = Vec::new();
        (&mut zip_file)
            .take(budget + 1)
            .read_to_end(&mut bytes)
            .map_err(from_io)?;
        if bytes.len() as u64 > budget {
            return Err(AppError::msg("TextPack 过大"));
        }
        total += bytes.len() as u64;
        entries.push(PackEntry { name, bytes });
    }
    locate(entries)
}

fn locate(entries: Vec<PackEntry>) -> AppResult<Pack> {
    let infos: Vec<(usize, String)> = entries
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| info_parent(&entry.name).map(|parent| (index, parent)))
        .collect();
    if infos.is_empty() {
        return Err(AppError::msg("TextPack 缺少 info.json"));
    }
    let roots: Vec<_> = infos
        .iter()
        .filter(|(_, parent)| parent.is_empty())
        .cloned()
        .collect();
    let (info_index, parent) = if !roots.is_empty() {
        if roots.len() != 1 {
            return Err(AppError::msg("TextPack 里有多份 info.json"));
        }
        roots[0].clone()
    } else if infos.len() == 1 {
        infos[0].clone()
    } else {
        return Err(AppError::msg("TextPack 里有多份 info.json"));
    };
    let prefix = if parent.is_empty() {
        String::new()
    } else {
        format!("{parent}/")
    };

    let info = parse_info(&entries[info_index].bytes)?;
    let version = info
        .get("version")
        .and_then(Value::as_i64)
        .ok_or_else(|| AppError::msg("info.json 缺少有效的 version"))?;
    if version != 1 && version != 2 {
        return Err(AppError::msg(format!(
            "不支持的 TextBundle 版本：{version}"
        )));
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

    let mut texts = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let Some(rest) = entry.name.strip_prefix(&prefix) else {
            continue;
        };
        if rest.contains('/') {
            continue;
        }
        let lower = rest.to_ascii_lowercase();
        let Some(extension) = lower.strip_prefix("text.") else {
            continue;
        };
        if extension.is_empty() || extension.contains('.') {
            continue;
        }
        texts.push((index, rest.to_string()));
    }
    if texts.is_empty() {
        return Err(AppError::msg("TextPack 缺少正文文件（text.*）"));
    }
    if texts.len() > 1 {
        let names = texts
            .iter()
            .map(|(_, name)| name.as_str())
            .collect::<Vec<_>>()
            .join("、");
        return Err(AppError::msg(format!(
            "TextBundle 存在多个候选正文文件：{names}"
        )));
    }
    let (text_index, text_file_name) = texts.remove(0);
    Ok(Pack {
        entries,
        prefix,
        info_index,
        text_index,
        text_file_name,
        version,
    })
}

fn store(path: &Path, entries: &[PackEntry]) -> AppResult<()> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for entry in entries {
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        writer.start_file(&entry.name, options).map_err(zip_err)?;
        writer.write_all(&entry.bytes).map_err(from_io)?;
    }
    let cursor = writer.finish().map_err(zip_err)?;
    atomic_write(path, &cursor.into_inner()).map_err(from_io)
}

fn info_parent(name: &str) -> Option<String> {
    let (parent, file) = match name.rsplit_once('/') {
        Some((parent, file)) => (parent, file),
        None => ("", name),
    };
    if parent.contains('/') || !file.eq_ignore_ascii_case("info.json") {
        None
    } else {
        Some(parent.to_string())
    }
}

fn parse_info(bytes: &[u8]) -> AppResult<Value> {
    let json = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    let value: Value =
        serde_json::from_slice(json).map_err(|_| AppError::msg("info.json 无法解析"))?;
    if !value.is_object() {
        return Err(AppError::msg("info.json 必须是 JSON 对象"));
    }
    Ok(value)
}

fn asset_relative(reference: &str) -> AppResult<String> {
    let reference = normalize_reference(reference)?;
    let relative = Path::new(&reference);
    if relative.is_absolute() {
        return Err(AppError::msg("不允许使用绝对路径读取文档包资源"));
    }
    let mut parts = Vec::new();
    for component in relative.components() {
        match component {
            Component::Normal(name) => {
                let text = name.to_string_lossy();
                if text.is_empty() || text == "." || text == ".." || text.contains(':') {
                    return Err(AppError::msg("资源路径无效"));
                }
                parts.push(text.into_owned());
            }
            _ => return Err(AppError::msg("资源路径不能越过文档包")),
        }
    }
    match parts.first() {
        Some(name) if name.eq_ignore_ascii_case("assets") && parts.len() >= 2 => {}
        Some(_) => return Err(AppError::msg("资源路径必须位于 assets/ 目录内")),
        None => return Err(AppError::msg("资源路径无效")),
    }
    Ok(parts.join("/"))
}

fn entry_index(entries: &[PackEntry], name: &str) -> Option<usize> {
    if let Some(index) = entries.iter().position(|entry| entry.name == name) {
        return Some(index);
    }
    let mut found = None;
    for (index, entry) in entries.iter().enumerate() {
        if entry.name.eq_ignore_ascii_case(name) {
            if found.is_some() {
                return None;
            }
            found = Some(index);
        }
    }
    found
}

fn unique_asset_name(pack: &Pack, extension: &str) -> AppResult<String> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(1);
    let assets_prefix = format!("{}assets/", pack.prefix);
    for _ in 0..16 {
        let seq = SEQ.fetch_add(1, Ordering::Relaxed);
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let name = format!("image-{millis}-{seq}.{extension}");
        if entry_index(&pack.entries, &format!("{assets_prefix}{name}")).is_none() {
            return Ok(name);
        }
    }
    Err(AppError::msg("无法生成不冲突的附件文件名"))
}

fn payload(bytes: &[u8], reference: &str) -> AppResult<AssetPayload> {
    if bytes.is_empty() {
        return Err(AppError::msg("图片文件是空的"));
    }
    if bytes.len() as u64 > MAX_ASSET_BYTES {
        return Err(AppError::msg("图片文件过大"));
    }
    let mime =
        sniff_for_display(bytes, reference).ok_or_else(|| AppError::msg("文件不是受支持的图片"))?;
    Ok(AssetPayload {
        mime_type: mime,
        data_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
    })
}

fn modified_ms(path: &Path) -> AppResult<u64> {
    let modified = fs::metadata(path)
        .map_err(from_io)?
        .modified()
        .unwrap_or(SystemTime::UNIX_EPOCH);
    Ok(modified
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0))
}

fn raw_name_is_unsafe(raw: &str) -> bool {
    if raw.contains('\0') || raw.starts_with('/') || raw.starts_with('\\') {
        return true;
    }
    raw.replace('\\', "/")
        .split('/')
        .any(|part| part == ".." || part.contains(':'))
}

fn zip_rel_name(path: &Path) -> AppResult<String> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => {
                let text = part.to_string_lossy();
                if text.is_empty()
                    || text == "."
                    || text == ".."
                    || text.contains(':')
                    || text.contains('\0')
                {
                    return Err(AppError::msg("TextPack 含有不安全的路径"));
                }
                parts.push(text.into_owned());
            }
            Component::CurDir => {}
            _ => return Err(AppError::msg("TextPack 含有不安全的路径")),
        }
    }
    if parts.is_empty() {
        return Err(AppError::msg("TextPack 含有不安全的路径"));
    }
    Ok(parts.join("/"))
}

fn zip_err(err: ZipError) -> AppError {
    match err {
        ZipError::Io(err) => from_io(err),
        ZipError::InvalidPassword => AppError::msg("TextPack 已加密，无法打开"),
        ZipError::UnsupportedArchive(_) | ZipError::CompressionMethodNotSupported(_) => {
            AppError::msg("TextPack 使用了不支持的压缩方式")
        }
        ZipError::InvalidArchive(_) | ZipError::FileNotFound => {
            AppError::msg("TextPack 不是有效的 zip 压缩包")
        }
        _ => AppError::msg("TextPack 无法读取"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::document::DocumentStore;
    use crate::infrastructure::document::FsDocument;
    use base64::Engine;
    use std::io::Write;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\nasset";
    const INFO: &str = "{\n  \"version\": 2,\n  \"creatorIdentifier\": \"com.example.keep\"\n}\n";

    fn write_pack(path: &Path, files: &[(&str, &[u8])]) {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        for (name, bytes) in files {
            let options =
                SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
            writer.start_file(*name, options).unwrap();
            writer.write_all(bytes).unwrap();
        }
        let cursor = writer.finish().unwrap();
        fs::write(path, cursor.into_inner()).unwrap();
    }

    fn read_zip_entry(path: &Path, name: &str) -> Vec<u8> {
        let file = File::open(path).unwrap();
        let mut archive = ZipArchive::new(file).unwrap();
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).unwrap();
            if entry.name().unwrap() == name {
                let mut bytes = Vec::new();
                entry.read_to_end(&mut bytes).unwrap();
                return bytes;
            }
        }
        panic!("缺少 {name}");
    }

    #[test]
    fn opens_flat_and_wrapped_example_textpack() {
        let dir = tempfile::tempdir().unwrap();
        let flat = dir.path().join("example.textpack");
        write_pack(
            &flat,
            &[
                ("info.json", INFO.as_bytes()),
                ("text.md", "正文 ![图](assets/keep.png)\n".as_bytes()),
                ("assets/keep.png", PNG),
            ],
        );
        let opened = FsDocument.open(&flat).unwrap();
        assert_eq!(opened.kind, "textbundle");
        assert!(opened.path.ends_with("example.textpack"));
        assert_eq!(opened.display_name, "example.textpack");
        assert_eq!(opened.text_file_name.as_deref(), Some("text.md"));
        assert_eq!(opened.version, Some(2));
        assert!(opened.content.contains("assets/keep.png"));
        let asset = FsDocument
            .read_asset(Path::new(&opened.path), "assets/keep.png")
            .unwrap();
        assert_eq!(asset.mime_type, "image/png");
        let escaped = FsDocument
            .read_asset(Path::new(&opened.path), "assets/../info.json")
            .unwrap_err();
        assert_ne!(escaped.code, "ok");
        assert!(!escaped.message.contains("creatorIdentifier"));

        let wrapped = dir.path().join("wrapped.textpack");
        write_pack(
            &wrapped,
            &[
                ("示例.textbundle/info.json", b"{\"version\":1}"),
                ("示例.textbundle/text.markdown", "包内正文".as_bytes()),
                ("__MACOSX/示例.textbundle/._text.markdown", b"junk"),
            ],
        );
        let bundle = FsDocument.open(&wrapped).unwrap();
        assert_eq!(bundle.content, "包内正文");
        assert_eq!(bundle.text_file_name.as_deref(), Some("text.markdown"));
        assert_eq!(bundle.version, Some(1));
    }

    #[test]
    fn save_and_assets_keep_info_json() {
        let dir = tempfile::tempdir().unwrap();
        let pack = dir.path().join("example.textpack");
        write_pack(
            &pack,
            &[
                ("info.json", INFO.as_bytes()),
                ("text.md", b"old"),
                ("assets/keep.png", PNG),
            ],
        );
        let opened = FsDocument.open(&pack).unwrap();
        let changed = FsTextPack
            .save(
                Path::new(&opened.path),
                "new",
                false,
                "nope.md",
                Some(&opened.stamp.sha256),
                false,
            )
            .unwrap_err();
        assert!(changed.message.contains("正文文件"));

        let stale = FsTextPack
            .save(
                Path::new(&opened.path),
                "new",
                false,
                "text.md",
                Some("deadbeef"),
                false,
            )
            .unwrap_err();
        assert_eq!(stale.code, "external_change");

        FsTextPack
            .save(
                Path::new(&opened.path),
                "新正文\n",
                false,
                "text.md",
                Some(&opened.stamp.sha256),
                false,
            )
            .unwrap();
        assert_eq!(read_zip_entry(&pack, "info.json"), INFO.as_bytes());
        assert_eq!(read_zip_entry(&pack, "assets/keep.png"), PNG);

        let encoded = base64::engine::general_purpose::STANDARD.encode(PNG);
        let saved = FsDocument.write_asset(&pack, &encoded).unwrap();
        assert!(saved.relative_path.starts_with("assets/image-"));
        let again = FsDocument.open(&pack).unwrap();
        assert_eq!(again.content, "新正文\n");
        let image = FsDocument.read_asset(&pack, &saved.relative_path).unwrap();
        assert_eq!(image.mime_type, "image/png");
    }

    #[test]
    fn rejects_slip_and_non_zip() {
        let dir = tempfile::tempdir().unwrap();
        let slipped = dir.path().join("example.textpack");
        write_pack(
            &slipped,
            &[
                ("../secret.txt", b"nope"),
                ("info.json", INFO.as_bytes()),
                ("text.md", b"x"),
            ],
        );
        let err = FsDocument.open(&slipped).unwrap_err();
        assert!(err.message.contains("不安全"));
        assert!(!err.message.contains("暂不支持"));

        let junk = dir.path().join("plain.textpack");
        fs::write(&junk, b"this is not a zip").unwrap();
        let err = FsDocument.open(&junk).unwrap_err();
        assert!(err.message.contains("zip"));
    }
}
