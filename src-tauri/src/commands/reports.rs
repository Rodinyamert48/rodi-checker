use checker_core::reports::{self, ReportFormat, ReportRequest, Table};
use serde::Serialize;
use tauri::State;

use crate::error::CommandResult;
use crate::state::AppState;

const PREVIEW_ROWS: usize = 100;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportPreview {
    table: Table,
    total_rows: usize,
}

#[tauri::command]
pub async fn report_preview(
    state: State<'_, AppState>,
    request: ReportRequest,
) -> CommandResult<ReportPreview> {
    let mut table = reports::build(&state.storage, &request, state.settings().language).await?;
    let total_rows = table.rows.len();
    table.rows.truncate(PREVIEW_ROWS);
    Ok(ReportPreview { table, total_rows })
}

/// Writes the report to `path`; returns the number of rows.
#[tauri::command]
pub async fn report_export(
    state: State<'_, AppState>,
    request: ReportRequest,
    format: ReportFormat,
    path: String,
) -> CommandResult<usize> {
    let language = state.settings().language;
    let table = reports::build(&state.storage, &request, language).await?;
    tokio::fs::write(&path, table.render(format, language)).await?;
    Ok(table.rows.len())
}
