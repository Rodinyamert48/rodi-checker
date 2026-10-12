use checker_core::storage::Monitor;
use checker_core::CheckKind;
use tauri::State;

use crate::error::CommandResult;
use crate::state::AppState;

#[tauri::command]
pub async fn list_monitors(
    state: State<'_, AppState>,
    kind: Option<CheckKind>,
) -> CommandResult<Vec<Monitor>> {
    Ok(state.storage.list_monitors(kind).await?)
}
