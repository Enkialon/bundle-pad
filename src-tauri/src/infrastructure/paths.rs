use std::fs;
use std::path::{Component, Path, PathBuf};

use super::atomic_write::atomic_write;
use super::image_sniff::MAX_ASSET_BYTES;
use super::stamp::stamp_file;
use crate::app::error::{from_io, AppError, AppResult};
use crate::app::stamp::FileStamp;

pub fn friendly(path: &Path) -> AppResult<PathBuf> {
    dunce::canonicalize(path).map_err(from_io)
}

pub fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "未命名".to_string())
}

pub fn is_markdown_extension(path: &Path) -> bool {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) => ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown"),
        None => false,
    }
}

pub fn is_textbundle_name(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.to_ascii_lowercase().ends_with(".textbundle"))
}

pub fn normalize_reference(reference: &str) -> AppResult<String> {
    if reference.is_empty() || reference.contains('\0') || reference.contains('\n') || reference.contains('\r') {
        return Err(AppError::msg("图片路径无效"));
    }
    Ok(reference.replace('\\', "/"))
}

pub fn is_remote_reference(reference: &str) -> bool {
    let lower = reference.to_ascii_lowercase();
    lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("mailto:")
        || lower.starts_with("blob:")
        || lower.starts_with("data:")
}

/// TextBundle 资源必须位于包内 assets/，拒绝绝对路径、盘符和 `..`。
pub fn resolve_bundle_asset(bundle: &Path, reference: &str) -> AppResult<PathBuf> {
    let reference = normalize_reference(reference)?;
    let relative = Path::new(&reference);
    if relative.is_absolute() {
        return Err(AppError::msg("不允许使用绝对路径读取文档包资源"));
    }

    let mut components = relative.components();
    match components.next() {
        Some(Component::Normal(name)) if name.to_string_lossy().eq_ignore_ascii_case("assets") => {}
        _ => return Err(AppError::msg("资源路径必须位于 assets/ 目录内")),
    }

    let mut depth = 0_usize;
    for component in components {
        match component {
            Component::Normal(name) => {
                let text = name.to_string_lossy();
                if text.is_empty() || text == "." || text == ".." || text.contains(':') {
                    return Err(AppError::msg("资源路径无效"));
                }
                depth += 1;
            }
            _ => return Err(AppError::msg("资源路径不能越过文档包")),
        }
    }
    if depth == 0 {
        return Err(AppError::msg("资源路径无效"));
    }

    let candidate = bundle.join(Path::new(&reference));
    ensure_inside_bundle(bundle, &candidate)?;
    Ok(candidate)
}

pub fn ensure_assets_dir(bundle: &Path) -> AppResult<PathBuf> {
    let assets = bundle.join("assets");
    if !assets.exists() {
        fs::create_dir(&assets).map_err(from_io)?;
    }
    if !assets.is_dir() {
        return Err(AppError::msg("assets 不是目录"));
    }
    let bundle_canon = dunce::canonicalize(bundle).map_err(from_io)?;
    let assets_canon = dunce::canonicalize(&assets).map_err(from_io)?;
    if !is_strict_child(&bundle_canon, &assets_canon) {
        return Err(AppError::msg("assets 目录不能指向文档包之外"));
    }
    Ok(assets_canon)
}

pub fn markdown_asset_dir(doc_path: &Path) -> AppResult<(PathBuf, String)> {
    if !is_markdown_extension(doc_path) {
        return Err(AppError::msg("只能为 Markdown 文档保存图片"));
    }
    let parent = doc_path.parent().filter(|path| path.is_dir()).ok_or_else(|| AppError::msg("目标文件夹不存在"))?;
    let stem = doc_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty() && !stem.contains(['/', '\\', ':']))
        .ok_or_else(|| AppError::msg("文档名称无效"))?;
    let dir_name = format!("{stem}.assets");
    let dir = parent.join(&dir_name);
    if !dir.exists() {
        fs::create_dir(&dir).map_err(from_io)?;
    }
    if !dir.is_dir() {
        return Err(AppError::msg("图片目录无效"));
    }
    let parent_canon = dunce::canonicalize(parent).map_err(from_io)?;
    let dir_canon = dunce::canonicalize(&dir).map_err(from_io)?;
    if !is_strict_child(&parent_canon, &dir_canon) {
        return Err(AppError::msg("图片目录不能指向文档所在文件夹之外"));
    }
    Ok((dir_canon, dir_name))
}

pub fn resolve_markdown_image(doc_path: &Path, reference: &str) -> AppResult<PathBuf> {
    let reference = normalize_reference(reference)?;
    if is_remote_reference(&reference) {
        return Err(AppError::msg("远程图片不会下载到本地"));
    }
    let relative = Path::new(&reference);
    if !relative.is_absolute() {
        for component in relative.components() {
            match component {
                Component::Normal(name) => {
                    let text = name.to_string_lossy();
                    if text.contains(':') {
                        return Err(AppError::msg("图片路径无效"));
                    }
                }
                Component::CurDir | Component::ParentDir => {}
                _ => return Err(AppError::msg("图片路径无效")),
            }
        }
    }
    let parent = doc_path.parent().ok_or_else(|| AppError::msg("文档路径无效"))?;
    let full = if relative.is_absolute() { relative.to_path_buf() } else { parent.join(relative) };
    if !full.exists() {
        return Err(AppError::asset_missing(&reference));
    }
    let canon = dunce::canonicalize(&full).map_err(|_| AppError::asset_missing(&reference))?;
    if !canon.is_file() {
        return Err(AppError::asset_missing(&reference));
    }
    Ok(canon)
}

pub fn read_limited(path: &Path) -> AppResult<Vec<u8>> {
    let metadata = fs::metadata(path).map_err(from_io)?;
    if !metadata.is_file() {
        return Err(AppError::msg("图片路径不是文件"));
    }
    if metadata.len() > MAX_ASSET_BYTES {
        return Err(AppError::msg("图片文件过大"));
    }
    let bytes = fs::read(path).map_err(from_io)?;
    if bytes.len() as u64 > MAX_ASSET_BYTES {
        return Err(AppError::msg("图片文件过大"));
    }
    if bytes.is_empty() {
        return Err(AppError::msg("图片文件是空的"));
    }
    Ok(bytes)
}

pub fn staging_images_dir() -> AppResult<PathBuf> {
    let root = app_data_dir()?.join("BundlePad").join("images");
    fs::create_dir_all(&root).map_err(|_| AppError::msg("无法创建临时附件目录"))?;
    dunce::canonicalize(&root).map_err(from_io)
}

pub fn resolve_staged_file(directory: &Path, reference: &str) -> AppResult<PathBuf> {
    let reference = normalize_reference(reference)?;
    let candidate = PathBuf::from(&reference);
    if !candidate.is_absolute() {
        return Err(AppError::asset_missing(&reference));
    }
    let root = dunce::canonicalize(directory).map_err(from_io)?;
    let file = dunce::canonicalize(&candidate).map_err(|_| AppError::asset_missing(&reference))?;
    if !file.is_file() || !is_strict_child(&root, &file) {
        return Err(AppError::msg("附件路径无效"));
    }
    Ok(file)
}

pub fn write_unique_image(directory: &Path, bytes: &[u8], extension: &str) -> AppResult<String> {
    let name = unique_image_name(directory, extension)?;
    let target = directory.join(&name);
    if target.parent() != Some(directory) || name.contains(['/', '\\']) {
        return Err(AppError::msg("图片文件名无效"));
    }
    atomic_write(&target, bytes).map_err(from_io)?;
    Ok(name)
}

pub fn commit_bytes(path: &Path, bytes: &[u8], expected_sha256: Option<&str>, overwrite: bool) -> AppResult<FileStamp> {
    if let Some(expected) = expected_sha256 {
        if !path.is_file() {
            return Err(AppError::msg("文档路径不存在"));
        }
        let current = stamp_file(path)?;
        if current.sha256 != expected && !overwrite {
            return Err(AppError::external_change());
        }
    } else if let Some(parent) = path.parent() {
        if !parent.is_dir() {
            return Err(AppError::msg("目标文件夹不存在"));
        }
    }
    atomic_write(path, bytes).map_err(from_io)?;
    stamp_file(path)
}

fn app_data_dir() -> AppResult<PathBuf> {
    #[cfg(windows)]
    let dir = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(target_os = "macos")]
    let dir = std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Library").join("Application Support"));
    #[cfg(all(unix, not(target_os = "macos")))]
    let dir = std::env::var_os("XDG_DATA_HOME").filter(|value| !value.is_empty()).map(PathBuf::from).or_else(|| {
        std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local").join("share"))
    });
    dir.filter(|path| !path.as_os_str().is_empty()).ok_or_else(|| AppError::msg("找不到用户数据目录"))
}

fn ensure_inside_bundle(bundle: &Path, candidate: &Path) -> AppResult<()> {
    if !candidate.exists() {
        return Ok(());
    }
    let bundle_canon = dunce::canonicalize(bundle).map_err(from_io)?;
    let file_canon = dunce::canonicalize(candidate).map_err(from_io)?;
    if !is_strict_child(&bundle_canon, &file_canon) {
        return Err(AppError::msg("资源路径不能越过文档包"));
    }
    Ok(())
}

fn is_strict_child(root: &Path, child: &Path) -> bool {
    let root_key = compare_key(root);
    let child_key = compare_key(child);
    child_key.starts_with(&root_key) && child_key != root_key
}

fn compare_key(path: &Path) -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(path.to_string_lossy().to_lowercase())
    } else {
        path.to_path_buf()
    }
}

fn unique_image_name(directory: &Path, extension: &str) -> AppResult<String> {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static SEQ: AtomicU64 = AtomicU64::new(1);
    for _ in 0..16 {
        let seq = SEQ.fetch_add(1, Ordering::Relaxed);
        let millis = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
        let name = format!("image-{millis}-{seq}.{extension}");
        if !directory.join(&name).exists() {
            return Ok(name);
        }
    }
    Err(AppError::msg("无法生成不冲突的图片文件名"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn bundle() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("demo.textbundle");
        fs::create_dir_all(root.join("assets")).unwrap();
        fs::write(root.join("assets").join("keep.png"), b"png").unwrap();
        fs::write(dir.path().join("secret.txt"), b"secret-marker").unwrap();
        dir
    }

    #[test]
    fn rejects_bundle_escape_paths() {
        let dir = bundle();
        let root = dir.path().join("demo.textbundle");
        for reference in [
            "../secret.txt",
            "assets/../../secret.txt",
            "assets/../secret.txt",
            "assets\\..\\secret.txt",
            "/assets/keep.png",
            "C:/secret.txt",
            "assets/../../secret.txt",
        ] {
            let error = resolve_bundle_asset(&root, reference).unwrap_err();
            assert!(!error.message.contains("secret-marker"), "{reference}");
            assert_eq!(fs::read_to_string(dir.path().join("secret.txt")).unwrap(), "secret-marker");
        }
    }

    #[test]
    fn accepts_asset_inside_bundle() {
        let dir = bundle();
        let root = dir.path().join("demo.textbundle");
        let resolved = resolve_bundle_asset(&root, "assets/keep.png").unwrap();
        assert!(resolved.is_file());
        assert!(resolved.ends_with("keep.png"));
    }
}
