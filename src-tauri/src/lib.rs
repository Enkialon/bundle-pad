mod assets;
mod atomic_write;
mod commands;
mod encoding;
mod error;
mod image_sniff;
mod markdown_doc;
mod models;
mod paths;
mod stamp;
mod textbundle;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default().plugin(tauri_plugin_dialog::init());
    commands::register(builder)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
