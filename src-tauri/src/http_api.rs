//! 浏览器适配使用的本地 HTTP。文档读写与 Tauri 命令调用同一套 [`FsDocument`]。

use std::path::Path;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::app::document::DocumentStore;
use crate::app::error::{AppError, AppResult};
use crate::infrastructure::FsDocument;

pub fn dispatch(method: &str, path: &str, body: &str) -> (u16, String) {
    if method.eq_ignore_ascii_case("OPTIONS") {
        return (204, String::new());
    }
    if method != "POST" {
        return error_body(405, "error", "只接受 POST");
    }
    match path {
        "/api/launch-paths" => json_ok(&Vec::<String>::new()),
        "/api/open-document" => {
            with_json(body, |req: OpenBody| FsDocument.open(Path::new(&req.path)))
        }
        "/api/save-document" => with_json(body, |req: SaveBody| {
            FsDocument.save(
                Path::new(&req.path),
                &req.content,
                req.has_bom,
                req.text_file_name.as_deref(),
                req.expected_sha256.as_deref(),
                req.overwrite_external,
            )
        }),
        "/api/read-asset" => with_json(body, |req: ReadAssetBody| {
            FsDocument.read_asset(Path::new(&req.document_path), &req.reference)
        }),
        "/api/write-asset" => with_json(body, |req: WriteAssetBody| {
            FsDocument.write_asset(Path::new(&req.document_path), &req.data_base64)
        }),
        "/api/import-local-image" => with_json(body, |req: ImportBody| {
            FsDocument
                .import_local_image(Path::new(&req.document_path), Path::new(&req.source_path))
        }),
        _ => dispatch_dialog(path, body),
    }
}

#[cfg(feature = "http")]
pub fn serve(addr: &str) {
    let server = tiny_http::Server::http(addr).unwrap_or_else(|err| {
        panic!("文档服务监听失败 {addr}: {err}");
    });
    eprintln!("BundlePad 文档服务已监听 http://{addr}");
    for mut request in server.incoming_requests() {
        let method = request.method().as_str().to_string();
        let url = request.url().to_string();
        let path = url.split('?').next().unwrap_or(url.as_str());
        let mut body = String::new();
        let mut reader = request.as_reader();
        let _ = std::io::Read::read_to_string(&mut reader, &mut body);
        let (status, payload) = dispatch(&method, path, &body);
        let response = tiny_http::Response::from_string(payload)
            .with_status_code(status)
            .with_header(header("Content-Type", "application/json; charset=utf-8"))
            .with_header(header(
                "Access-Control-Allow-Origin",
                "http://localhost:1420",
            ))
            .with_header(header("Access-Control-Allow-Methods", "POST, OPTIONS"))
            .with_header(header("Access-Control-Allow-Headers", "content-type"));
        let _ = request.respond(response);
    }
}

#[cfg(feature = "http")]
fn header(name: &str, value: &str) -> tiny_http::Header {
    tiny_http::Header::from_bytes(name.as_bytes(), value.as_bytes()).expect("响应头")
}

#[cfg(feature = "http")]
fn dispatch_dialog(path: &str, body: &str) -> (u16, String) {
    match path {
        "/api/show-error" => show_message(body, rfd::MessageLevel::Error),
        "/api/show-warning" => show_message(body, rfd::MessageLevel::Warning),
        "/api/confirm-unsaved" => json_text(confirm_unsaved()),
        "/api/confirm-external-change" => json_text(confirm_external()),
        "/api/pick-markdown" => pick_path(body, pick_markdown),
        "/api/pick-textbundle" => pick_path(body, pick_textbundle),
        "/api/pick-save-markdown" => pick_path(body, pick_save_markdown),
        _ => error_body(404, "error", "未知接口"),
    }
}

#[cfg(not(feature = "http"))]
fn dispatch_dialog(_path: &str, _body: &str) -> (u16, String) {
    error_body(404, "error", "未知接口")
}

#[cfg(feature = "http")]
fn show_message(body: &str, level: rfd::MessageLevel) -> (u16, String) {
    let text = match parse::<TextBody>(body) {
        Ok(value) => value.text,
        Err(reply) => return reply,
    };
    let _ = rfd::MessageDialog::new()
        .set_title("BundlePad")
        .set_description(&text)
        .set_level(level)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
    json_ok(&serde_json::json!({}))
}

#[cfg(feature = "http")]
fn confirm_unsaved() -> &'static str {
    match rfd::MessageDialog::new()
        .set_title("BundlePad")
        .set_description("当前文档有未保存的修改。")
        .set_level(rfd::MessageLevel::Warning)
        .set_buttons(rfd::MessageButtons::YesNoCancel)
        .show()
    {
        rfd::MessageDialogResult::Yes => "save",
        rfd::MessageDialogResult::No => "discard",
        _ => "cancel",
    }
}

#[cfg(feature = "http")]
fn confirm_external() -> &'static str {
    match rfd::MessageDialog::new()
        .set_title("BundlePad")
        .set_description("这个文档已被其他程序修改。重新加载会丢弃当前未保存的修改；覆盖保存会用当前内容替换磁盘上的文件。")
        .set_level(rfd::MessageLevel::Warning)
        .set_buttons(rfd::MessageButtons::YesNoCancel)
        .show()
    {
        rfd::MessageDialogResult::Yes => "reload",
        rfd::MessageDialogResult::No => "overwrite",
        _ => "cancel",
    }
}

#[cfg(feature = "http")]
fn pick_path(body: &str, pick: fn(Option<String>) -> Option<String>) -> (u16, String) {
    let default_path = match parse::<PickBody>(body) {
        Ok(value) => value.default_path.filter(|item| !item.is_empty()),
        Err(reply) => return reply,
    };
    json_ok(&PathBody {
        path: pick(default_path),
    })
}

#[cfg(feature = "http")]
fn pick_markdown(default_path: Option<String>) -> Option<String> {
    let dialog = with_directory(
        rfd::FileDialog::new()
            .set_title("打开文档")
            .add_filter("Markdown 与 TextPack", &["md", "markdown", "textpack"]),
        default_path,
    );
    dialog.pick_file().map(path_to_string)
}

#[cfg(feature = "http")]
fn pick_textbundle(default_path: Option<String>) -> Option<String> {
    let dialog = with_directory(
        rfd::FileDialog::new().set_title("打开 TextBundle 文档包"),
        default_path,
    );
    dialog.pick_folder().map(path_to_string)
}

#[cfg(feature = "http")]
fn pick_save_markdown(default_path: Option<String>) -> Option<String> {
    let dialog = with_directory(
        rfd::FileDialog::new()
            .set_title("另存为 Markdown")
            .add_filter("Markdown", &["md", "markdown"]),
        default_path,
    );
    dialog.save_file().map(path_to_string)
}

#[cfg(feature = "http")]
fn with_directory(dialog: rfd::FileDialog, default_path: Option<String>) -> rfd::FileDialog {
    let Some(value) = default_path else {
        return dialog;
    };
    let path = Path::new(&value);
    let directory = if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent().unwrap_or(path).to_path_buf()
    };
    if directory.as_os_str().is_empty() {
        dialog
    } else {
        dialog.set_directory(&directory)
    }
}

#[cfg(feature = "http")]
fn path_to_string(path: std::path::PathBuf) -> String {
    dunce::simplified(&path).to_string_lossy().into_owned()
}

fn with_json<T, R>(body: &str, work: impl FnOnce(T) -> AppResult<R>) -> (u16, String)
where
    T: DeserializeOwned,
    R: Serialize,
{
    match parse::<T>(body) {
        Ok(value) => from_result(work(value)),
        Err(reply) => reply,
    }
}

fn parse<T: DeserializeOwned>(body: &str) -> Result<T, (u16, String)> {
    serde_json::from_str(body).map_err(|_| error_body(400, "error", "请求无法解析"))
}

fn from_result<T: Serialize>(result: AppResult<T>) -> (u16, String) {
    match result {
        Ok(value) => json_ok(&value),
        Err(err) => {
            let status = if err.code == "external_change" {
                409
            } else {
                400
            };
            (
                status,
                serde_json::to_string(&err).unwrap_or_else(|_| fallback_error()),
            )
        }
    }
}

fn json_ok<T: Serialize>(value: &T) -> (u16, String) {
    (
        200,
        serde_json::to_string(value).unwrap_or_else(|_| fallback_error()),
    )
}

#[cfg(feature = "http")]
fn json_text(value: &str) -> (u16, String) {
    (
        200,
        serde_json::to_string(value).unwrap_or_else(|_| "\"cancel\"".to_string()),
    )
}

fn error_body(status: u16, code: &str, message: &str) -> (u16, String) {
    let err = AppError::new(code, message);
    (
        status,
        serde_json::to_string(&err).unwrap_or_else(|_| fallback_error()),
    )
}

fn fallback_error() -> String {
    r#"{"code":"error","message":"响应序列化失败"}"#.to_string()
}

#[derive(serde::Deserialize)]
struct OpenBody {
    path: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveBody {
    path: String,
    content: String,
    has_bom: bool,
    text_file_name: Option<String>,
    expected_sha256: Option<String>,
    overwrite_external: bool,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReadAssetBody {
    document_path: String,
    reference: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct WriteAssetBody {
    document_path: String,
    data_base64: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportBody {
    document_path: String,
    source_path: String,
}

#[cfg(feature = "http")]
#[derive(serde::Deserialize)]
struct TextBody {
    text: String,
}

#[cfg(feature = "http")]
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PickBody {
    #[serde(default)]
    default_path: Option<String>,
}

#[cfg(feature = "http")]
#[derive(Serialize)]
struct PathBody {
    path: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn opens_markdown_over_the_local_api() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("note.md");
        fs::write(&file, "hello").unwrap();
        let body = serde_json::json!({ "path": file }).to_string();
        let (status, payload) = dispatch("POST", "/api/open-document", &body);
        assert_eq!(status, 200, "{payload}");
        let value: serde_json::Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(value["kind"], "markdown");
        assert_eq!(value["content"], "hello");
        assert_eq!(value["hasBom"], false);
    }

    #[test]
    fn rejects_a_missing_document() {
        let (status, payload) = dispatch(
            "POST",
            "/api/open-document",
            r#"{"path":"Z:\\missing\\no-such.md"}"#,
        );
        assert_eq!(status, 400, "{payload}");
        let value: serde_json::Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(value["code"], "error");
    }

    #[test]
    fn rejects_unknown_routes() {
        let (status, _) = dispatch("POST", "/api/missing", "{}");
        assert_eq!(status, 404);
    }
}
