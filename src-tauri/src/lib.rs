mod app;
mod commands;
mod infrastructure;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default().plugin(tauri_plugin_dialog::init());
    commands::register(builder).run(tauri::generate_context!()).expect("error while running tauri application");
}
