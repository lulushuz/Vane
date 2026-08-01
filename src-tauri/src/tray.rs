use crate::commands::open_settings_window;
use crate::updater::check_for_updates;
use crate::AppState;
use tauri::menu::{IconMenuItem, MenuBuilder, MenuItem, PredefinedMenuItem};
use tauri::Manager;

pub fn setup_tray(app: &tauri::App) -> Result<(), tauri::Error> {
    if let Some(icon) = app.default_window_icon().cloned() {
        let header_item = IconMenuItem::with_id(
            app,
            "header",
            "Vane",
            false,
            Some(icon.clone()),
            None::<&str>,
        )?;
        let show_item = MenuItem::with_id(app, "show", "Show / Hide", true, None::<&str>)?;
        let update_item =
            MenuItem::with_id(app, "update", "Check for Updates...", true, None::<&str>)?;
        let settings_item = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
        let quit_item = MenuItem::with_id(app, "quit", "Quit Vane", true, None::<&str>)?;

        let menu = MenuBuilder::new(app)
            .items(&[
                &header_item,
                &PredefinedMenuItem::separator(app)?,
                &show_item,
                &update_item,
                &settings_item,
                &PredefinedMenuItem::separator(app)?,
                &quit_item,
            ])
            .build()?;

        let _tray = tauri::tray::TrayIconBuilder::new()
            .icon(icon)
            .tooltip("Vane")
            .menu(&menu)
            .on_menu_event(|app, event| match event.id().as_ref() {
                "quit" => {
                    tracing::info!("Exit requested from tray menu.");
                    if let Some(state) = app.try_state::<AppState>() {
                        let manager = state.engine_manager.clone();
                        let app_handle = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let _ = manager.stop(&app_handle).await;
                            app_handle.exit(0);
                        });
                        return;
                    }
                    app.exit(0);
                }
                "show" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let is_visible = window.is_visible().unwrap_or(false);
                        if is_visible {
                            let _ = window.hide();
                        } else {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                }
                "update" => {
                    tracing::info!("Update check requested from tray menu.");
                    let app_handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = check_for_updates(app_handle).await;
                    });
                }
                "settings" => {
                    tracing::info!("Settings requested from tray menu.");
                    let app_handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = open_settings_window(app_handle).await;
                    });
                }
                _ => {}
            })
            .on_tray_icon_event(|tray, event| {
                if let tauri::tray::TrayIconEvent::Click {
                    button: tauri::tray::MouseButton::Left,
                    button_state: tauri::tray::MouseButtonState::Up,
                    ..
                } = event
                {
                    let app_handle = tray.app_handle();
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let is_visible = window.is_visible().unwrap_or(false);
                        if is_visible {
                            let _ = window.hide();
                        } else {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                }
            })
            .build(app)?;
    }
    Ok(())
}
