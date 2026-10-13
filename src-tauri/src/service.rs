use crate::{database::Database, model::*, operations, rules, scanner, security};
use std::{
    collections::HashMap,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

pub struct Service {
    pub db: Mutex<Database>,
    pub cancel: AtomicBool,
    pub busy: AtomicBool,
    pub safe_mode: AtomicBool,
    pub stopping: AtomicBool,
    pub close_to_tray: AtomicBool,
    pub automatic: AtomicBool,
    pub previews: Mutex<HashMap<String, (u64, Vec<Proposal>)>>,
}
impl Service {
    pub fn open(path: &Path) -> Result<Arc<Self>> {
        let db = Database::open(path)?;
        let unsafe_restart = db.recover()?;
        let settings = db.settings()?;
        Ok(Arc::new(Self {
            db: Mutex::new(db),
            cancel: AtomicBool::new(false),
            busy: AtomicBool::new(false),
            safe_mode: AtomicBool::new(unsafe_restart),
            stopping: AtomicBool::new(false),
            close_to_tray: AtomicBool::new(settings.close_to_tray),
            automatic: AtomicBool::new(settings.automatic),
            previews: Mutex::new(HashMap::new()),
        }))
    }
    pub fn snapshot(&self) -> Result<Snapshot> {
        let db = self
            .db
            .lock()
            .map_err(|_| invalid("Database lock failed"))?;
        Ok(Snapshot {
            folders: db.list("folder")?,
            rules: db.list("rule")?,
            settings: db.settings()?,
            history: db.history()?,
            busy: self.busy.load(Ordering::SeqCst),
            safe_mode: self.safe_mode.load(Ordering::SeqCst),
        })
    }
    pub fn save_folder(&self, mut folder: Folder) -> Result<Folder> {
        security::directory(Path::new(&folder.path))?;
        if folder.name.trim().is_empty() || folder.name.len() > 120 {
            return Err(invalid("Folder name is required"));
        }
        let db = self
            .db
            .lock()
            .map_err(|_| invalid("Database lock failed"))?;
        let folders = db.list::<Folder>("folder")?;
        for old in &folders {
            if old.id != folder.id
                && (security::inside(Path::new(&old.path), Path::new(&folder.path))
                    || security::inside(Path::new(&folder.path), Path::new(&old.path)))
            {
                return Err(invalid("Overlapping source folders are unsupported"));
            }
        }
        if folder.id.is_empty() {
            folder.id = id();
        }
        let mut updated = folders;
        updated.retain(|f| f.id != folder.id);
        updated.push(folder.clone());
        for r in db.list::<Rule>("rule")? {
            rules::validate(&r, &updated)?;
        }
        db.put("folder", &folder.id, &folder)?;
        self.previews
            .lock()
            .map_err(|_| invalid("Preview lock failed"))?
            .clear();
        Ok(folder)
    }
    pub fn save_rule(&self, mut rule: Rule) -> Result<Rule> {
        let db = self
            .db
            .lock()
            .map_err(|_| invalid("Database lock failed"))?;
        rules::validate(&rule, &db.list("folder")?)?;
        if rule.id.is_empty() {
            rule.id = id();
            rule.created_at = now();
        }
        rule.updated_at = now();
        db.put("rule", &rule.id, &rule)?;
        self.previews
            .lock()
            .map_err(|_| invalid("Preview lock failed"))?
            .clear();
        Ok(rule)
    }
    pub fn save_settings(&self, settings: Settings) -> Result<()> {
        if settings.history_days > 36500 {
            return Err(invalid("History retention exceeds 100 years"));
        }
        if !["fr", "en"].contains(&settings.language.as_str())
            || !["light", "dark", "system"].contains(&settings.theme.as_str())
            || !(2..=300).contains(&settings.stabilization_seconds)
            || !(30..=86400).contains(&settings.reconciliation_seconds)
        {
            return Err(invalid("Invalid preferences"));
        }
        if self.automatic.load(Ordering::SeqCst) && !settings.automatic {
            self.cancel.store(true, Ordering::SeqCst);
        }
        self.db
            .lock()
            .map_err(|_| invalid("Database lock failed"))?
            .put("settings", "main", &settings)?;
        self.close_to_tray
            .store(settings.close_to_tray, Ordering::SeqCst);
        self.automatic.store(settings.automatic, Ordering::SeqCst);
        Ok(())
    }
    pub fn scan(
        &self,
        folder_id: &str,
        selected: &[String],
        progress: impl Fn(usize),
    ) -> Result<Scan> {
        let _busy = self.acquire()?;
        let db = self
            .db
            .lock()
            .map_err(|_| invalid("Database lock failed"))?;
        let folder = db
            .list::<Folder>("folder")?
            .into_iter()
            .find(|f| f.id == folder_id)
            .ok_or_else(|| invalid("Unknown source folder"))?;
        let scan = scanner::scan(&db, &folder, selected, &self.cancel, progress)?;
        let mut previews = self
            .previews
            .lock()
            .map_err(|_| invalid("Preview lock failed"))?;
        previews.clear();
        previews.insert(scan.token.clone(), (now(), scan.proposals.clone()));
        Ok(scan)
    }
    pub fn execute(
        &self,
        token: &str,
        ids: &[String],
        progress: impl Fn(usize, usize),
    ) -> Result<Vec<Operation>> {
        let _busy = self.acquire()?;
        let (created, proposals) = self
            .previews
            .lock()
            .map_err(|_| invalid("Preview lock failed"))?
            .remove(token)
            .ok_or_else(|| invalid("Preview expired or changed; scan again"))?;
        if now().saturating_sub(created) > 900 {
            return Err(invalid("Preview expired; scan again"));
        }
        if ids
            .iter()
            .any(|id| !proposals.iter().any(|p| &p.id == id && p.status == "ready"))
        {
            return Err(invalid("Invalid proposal selection"));
        }
        let db = self
            .db
            .lock()
            .map_err(|_| invalid("Database lock failed"))?;
        let mut done = vec![];
        for proposal in proposals.iter().filter(|p| ids.contains(&p.id)) {
            if self.cancel.load(Ordering::SeqCst) {
                break;
            }
            let operation = operations::execute(&db, proposal)?;
            if operation.status == "reconcile" {
                self.safe_mode.store(true, Ordering::SeqCst);
                self.cancel.store(true, Ordering::SeqCst);
            }
            done.push(operation);
            progress(done.len(), ids.len());
        }
        Ok(done)
    }
    pub fn acquire(&self) -> Result<Busy<'_>> {
        self.busy
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map_err(|_| invalid("Another operation is active"))?;
        if self.stopping.load(Ordering::SeqCst) {
            self.busy.store(false, Ordering::SeqCst);
            return Err(invalid("FilePilot is shutting down"));
        }
        self.cancel.store(false, Ordering::SeqCst);
        Ok(Busy(self))
    }
    pub fn undo_selection(&self, ids: &[String]) -> Result<Vec<Operation>> {
        let _busy = self.acquire()?;
        if ids.is_empty() || ids.len() > 100 {
            return Err(invalid("Select between 1 and 100 operations"));
        }
        let db = self
            .db
            .lock()
            .map_err(|_| invalid("Database lock failed"))?;
        let mut sources = std::collections::HashSet::new();
        for id in ids {
            let op = operations::undo_eligible(&db, id)?;
            if !sources.insert(security::path_key(Path::new(&op.source))) {
                return Err(invalid("Selected operations share an original path"));
            }
        }
        let mut done = Vec::new();
        for id in ids {
            if self.cancel.load(Ordering::SeqCst) {
                break;
            }
            let op = operations::undo(&db, id)?;
            if op.status == "reconcile" {
                self.safe_mode.store(true, Ordering::SeqCst);
                self.cancel.store(true, Ordering::SeqCst);
            }
            done.push(op);
        }
        Ok(done)
    }
}
pub struct Busy<'a>(&'a Service);
impl Drop for Busy<'_> {
    fn drop(&mut self) {
        self.0.busy.store(false, Ordering::SeqCst);
    }
}
