use crate::{model::*, operations, service::Service};
use std::{
    path::Path,
    sync::{atomic::Ordering, Arc},
};
use tauri::{Emitter, State};
type Shared<'a> = State<'a, Arc<Service>>;
fn message<T>(result: Result<T>) -> std::result::Result<T, String> {
    result.map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn snapshot(service: Shared<'_>) -> std::result::Result<Snapshot, String> {
    let s = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || message(s.snapshot()))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn save_folder(
    service: Shared<'_>,
    folder: Folder,
) -> std::result::Result<Folder, String> {
    let s = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || message(s.save_folder(folder)))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn save_rule(service: Shared<'_>, rule: Rule) -> std::result::Result<Rule, String> {
    let s = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || message(s.save_rule(rule)))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn save_settings(
    service: Shared<'_>,
    settings: Settings,
) -> std::result::Result<(), String> {
    let s = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || message(s.save_settings(settings)))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn remove_item(
    service: Shared<'_>,
    kind: String,
    id: String,
) -> std::result::Result<(), String> {
    let s = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        message((|| {
            if !["folder", "rule"].contains(&kind.as_str()) {
                return Err(invalid("Invalid object kind"));
            }
            let db = s.db.lock().map_err(|_| invalid("Database lock failed"))?;
            if kind == "folder"
                && db
                    .list::<Rule>("rule")?
                    .iter()
                    .any(|r| r.sources.contains(&id))
            {
                return Err(invalid("Remove or edit this folder's rules first"));
            }
            db.delete(&kind, &id)?;
            s.previews
                .lock()
                .map_err(|_| invalid("Preview lock failed"))?
                .clear();
            Ok(())
        })())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn scan_folder(
    app: tauri::AppHandle,
    service: Shared<'_>,
    folder_id: String,
    rule_ids: Vec<String>,
) -> std::result::Result<Scan, String> {
    let s = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        message(s.scan(&folder_id, &rule_ids, |count| {
            let _sent = app.emit(
                "progress",
                serde_json::json!({"phase":"scan","done":count,"total":null}),
            );
        }))
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn execute_preview(
    app: tauri::AppHandle,
    service: Shared<'_>,
    token: String,
    ids: Vec<String>,
) -> std::result::Result<Vec<Operation>, String> {
    let s = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = message(s.execute(&token, &ids, |done, total| {
            let _sent = app.emit(
                "progress",
                serde_json::json!({"phase":"execute","done":done,"total":total}),
            );
        }));
        let _sent = app.emit("data-changed", ());
        result
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn undo_operation(
    app: tauri::AppHandle,
    service: Shared<'_>,
    id: String,
) -> std::result::Result<Operation, String> {
    let s = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = message((|| {
            let _busy = s.acquire()?;
            let db = s.db.lock().map_err(|_| invalid("Database lock failed"))?;
            operations::undo(&db, &id)
        })());
        let _sent = app.emit("data-changed", ());
        result
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub fn cancel_operation(service: Shared<'_>) {
    service.cancel.store(true, Ordering::SeqCst);
}
#[tauri::command]
pub async fn acknowledge_recovery(service: Shared<'_>) -> std::result::Result<(), String> {
    let s = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move||message((|| {
        let _busy=s.acquire()?; let db=s.db.lock().map_err(|_|invalid("Database lock failed"))?;
        db.conn.execute("UPDATE history SET status='resolved',data=json_set(data,'$.status','resolved') WHERE status='reconcile'",[])?;
        s.safe_mode.store(false,Ordering::SeqCst); Ok(())
    })())).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub async fn open_folder(
    app: tauri::AppHandle,
    service: Shared<'_>,
    id: String,
) -> std::result::Result<(), String> {
    let s = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let db =
            s.db.lock()
                .map_err(|_| "Database lock failed".to_string())?;
        let folder = message(db.list::<Folder>("folder"))?
            .into_iter()
            .find(|f| f.id == id)
            .ok_or("Unknown folder")?;
        message(crate::security::directory(Path::new(&folder.path)))?;
        use tauri_plugin_opener::OpenerExt;
        app.opener()
            .open_path(folder.path, None::<&str>)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn open_project(app: tauri::AppHandle) -> std::result::Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri_plugin_opener::OpenerExt;
        app.opener()
            .open_url("https://github.com/crype-dev/FilePilot", None::<&str>)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn export_data(
    service: Shared<'_>,
    path: String,
    kind: String,
) -> std::result::Result<(), String> {
    let s = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        message((|| {
            crate::security::guard(Path::new(&path))?;
            let db = s.db.lock().map_err(|_| invalid("Database lock failed"))?;
            let data = match kind.as_str() {
                "rules" => serde_json::to_string_pretty(&db.list::<Rule>("rule")?)?,
                "history" => serde_json::to_string_pretty(&db.history()?)?,
                "settings" => serde_json::to_string_pretty(&db.settings()?)?,
                "diagnostics" => serde_json::to_string_pretty(&serde_json::json!({"os":std::env::consts::OS,"arch":std::env::consts::ARCH,"folders":db.list::<Folder>("folder")?.len(),"rules":db.list::<Rule>("rule")?.len(),"safe_mode":s.safe_mode.load(Ordering::SeqCst),"busy":s.busy.load(Ordering::SeqCst),"version":env!("CARGO_PKG_VERSION")}))?,
                _ => return Err(invalid("Unknown export")),
            };
            use std::io::Write;
            let mut output = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)?;
            output.write_all(data.as_bytes())?;
            output.sync_all()?;
            Ok(())
        })())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn import_rules(service: Shared<'_>, path: String) -> std::result::Result<usize, String> {
    let s = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        message((|| {
            crate::security::guard(Path::new(&path))?;
            if std::fs::metadata(&path)?.len() > 1_000_000 {
                return Err(invalid("Import exceeds 1 MB"));
            }
            let mut rules: Vec<Rule> = serde_json::from_slice(&std::fs::read(path)?)?;
            if rules.len() > 200 {
                return Err(invalid("Maximum 200 rules per import"));
            }
            let mut db = s.db.lock().map_err(|_| invalid("Database lock failed"))?;
            let folders = db.list::<Folder>("folder")?;
            for rule in &mut rules {
                crate::rules::validate(rule, &folders)?;
                rule.id = crate::model::id();
                rule.enabled = false;
                rule.created_at = now();
                rule.updated_at = now();
            }
            let tx = db.conn.transaction()?;
            for rule in &rules {
                tx.execute(
                    "INSERT INTO objects VALUES('rule',?1,?2)",
                    rusqlite::params![rule.id, serde_json::to_string(rule)?],
                )?;
            }
            tx.commit()?;
            s.previews
                .lock()
                .map_err(|_| invalid("Preview lock failed"))?
                .clear();
            Ok(rules.len())
        })())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn check_update(app: tauri::AppHandle) -> std::result::Result<Option<String>, String> {
    if !app.config().plugins.0.contains_key("updater") {
        return Err("Updater unavailable: this local build has no production signing key".into());
    }
    use tauri_plugin_updater::UpdaterExt;
    let update = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?;
    Ok(update.map(|u| u.version))
}
#[tauri::command]
pub async fn install_update(
    app: tauri::AppHandle,
    service: Shared<'_>,
) -> std::result::Result<(), String> {
    if !app.config().plugins.0.contains_key("updater") {
        return Err("Updater unavailable: this local build has no production signing key".into());
    }
    let _busy = service.acquire().map_err(|e| e.to_string())?;
    use tauri_plugin_updater::UpdaterExt;
    let update = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?
        .ok_or("No update available")?;
    update
        .download_and_install(
            |chunk, total| {
                let _sent = app.emit(
                    "update-progress",
                    serde_json::json!({"chunk":chunk,"total":total}),
                );
            },
            || {},
        )
        .await
        .map_err(|e| e.to_string())?;
    app.restart()
}
#[tauri::command]
pub fn app_version(app: tauri::AppHandle) -> String {
    app.package_info().version.to_string()
}
#[tauri::command]
pub async fn purge_history(service: Shared<'_>) -> std::result::Result<usize, String> {
    let s = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        message((|| {
            let _busy = s.acquire()?;
            let db = s.db.lock().map_err(|_| invalid("Database lock failed"))?;
            if db.settings()?.automatic {
                return Err(invalid("Pause automation before clearing history"));
            }
            db.purge_history(now().saturating_add(1))
        })())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn undo_operations(
    app: tauri::AppHandle,
    service: Shared<'_>,
    ids: Vec<String>,
) -> std::result::Result<Vec<Operation>, String> {
    let s = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = message(s.undo_selection(&ids));
        let _sent = app.emit("data-changed", ());
        result
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn reset_preferences(service: Shared<'_>) -> std::result::Result<(), String> {
    let s = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || message(s.save_settings(Settings::default())))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn open_history_location(
    app: tauri::AppHandle,
    service: Shared<'_>,
    id: String,
    side: String,
) -> std::result::Result<(), String> {
    let s = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let db = s.db.lock().map_err(|_| "Database lock failed")?;
        let op = message(db.history())?
            .into_iter()
            .find(|o| o.id == id)
            .ok_or("Unknown operation")?;
        let path = match side.as_str() {
            "source" => op.source,
            "destination" => op.destination,
            _ => return Err("Invalid history location".into()),
        };
        let parent = Path::new(&path).parent().ok_or("Missing parent folder")?;
        message(crate::security::directory(parent))?;
        use tauri_plugin_opener::OpenerExt;
        app.opener()
            .open_path(parent.to_string_lossy(), None::<&str>)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
