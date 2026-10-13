mod commands;
pub mod database;
pub mod model;
pub mod operations;
pub mod rules;
pub mod scanner;
pub mod security;
pub mod service;
pub mod watcher;
use tauri::{Emitter, Manager};

fn quit_safely(app: &tauri::AppHandle) {
    use std::sync::atomic::Ordering;
    let service = app
        .state::<std::sync::Arc<service::Service>>()
        .inner()
        .clone();
    if service.stopping.swap(true, Ordering::SeqCst) {
        return;
    }
    service.cancel.store(true, Ordering::SeqCst);
    let app = app.clone();
    std::thread::spawn(move || {
        while service.busy.load(Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        app.exit(0);
    });
}

pub fn run() {
    tracing_subscriber::fmt().with_target(false).init();
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _shown = window.show();
                let _focus = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .args(["--minimized"])
                .build(),
        )
        .setup(|app| {
            if app.config().plugins.0.contains_key("updater") {
                app.handle()
                    .plugin(tauri_plugin_updater::Builder::new().build())?;
            }
            let directory = app.path().app_data_dir()?;
            #[cfg(debug_assertions)]
            let directory = std::env::var_os("FILEPILOT_TEST_DATA_DIR")
                .map(std::path::PathBuf::from)
                .unwrap_or(directory);
            std::fs::create_dir_all(&directory)?;
            let service = service::Service::open(&directory.join("filepilot.sqlite3"))?;
            app.manage(service.clone());
            use tauri::menu::{Menu, MenuItem};
            let open = MenuItem::with_id(app, "open", "FilePilot", true, None::<&str>)?;
            let organize =
                MenuItem::with_id(app, "organize", "Organiser / Organize", true, None::<&str>)?;
            let pause = MenuItem::with_id(app, "pause", "Suspendre / Pause", true, None::<&str>)?;
            let settings =
                MenuItem::with_id(app, "settings", "Paramètres / Settings", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quitter / Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &organize, &pause, &settings, &quit])?;
            tauri::tray::TrayIconBuilder::new()
                .icon(
                    app.default_window_icon()
                        .ok_or("Missing application icon")?
                        .clone(),
                )
                .tooltip("FilePilot")
                .menu(&menu)
                .on_menu_event(|app, event| {
                    if event.id.as_ref() == "quit" {
                        quit_safely(app);
                        return;
                    }
                    if event.id.as_ref() == "pause" {
                        let s = app
                            .state::<std::sync::Arc<service::Service>>()
                            .inner()
                            .clone();
                        if s.automatic.load(std::sync::atomic::Ordering::SeqCst) {
                            s.cancel.store(true, std::sync::atomic::Ordering::SeqCst);
                        }
                        let app = app.clone();
                        tauri::async_runtime::spawn_blocking(move || {
                            if let Ok(mut settings) =
                                s.db.lock()
                                    .map(|db| db.settings())
                                    .unwrap_or_else(|_| Err(model::invalid("Database lock failed")))
                            {
                                settings.automatic = !settings.automatic;
                                let _saved = s.save_settings(settings);
                                let _sent = app.emit("data-changed", ());
                            }
                        });
                    }
                    if let Some(window) = app.get_webview_window("main") {
                        let _shown = window.show();
                        let _focus = window.set_focus();
                    }
                    if ["organize", "settings"].contains(&event.id.as_ref()) {
                        let _sent = app.emit("navigate", event.id.as_ref());
                    }
                })
                .build(app)?;
            app.manage(watcher::start(service, app.handle().clone()));
            if std::env::args().any(|arg| arg == "--minimized") {
                if let Some(window) = app.get_webview_window("main") {
                    window.hide()?;
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let service = window.state::<std::sync::Arc<service::Service>>();
                if service
                    .close_to_tray
                    .load(std::sync::atomic::Ordering::SeqCst)
                {
                    api.prevent_close();
                    let _hidden = window.hide();
                } else {
                    api.prevent_close();
                    quit_safely(window.app_handle());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::snapshot,
            commands::save_folder,
            commands::save_rule,
            commands::save_settings,
            commands::remove_item,
            commands::scan_folder,
            commands::execute_preview,
            commands::undo_operation,
            commands::cancel_operation,
            commands::acknowledge_recovery,
            commands::open_folder,
            commands::open_project,
            commands::export_data,
            commands::import_rules,
            commands::check_update,
            commands::install_update,
            commands::app_version,
            commands::purge_history,
            commands::reset_preferences,
            commands::open_history_location,
            commands::undo_operations
        ])
        .run(tauri::generate_context!())
        .expect("FilePilot could not start");
}
