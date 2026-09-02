mod cleanup;
mod gradle;
mod scan;

use gradle::GradleState;
use std::{collections::HashMap, path::PathBuf, sync::Mutex};
use tauri::State;

#[derive(Default)]
struct NodeState(Mutex<HashMap<String, PathBuf>>);

#[tauri::command]
async fn scan_projects(
    root_path: String,
    state: State<'_, NodeState>,
) -> Result<scan::ScanResult, String> {
    let result = tauri::async_runtime::spawn_blocking(move || scan::scan(&root_path))
        .await
        .map_err(|error| format!("A varredura foi interrompida: {error}"))??;
    *state
        .0
        .lock()
        .map_err(|_| "O snapshot Node.js está indisponível.".to_string())? = result
        .projects
        .iter()
        .map(|project| {
            (
                project.id.clone(),
                PathBuf::from(&project.node_modules_path),
            )
        })
        .collect();
    Ok(result)
}

#[tauri::command]
async fn cleanup_projects(
    ids: Vec<String>,
    confirmed: bool,
    state: State<'_, NodeState>,
) -> Result<cleanup::CleanupResult, String> {
    let paths = {
        let registry = state
            .0
            .lock()
            .map_err(|_| "O snapshot Node.js está indisponível.".to_string())?;
        ids.iter()
            .map(|id| {
                registry
                    .get(id)
                    .ok_or_else(|| "Item Node.js desconhecido ou snapshot expirado.".to_string())
                    .map(|path| path.to_string_lossy().into_owned())
            })
            .collect::<Result<Vec<_>, _>>()?
    };
    tauri::async_runtime::spawn_blocking(move || cleanup::cleanup(&paths, confirmed))
        .await
        .map_err(|error| format!("A limpeza foi interrompida: {error}"))?
}

#[tauri::command]
async fn scan_gradle(
    root_path: String,
    state: State<'_, GradleState>,
) -> Result<gradle::GradleScan, String> {
    let (result, targets) = tauri::async_runtime::spawn_blocking(move || gradle::scan(&root_path))
        .await
        .map_err(|error| format!("A varredura Gradle foi interrompida: {error}"))??;
    *state
        .0
        .lock()
        .map_err(|_| "O snapshot de segurança está indisponível.".to_string())? = targets;
    Ok(result)
}

#[tauri::command]
async fn cleanup_gradle(
    ids: Vec<String>,
    confirmed: bool,
    state: State<'_, GradleState>,
) -> Result<gradle::CleanResult, String> {
    let snapshot = state
        .0
        .lock()
        .map_err(|_| "O snapshot de segurança está indisponível.".to_string())?
        .clone();
    tauri::async_runtime::spawn_blocking(move || gradle::clean(&ids, confirmed, &snapshot))
        .await
        .map_err(|error| format!("A limpeza Gradle foi interrompida: {error}"))?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(GradleState::default())
        .manage(NodeState::default())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            scan_projects,
            cleanup_projects,
            scan_gradle,
            cleanup_gradle
        ])
        .run(tauri::generate_context!())
        .expect("failed to run NodeSweep");
}
