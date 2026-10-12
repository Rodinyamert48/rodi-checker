use checker_core::AppSettings;
use tauri::State;
use tauri_plugin_autostart::ManagerExt;

use crate::error::CommandResult;
use crate::state::AppState;

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> AppSettings {
    state.settings()
}

#[tauri::command]
pub async fn save_settings(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    settings: AppSettings,
) -> CommandResult<AppSettings> {
    let previous = state.settings();
    let saved = settings.save(&state.storage).await?;

    if saved.autostart != previous.autostart {
        let autostart = app.autolaunch();
        if saved.autostart {
            autostart.enable()?;
        } else {
            autostart.disable()?;
        }
    }

    state.set_settings(saved.clone());
    Ok(saved)
}
