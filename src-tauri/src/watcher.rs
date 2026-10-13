use crate::{model::*, operations, scanner, service::Service};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    collections::HashMap,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter};

#[derive(Debug)]
pub enum WorkerEvent {
    Ready,
    Changed(usize),
    Error(String),
}
pub struct Worker {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _joined = thread.join();
        }
    }
}
pub fn start(service: Arc<Service>, app: AppHandle) -> Worker {
    let state = service.clone();
    start_worker(
        service,
        Arc::new(move |event| match event {
            WorkerEvent::Ready => {}
            WorkerEvent::Error(error) => {
                let _sent = app.emit("engine-error", error);
            }
            WorkerEvent::Changed(count) => {
                let _sent = app.emit("data-changed", ());
                if state
                    .db
                    .try_lock()
                    .is_ok_and(|db| db.settings().is_ok_and(|s| s.notifications))
                {
                    use tauri_plugin_notification::NotificationExt;
                    let _shown = app
                        .notification()
                        .builder()
                        .title("FilePilot")
                        .body(format!("{count} file(s) organized"))
                        .show();
                }
            }
        }),
    )
}
pub fn start_worker(
    service: Arc<Service>,
    events: Arc<dyn Fn(WorkerEvent) + Send + Sync>,
) -> Worker {
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = stop.clone();
    let thread = std::thread::spawn(move || {
        let dirty = Arc::new(AtomicBool::new(true));
        let changed = dirty.clone();
        let errors = events.clone();
        let mut watcher = match RecommendedWatcher::new(
            move |event: notify::Result<notify::Event>| {
                changed.store(true, Ordering::Relaxed);
                if let Err(e) = event {
                    errors(WorkerEvent::Error(e.to_string()));
                }
            },
            notify::Config::default(),
        ) {
            Ok(w) => w,
            Err(e) => {
                events(WorkerEvent::Error(e.to_string()));
                return;
            }
        };
        let mut configuration = String::new();
        let mut watched = Vec::new();
        let mut observed: HashMap<String, (String, Instant, bool)> = HashMap::new();
        let mut excluded: HashMap<String, (u64, u128)> = HashMap::new();
        let mut last_scan = Instant::now();
        let mut baseline = true;
        let mut last_retention = None::<Instant>;
        while !stopped.load(Ordering::SeqCst) && !service.stopping.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(250));
            let db = match service.db.try_lock() {
                Ok(db) => db,
                Err(_) => continue,
            };
            let settings = match db.settings() {
                Ok(s) => s,
                Err(e) => {
                    events(WorkerEvent::Error(e.to_string()));
                    continue;
                }
            };
            if settings.history_days > 0
                && last_retention.is_none_or(|t| t.elapsed().as_secs() >= 86400)
            {
                if let Err(e) =
                    db.purge_history(now().saturating_sub(settings.history_days * 86400))
                {
                    events(WorkerEvent::Error(e.to_string()));
                }
                last_retention = Some(Instant::now());
            }
            let folders = match db.list::<Folder>("folder") {
                Ok(f) => f,
                Err(e) => {
                    events(WorkerEvent::Error(e.to_string()));
                    continue;
                }
            };
            let signature = match serde_json::to_string(&(&folders, &settings.automatic)) {
                Ok(s) => s,
                Err(_) => continue,
            };
            if signature != configuration {
                for path in &watched {
                    let _removed = watcher.unwatch(Path::new(path));
                }
                watched.clear();
                observed.clear();
                excluded.clear();
                baseline = true;
                for folder in folders.iter().filter(|f| f.enabled) {
                    match watcher.watch(
                        Path::new(&folder.path),
                        if folder.recursive {
                            RecursiveMode::Recursive
                        } else {
                            RecursiveMode::NonRecursive
                        },
                    ) {
                        Ok(()) => watched.push(folder.path.clone()),
                        Err(e) => events(WorkerEvent::Error(e.to_string())),
                    }
                }
                configuration = signature;
                dirty.store(true, Ordering::Relaxed);
            }
            if !settings.automatic || service.safe_mode.load(Ordering::SeqCst) {
                continue;
            }
            let settling = observed.values().any(|(_, _, handled)| !handled);
            if !dirty.load(Ordering::Relaxed)
                && !settling
                && last_scan.elapsed().as_secs() < settings.reconciliation_seconds
            {
                continue;
            }
            // Debounce noisy events; observations remain separated by the stability interval.
            if !baseline && last_scan.elapsed().as_secs() < settings.stabilization_seconds {
                continue;
            }
            let busy = match service.acquire() {
                Ok(b) => b,
                Err(_) => continue,
            };
            dirty.store(false, Ordering::Relaxed);
            last_scan = Instant::now();
            let mut current = std::collections::HashSet::new();
            let mut completed = 0;
            for folder in folders.iter().filter(|f| f.enabled) {
                match scanner::scan(&db, folder, &[], &service.cancel, |_| {}) {
                    Ok(scan) => {
                        let inventory: HashMap<_, _> = scan
                            .inventory
                            .iter()
                            .map(|f| (f.source.clone(), (f.size, f.modified_ns)))
                            .collect();
                        // Baseline every file, including files that do not match a rule yet.
                        // Keep exclusions from other watched folders; remove entries only within this source.
                        excluded.retain(|path, signature| {
                            !Path::new(path).starts_with(&folder.path)
                                || inventory.get(path) == Some(signature)
                        });
                        if baseline && !settings.scan_on_start {
                            excluded.extend(inventory.clone());
                        }
                        for p in scan.proposals.iter().filter(|p| p.status == "ready") {
                            if excluded.get(&p.source).is_some_and(|signature| {
                                inventory.get(&p.source) == Some(signature)
                            }) {
                                continue;
                            }
                            current.insert(p.source.clone());
                            let fingerprint = format!("{}:{}:{}", p.size, p.modified, p.hash);
                            let entry = observed.entry(p.source.clone()).or_insert((
                                fingerprint.clone(),
                                Instant::now(),
                                baseline && !settings.scan_on_start,
                            ));
                            if entry.0 != fingerprint {
                                *entry = (fingerprint, Instant::now(), false);
                            }
                            if !entry.2
                                && entry.1.elapsed().as_secs() >= settings.stabilization_seconds
                                && !service.cancel.load(Ordering::SeqCst)
                            {
                                match operations::execute(&db, p) {
                                    Ok(op) => {
                                        entry.2 = true;
                                        if op.status == "completed" {
                                            completed += 1;
                                        } else {
                                            if op.status == "reconcile" {
                                                service.safe_mode.store(true, Ordering::SeqCst);
                                                service.cancel.store(true, Ordering::SeqCst);
                                            }
                                            events(WorkerEvent::Error(
                                                op.error.unwrap_or_default(),
                                            ));
                                        }
                                    }
                                    Err(e) => {
                                        entry.2 = true;
                                        events(WorkerEvent::Error(e.to_string()));
                                    }
                                }
                            }
                        }
                        for error in scan.errors {
                            events(WorkerEvent::Error(error));
                        }
                    }
                    Err(e) => events(WorkerEvent::Error(e.to_string())),
                }
            }
            let first = baseline;
            baseline = false;
            observed.retain(|path, _| current.contains(path));
            drop(db);
            drop(busy);
            if first {
                events(WorkerEvent::Ready);
            }
            if completed > 0 {
                events(WorkerEvent::Changed(completed));
            }
        }
    });
    Worker {
        stop,
        thread: Some(thread),
    }
}
