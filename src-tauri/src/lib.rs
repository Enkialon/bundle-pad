mod app;
mod commands;
#[cfg(any(feature = "http", test))]
mod http_api;
mod infrastructure;
mod print_pdf;

#[cfg(feature = "http")]
pub fn serve_http(addr: &str) {
    http_api::serve(addr);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default().plugin(tauri_plugin_dialog::init());
    commands::register(builder).run(tauri::generate_context!()).expect("error while running tauri application");
}
