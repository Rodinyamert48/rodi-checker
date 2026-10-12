mod commands;
mod error;
mod monitor_runtime;
mod notify;
mod secret_key;
mod state;
mod system_runtime;
mod tray;

use std::sync::{Mutex, RwLock};

use checker_core::web::WebChecker;
use checker_core::{AppSettings, Scheduler, Storage};
use tauri::{Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;

use state::AppState;

const DATABASE_FILE: &str = "rodi-checker.db";

pub fn run() {
    tauri::Builder::default()
        // Must be registered first so a second launch focuses the running app.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_main_window(app);
        }))
        .plugin(tauri_plugin_log::Builder::new().level(log_level()).build())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let database_path = data_dir.join(DATABASE_FILE);

            let (storage, settings) = tauri::async_runtime::block_on(async {
                let storage = Storage::open(&database_path).await?;
                let settings = AppSettings::load(&storage).await?;
                anyhow::Ok((storage, settings))
            })?;

            let retention_days = settings.retention_days;
            let start_hidden =
                settings.start_minimized || std::env::args().any(|arg| arg == "--minimized");

            let secrets = secret_key::load(&data_dir)?;

            app.manage(AppState {
                storage: storage.clone(),
                scheduler: Scheduler::new(),
                web: WebChecker::new(),
                secrets,
                proxy_jobs: Default::default(),
                files_busy: Default::default(),
                files_cancel: Default::default(),
                watchers: Default::default(),
                system: Mutex::new(checker_core::system::SystemMonitor::new()),
                last_snapshot: Default::default(),
                alerts_firing: Default::default(),
                data_dir,
                database_path,
                settings: RwLock::new(settings),
            });

            tauri::async_runtime::spawn(async move {
                match storage.prune(retention_days).await {
                    Ok(removed) => tracing::info!("pruned {removed} old rows"),
                    Err(err) => tracing::warn!("pruning history failed: {err}"),
                }
            });

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                monitor_runtime::schedule_all(&handle).await;
                commands::files::restore_watchers(&handle).await;
                tray::refresh_status(&handle).await;
            });

            system_runtime::start(app.handle().clone());

            tray::create(app.handle())?;

            if !start_hidden {
                tray::show_main_window(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<AppState>();
                if state.settings().minimize_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::app_info,
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::monitors::list_monitors,
            commands::monitors::create_monitor,
            commands::monitors::update_monitor,
            commands::monitors::delete_monitor,
            commands::monitors::run_monitor_now,
            commands::monitors::monitor_results,
            commands::monitors::monitor_stats,
            commands::monitors::latest_results,
            commands::monitors::list_incidents,
            commands::app::read_text_file,
            commands::proxy::proxy_preview,
            commands::proxy::proxy_import,
            commands::proxy::proxy_lists,
            commands::proxy::proxy_items,
            commands::proxy::proxy_delete_list,
            commands::proxy::proxy_remove_dead,
            commands::proxy::proxy_running,
            commands::proxy::proxy_cancel,
            commands::proxy::proxy_check,
            commands::proxy::proxy_export,
            commands::files::files_hash,
            commands::files::files_baselines,
            commands::files::files_cancel,
            commands::files::files_create_baseline,
            commands::files::files_compare,
            commands::files::files_delete_baseline,
            commands::files::files_watching,
            commands::files::files_watch,
            commands::system::system_snapshot,
            commands::system::system_processes,
            commands::system::system_kill,
            commands::system::system_ports,
            commands::system::system_services,
            commands::system::system_history,
            commands::system::system_alert_rules,
            commands::system::system_create_alert_rule,
            commands::system::system_toggle_alert_rule,
            commands::system::system_delete_alert_rule,
            commands::system::system_firing,
            commands::reports::report_preview,
            commands::reports::report_export,
            commands::dashboard::dashboard_summary,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Rodi Checker");
}

fn log_level() -> log::LevelFilter {
    if cfg!(debug_assertions) {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    }
}
