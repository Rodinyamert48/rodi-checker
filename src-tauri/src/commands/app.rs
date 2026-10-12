use serde::Serialize;
use tauri::State;

use crate::state::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    name: String,
    version: String,
    data_dir: String,
    database_path: String,
}

#[tauri::command]
pub fn app_info(app: tauri::AppHandle, state: State<'_, AppState>) -> AppInfo {
    let package = app.package_info();
    AppInfo {
        name: package.name.clone(),
        version: package.version.to_string(),
        data_dir: state.data_dir.display().to_string(),
        database_path: state.database_path.display().to_string(),
    }
}

/// Reads a user-selected text file (proxy lists, hash files). Max 50 MB.
#[tauri::command]
pub async fn read_text_file(path: String) -> crate::error::CommandResult<String> {
    let metadata = tokio::fs::metadata(&path).await?;
    if metadata.len() > 50 * 1024 * 1024 {
        return Err("file is larger than 50 MB".into());
    }
    let bytes = tokio::fs::read(&path).await?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}
