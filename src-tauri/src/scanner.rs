use crate::{database::Database, model::*, operations, rules, security};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

pub fn scan(
    db: &Database,
    folder: &Folder,
    selected: &[String],
    cancel: &AtomicBool,
    progress: impl Fn(usize),
) -> Result<Scan> {
    let started = std::time::Instant::now();
    let root = Path::new(&folder.path);
    security::directory(root)?;
    let mut all_rules = db.list::<Rule>("rule")?;
    all_rules.sort_by(|a, b| a.priority.cmp(&b.priority).then(a.id.cmp(&b.id)));
    let applicable: Vec<_> = all_rules
        .iter()
        .filter(|r| {
            r.enabled
                && r.sources.contains(&folder.id)
                && (selected.is_empty() || selected.contains(&r.id))
        })
        .collect();
    let folders = db.list::<Folder>("folder")?;
    for rule in &applicable {
        rules::validate(rule, &folders)?;
    }
    let mut result = Scan {
        inventory: vec![],
        token: id(),
        proposals: vec![],
        scanned: 0,
        elapsed_ms: 0,
        errors: vec![],
    };
    let mut dirs = vec![root.to_owned()];
    let mut reserved = HashSet::new();
    while let Some(dir) = dirs.pop() {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        if let Err(e) = security::guard(&dir) {
            result.errors.push(e.to_string());
            continue;
        }
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(e) => {
                result.errors.push(e.to_string());
                continue;
            }
        };
        for entry in entries {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    result.errors.push(e.to_string());
                    continue;
                }
            };
            let path = entry.path();
            let meta = match std::fs::symlink_metadata(&path) {
                Ok(m) => m,
                Err(e) => {
                    result.errors.push(e.to_string());
                    continue;
                }
            };
            // The parent was checked above; check the entry once rather than all ancestors per file.
            if security::entry_guard(&meta).is_err() {
                continue;
            }
            if meta.is_dir() {
                if folder.recursive {
                    dirs.push(path);
                }
                continue;
            }
            if !meta.is_file() || security::temporary(&path) {
                continue;
            }
            result.scanned += 1;
            if result.scanned.is_multiple_of(100) {
                progress(result.scanned);
            }
            let size = meta.len();
            result.inventory.push(ObservedFile {
                source: path.to_string_lossy().into(),
                size,
                modified_ns: meta
                    .modified()?
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos(),
            });
            let modified = meta
                .modified()?
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            if db.suppresses(&path.to_string_lossy(), size, modified)? {
                continue;
            }
            if let Some(rule) = applicable
                .iter()
                .find(|r| rules::matches(r, &path, root, size, modified))
            {
                let target = if rule.action == Action::Move {
                    PathBuf::from(&rule.destination).join(path.file_name().unwrap_or_default())
                } else {
                    path.with_file_name(rules::render_name(
                        &rule.rename_template,
                        &path,
                        result.scanned,
                    )?)
                };
                let mut status = "ready";
                let mut reason = String::new();
                let mut target = target;
                if target == path {
                    status = "skipped";
                    reason = "Already at destination".into();
                } else if target.exists() || reserved.contains(&target) {
                    if rule.conflict == Conflict::Skip {
                        status = "skipped";
                        reason = "Destination is occupied".into();
                    } else {
                        target = operations::unique(&target, &reserved)?;
                    }
                }
                let hash = match operations::hash_file(&path) {
                    Ok(h) => h,
                    Err(e) => {
                        status = "error";
                        reason = e.to_string();
                        String::new()
                    }
                };
                if status == "ready" {
                    reserved.insert(target.clone());
                }
                result.proposals.push(Proposal {
                    action: rule.action.clone(),
                    id: id(),
                    source: path.to_string_lossy().into(),
                    destination: target.to_string_lossy().into(),
                    rule_id: rule.id.clone(),
                    rule_name: rule.name.clone(),
                    hash,
                    size,
                    modified,
                    status: status.into(),
                    reason,
                });
            }
        }
    }
    result.elapsed_ms = started.elapsed().as_millis();
    Ok(result)
}
