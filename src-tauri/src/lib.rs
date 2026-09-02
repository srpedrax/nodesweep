mod cleanup;
mod scan;

#[tauri::command]
async fn scan_projects(root_path: String) -> Result<scan::ScanResult, String> {
    tauri::async_runtime::spawn_blocking(move || scan::scan(&root_path))
        .await
        .map_err(|error| format!("A varredura foi interrompida: {error}"))?
}

#[tauri::command]
async fn cleanup_projects(
    paths: Vec<String>,
    confirmed: bool,
) -> Result<cleanup::CleanupResult, String> {
    tauri::async_runtime::spawn_blocking(move || cleanup::cleanup(&paths, confirmed))
        .await
        .map_err(|error| format!("A limpeza foi interrompida: {error}"))?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![scan_projects, cleanup_projects])
        .run(tauri::generate_context!())
        .expect("failed to run NodeSweep");
}
