use checker_core::Status;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

use crate::notify;
use crate::state::AppState;

const TRAY_ID: &str = "main";

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let show = MenuItem::with_id(
        app,
        "show",
        notify::tr_en(app, "Göster", "Show"),
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(
        app,
        "quit",
        notify::tr_en(app, "Çıkış", "Quit"),
        true,
        None::<&str>,
    )?;
    Menu::with_items(app, &[&show, &PredefinedMenuItem::separator(app)?, &quit])
}

/// Re-creates the tray menu, e.g. after the language changed.
pub fn rebuild_menu(app: &AppHandle) {
    if let (Some(tray), Ok(menu)) = (app.tray_by_id(TRAY_ID), build_menu(app)) {
        let _ = tray.set_menu(Some(menu));
    }
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app)?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Rodi Checker")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
            "quit" => {
                app.state::<AppState>().scheduler.shutdown();
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

/// Overall health shown on the tray icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Health {
    Unknown,
    Ok,
    Warning,
    Critical,
}

impl Health {
    fn color(self) -> Option<[u8; 3]> {
        match self {
            Health::Unknown => None,
            Health::Ok => Some([0x16, 0xa3, 0x4a]),
            Health::Warning => Some([0xd9, 0x77, 0x06]),
            Health::Critical => Some([0xdc, 0x26, 0x26]),
        }
    }
}

/// App icon with a status dot in the bottom-right corner.
fn icon_with_dot(base: &Image<'_>, color: [u8; 3]) -> Image<'static> {
    let (width, height) = (base.width(), base.height());
    let mut rgba = base.rgba().to_vec();
    let radius = width.min(height) as f32 * 0.22;
    let (cx, cy) = (width as f32 - radius - 1.0, height as f32 - radius - 1.0);
    for y in 0..height {
        for x in 0..width {
            let distance = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
            let index = ((y * width + x) * 4) as usize;
            if distance <= radius {
                rgba[index..index + 4].copy_from_slice(&[color[0], color[1], color[2], 255]);
            } else if distance <= radius + 1.5 {
                // White ring separating the dot from the icon.
                rgba[index..index + 4].copy_from_slice(&[255, 255, 255, 255]);
            }
        }
    }
    Image::new_owned(rgba, width, height)
}

/// Recomputes the overall health and updates the tray icon and tooltip.
pub async fn refresh_status(app: &AppHandle) {
    let state = app.state::<AppState>();
    let Ok(monitors) = state.storage.list_monitors(None).await else {
        return;
    };
    let Ok(latest) = state.storage.latest_results().await else {
        return;
    };
    let enabled: Vec<_> = monitors
        .iter()
        .filter(|m| m.enabled)
        .map(|m| m.id)
        .collect();
    let statuses: Vec<Status> = latest
        .iter()
        .filter(|r| enabled.contains(&r.monitor_id))
        .map(|r| r.result.status)
        .collect();
    let down = statuses.iter().filter(|s| !s.is_ok()).count();
    let degraded = statuses.iter().filter(|s| **s == Status::Degraded).count();
    let alerts = state.alerts_firing.lock().unwrap().len();

    let health = if down > 0 {
        Health::Critical
    } else if degraded > 0 || alerts > 0 {
        Health::Warning
    } else if statuses.is_empty() {
        Health::Unknown
    } else {
        Health::Ok
    };

    let up = statuses.len() - down;
    let mut tooltip = format!(
        "Rodi Checker\n{}",
        notify::tr_en(
            app,
            format!("{up}/{} izleyici çalışıyor", statuses.len()),
            format!("{up}/{} monitors up", statuses.len())
        )
    );
    if alerts > 0 {
        tooltip.push('\n');
        tooltip.push_str(&notify::tr_en(
            app,
            format!("{alerts} aktif uyarı"),
            format!("{alerts} active alerts"),
        ));
    }

    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let _ = tray.set_tooltip(Some(&tooltip));
    if let Some(base) = app.default_window_icon() {
        let icon = match health.color() {
            Some(color) => icon_with_dot(base, color),
            None => base.clone().to_owned(),
        };
        let _ = tray.set_icon(Some(icon));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draws_status_dot() {
        let base = Image::new_owned(vec![0; 32 * 32 * 4], 32, 32);
        let icon = icon_with_dot(&base, [1, 2, 3]);
        let pixel = |x: u32, y: u32| icon.rgba()[((y * 32 + x) * 4) as usize..][..4].to_vec();
        assert_eq!(pixel(31 - 7, 31 - 7), vec![1, 2, 3, 255]);
        assert_eq!(pixel(0, 0), vec![0, 0, 0, 0]);
    }
}
