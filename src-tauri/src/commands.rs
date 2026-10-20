use std::path::Path;

use crate::error::{AppError, AppResult};
use crate::markdown_doc;
use crate::models::{AssetPayload, ClassifiedPath, OpenedMarkdown, OpenedTextBundle, SavedAsset};
use crate::textbundle;

fn launch_document_args() -> Vec<String> {
    std::env::args()
        .skip(1)
        .filter(|arg| !arg.is_empty() && !arg.starts_with('-'))
        .filter(|arg| !arg.starts_with("http://") && !arg.starts_with("https://"))
        .collect()
}

#[tauri::command]
fn launch_paths() -> Vec<String> {
    launch_document_args()
}

#[tauri::command]
async fn classify_path(path: String) -> Result<ClassifiedPath, AppError> {
    spawn(move || textbundle::classify(Path::new(&path))).await
}

#[tauri::command]
async fn open_markdown(path: String) -> Result<OpenedMarkdown, AppError> {
    spawn(move || markdown_doc::open(Path::new(&path))).await
}

#[tauri::command]
async fn open_textbundle(path: String) -> Result<OpenedTextBundle, AppError> {
    spawn(move || textbundle::open(Path::new(&path))).await
}

#[tauri::command]
async fn save_markdown(
    path: String,
    content: String,
    has_bom: bool,
    expected_sha256: Option<String>,
    overwrite_external: bool,
) -> Result<crate::stamp::FileStamp, AppError> {
    spawn(move || {
        markdown_doc::save(
            Path::new(&path),
            &content,
            has_bom,
            expected_sha256.as_deref(),
            overwrite_external,
        )
    })
    .await
}

#[tauri::command]
async fn save_textbundle(
    path: String,
    content: String,
    has_bom: bool,
    text_file_name: String,
    expected_sha256: Option<String>,
    overwrite_external: bool,
) -> Result<crate::stamp::FileStamp, AppError> {
    spawn(move || {
        textbundle::save(
            Path::new(&path),
            &content,
            has_bom,
            &text_file_name,
            expected_sha256.as_deref(),
            overwrite_external,
        )
    })
    .await
}

#[tauri::command]
async fn save_bundle_asset(bundle_path: String, data_base64: String) -> Result<SavedAsset, AppError> {
    spawn(move || textbundle::save_asset(Path::new(&bundle_path), &data_base64)).await
}

#[tauri::command]
async fn save_markdown_asset(doc_path: String, data_base64: String) -> Result<SavedAsset, AppError> {
    spawn(move || markdown_doc::save_asset(Path::new(&doc_path), &data_base64)).await
}

#[tauri::command]
async fn read_bundle_asset(bundle_path: String, relative_path: String) -> Result<AssetPayload, AppError> {
    spawn(move || textbundle::read_asset(Path::new(&bundle_path), &relative_path)).await
}

#[tauri::command]
async fn read_markdown_asset(doc_path: String, reference: String) -> Result<AssetPayload, AppError> {
    spawn(move || markdown_doc::read_asset(Path::new(&doc_path), &reference)).await
}

#[tauri::command]
async fn import_local_image(document_path: String, document_kind: String, source_path: String) -> Result<SavedAsset, AppError> {
    spawn(move || match document_kind.as_str() {
        "markdown" => markdown_doc::import_file(Path::new(&document_path), Path::new(&source_path)),
        "textbundle" => textbundle::import_file(Path::new(&document_path), Path::new(&source_path)),
        _ => Err(AppError::msg("请先保存文档，再插入图片。")),
    })
    .await
}

async fn spawn<T: Send + 'static>(work: impl FnOnce() -> AppResult<T> + Send + 'static) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|_| AppError::msg("后台任务失败"))?
}

pub fn register<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder.invoke_handler(tauri::generate_handler![
        launch_paths,
        classify_path,
        open_markdown,
        open_textbundle,
        save_markdown,
        save_textbundle,
        save_bundle_asset,
        save_markdown_asset,
        read_bundle_asset,
        read_markdown_asset,
        import_local_image,
    ])
}
