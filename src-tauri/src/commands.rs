use std::path::Path;

use crate::app::asset::{AssetPayload, SavedAsset};
use crate::app::document::{DocumentStore, OpenedDocument};
use crate::app::error::{AppError, AppResult};
use crate::app::stamp::FileStamp;
use crate::infrastructure::FsDocument;
use crate::print_pdf;

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
async fn open_document(path: String) -> Result<OpenedDocument, AppError> {
    spawn(move || FsDocument.open(Path::new(&path))).await
}

#[tauri::command]
async fn save_document(
    path: String,
    content: String,
    has_bom: bool,
    text_file_name: Option<String>,
    expected_sha256: Option<String>,
    overwrite_external: bool,
) -> Result<FileStamp, AppError> {
    spawn(move || {
        FsDocument.save(
            Path::new(&path),
            &content,
            has_bom,
            text_file_name.as_deref(),
            expected_sha256.as_deref(),
            overwrite_external,
        )
    })
    .await
}

#[tauri::command]
async fn read_asset(document_path: String, reference: String) -> Result<AssetPayload, AppError> {
    spawn(move || FsDocument.read_asset(Path::new(&document_path), &reference)).await
}

#[tauri::command]
async fn write_asset(document_path: String, data_base64: String) -> Result<SavedAsset, AppError> {
    spawn(move || FsDocument.write_asset(Path::new(&document_path), &data_base64)).await
}

#[tauri::command]
async fn import_local_image(document_path: String, source_path: String) -> Result<SavedAsset, AppError> {
    spawn(move || FsDocument.import_local_image(Path::new(&document_path), Path::new(&source_path))).await
}

#[tauri::command]
async fn export_pdf<R: tauri::Runtime>(app: tauri::AppHandle<R>, path: String) -> Result<(), AppError> {
    use tauri::Manager;

    let window = app
        .get_webview_window("main")
        .ok_or_else(|| AppError::msg("找不到主窗口"))?;
    tauri::async_runtime::spawn_blocking(move || print_pdf::export_pdf(window, path))
        .await
        .map_err(|_| AppError::msg("后台任务失败"))?
}

async fn spawn<T: Send + 'static>(work: impl FnOnce() -> AppResult<T> + Send + 'static) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(work).await.map_err(|_| AppError::msg("后台任务失败"))?
}

pub fn register<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder.invoke_handler(tauri::generate_handler![
        launch_paths,
        open_document,
        save_document,
        read_asset,
        write_asset,
        import_local_image,
        export_pdf,
    ])
}
