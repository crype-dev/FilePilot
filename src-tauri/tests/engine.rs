use filepilot_lib::{
    database::Database, model::*, operations, rules, scanner, security, service::Service,
};
use std::{fs, path::Path, sync::atomic::AtomicBool};
fn setup() -> (tempfile::TempDir, std::sync::Arc<Service>, Folder, Rule) {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    fs::create_dir(&source).unwrap();
    let service = Service::open(&temp.path().join("test.sqlite3")).unwrap();
    let folder = service
        .save_folder(Folder {
            id: String::new(),
            name: "Test".into(),
            path: source.to_string_lossy().into(),
            recursive: false,
            enabled: true,
        })
        .unwrap();
    let rule = service
        .save_rule(Rule {
            id: String::new(),
            name: "PDF".into(),
            description: String::new(),
            enabled: true,
            priority: 1,
            sources: vec![folder.id.clone()],
            conditions: Conditions {
                extensions: vec!["pdf".into()],
                ..Default::default()
            },
            exclusions: vec![],
            action: Action::Move,
            destination: temp.path().join("destination").to_string_lossy().into(),
            rename_template: "{name}.{ext}".into(),
            conflict: Conflict::Unique,
            created_at: 0,
            updated_at: 0,
        })
        .unwrap();
    (temp, service, folder, rule)
}
fn scan(service: &Service, folder: &Folder) -> Scan {
    service.scan(&folder.id, &[], |_| {}).unwrap()
}
fn fixture(folder: &Folder, name: &str) {
    fs::write(Path::new(&folder.path).join(name), b"original document").unwrap();
}
#[test]
fn simulation_never_changes_files() {
    let (_t, s, f, _) = setup();
    fixture(&f, "a.PDF");
    let preview = scan(&s, &f);
    assert_eq!(preview.scanned, 1);
    assert_eq!(preview.proposals.len(), 1);
    assert!(Path::new(&f.path).join("a.PDF").exists());
    assert!(!Path::new(&preview.proposals[0].destination).exists());
    assert!(s.snapshot().unwrap().history.is_empty());
}
#[test]
fn move_and_undo_are_real_and_persistent() {
    let (t, s, f, _) = setup();
    fixture(&f, "épreuve.pdf");
    let p = scan(&s, &f);
    let ops = s
        .execute(&p.token, &[p.proposals[0].id.clone()], |_, _| {})
        .unwrap();
    assert_eq!(ops[0].status, "completed");
    assert!(!Path::new(&ops[0].source).exists());
    assert_eq!(fs::read(&ops[0].destination).unwrap(), b"original document");
    drop(s);
    let db = Database::open(&t.path().join("test.sqlite3")).unwrap();
    assert_eq!(db.history().unwrap().len(), 1);
    let undo = operations::undo(&db, &ops[0].id).unwrap();
    assert_eq!(undo.status, "undone");
    assert!(Path::new(&undo.source).exists());
    assert!(!Path::new(&undo.destination).exists());
    assert!(db
        .suppresses(&undo.source, undo.size, undo.modified)
        .unwrap());
}
#[test]
fn undo_refuses_modified_destination() {
    let (_t, s, f, _) = setup();
    fixture(&f, "a.pdf");
    let p = scan(&s, &f);
    let op = s
        .execute(&p.token, &[p.proposals[0].id.clone()], |_, _| {})
        .unwrap()
        .remove(0);
    fs::write(&op.destination, b"external edit").unwrap();
    let db = s.db.lock().unwrap();
    assert!(operations::undo(&db, &op.id).is_err());
    assert_eq!(fs::read(&op.destination).unwrap(), b"external edit");
}
#[test]
fn undo_refuses_occupied_source() {
    let (_t, s, f, _) = setup();
    fixture(&f, "a.pdf");
    let p = scan(&s, &f);
    let op = s
        .execute(&p.token, &[p.proposals[0].id.clone()], |_, _| {})
        .unwrap()
        .remove(0);
    fs::write(&op.source, b"new file").unwrap();
    assert!(operations::undo(&s.db.lock().unwrap(), &op.id).is_err());
    assert_eq!(fs::read(&op.source).unwrap(), b"new file");
}
#[test]
fn unique_conflict_preserves_both_files() {
    let (_t, s, f, r) = setup();
    fixture(&f, "a.pdf");
    fs::create_dir(&r.destination).unwrap();
    fs::write(Path::new(&r.destination).join("a.pdf"), b"existing").unwrap();
    let p = scan(&s, &f);
    assert!(p.proposals[0].destination.ends_with("a (1).pdf"));
    let op = s
        .execute(&p.token, &[p.proposals[0].id.clone()], |_, _| {})
        .unwrap()
        .remove(0);
    assert_eq!(op.status, "completed");
    assert_eq!(
        fs::read(Path::new(&r.destination).join("a.pdf")).unwrap(),
        b"existing"
    );
}
#[test]
fn race_conflict_never_overwrites() {
    let (_t, s, f, _) = setup();
    fixture(&f, "a.pdf");
    let p = scan(&s, &f);
    let target = Path::new(&p.proposals[0].destination);
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::write(target, b"racing writer").unwrap();
    let op = s
        .execute(&p.token, &[p.proposals[0].id.clone()], |_, _| {})
        .unwrap()
        .remove(0);
    assert_ne!(op.status, "completed");
    assert_eq!(fs::read(target).unwrap(), b"racing writer");
    assert!(Path::new(&op.source).exists());
}
#[test]
fn skip_conflict_is_not_executable() {
    let (_t, s, f, mut r) = setup();
    r.conflict = Conflict::Skip;
    s.save_rule(r.clone()).unwrap();
    fixture(&f, "a.pdf");
    fs::create_dir(&r.destination).unwrap();
    fs::write(Path::new(&r.destination).join("a.pdf"), b"existing").unwrap();
    let p = scan(&s, &f);
    assert_eq!(p.proposals[0].status, "skipped");
    assert!(s
        .execute(&p.token, &[p.proposals[0].id.clone()], |_, _| {})
        .is_err());
}
#[test]
fn changed_source_after_preview_is_retained() {
    let (_t, s, f, _) = setup();
    fixture(&f, "a.pdf");
    let p = scan(&s, &f);
    fs::write(&p.proposals[0].source, b"changed").unwrap();
    let op = s
        .execute(&p.token, &[p.proposals[0].id.clone()], |_, _| {})
        .unwrap()
        .remove(0);
    assert_eq!(op.status, "failed");
    assert_eq!(fs::read(&op.source).unwrap(), b"changed");
}
#[test]
fn missing_source_is_recorded() {
    let (_t, s, f, _) = setup();
    fixture(&f, "a.pdf");
    let p = scan(&s, &f);
    fs::remove_file(&p.proposals[0].source).unwrap();
    let op = s
        .execute(&p.token, &[p.proposals[0].id.clone()], |_, _| {})
        .unwrap()
        .remove(0);
    assert_eq!(op.status, "failed");
    assert!(op.error.is_some());
}
#[test]
fn disabled_rules_and_temporary_files_are_excluded() {
    let (_t, s, f, mut r) = setup();
    fixture(&f, "a.pdf");
    fixture(&f, "b.crdownload");
    r.enabled = false;
    s.save_rule(r).unwrap();
    let p = scan(&s, &f);
    assert_eq!(p.scanned, 1);
    assert!(p.proposals.is_empty());
}
#[test]
fn conditions_use_and_and_values_use_or() {
    let (_t, _s, f, mut r) = setup();
    r.conditions.extensions = vec!["pdf".into(), "txt".into()];
    r.conditions.contains = vec!["invoice".into(), "receipt".into()];
    r.conditions.min_size = Some(10);
    assert!(rules::matches(
        &r,
        Path::new("invoice.PDF"),
        Path::new(&f.path),
        20,
        now()
    ));
    assert!(!rules::matches(
        &r,
        Path::new("holiday.pdf"),
        Path::new(&f.path),
        20,
        now()
    ));
    assert!(!rules::matches(
        &r,
        Path::new("invoice.pdf"),
        Path::new(&f.path),
        2,
        now()
    ));
}
#[test]
fn glob_exclusions_and_relative_paths() {
    let (_t, _s, f, mut r) = setup();
    r.conditions.globs = vec!["*.pdf".into()];
    r.conditions.relative_globs = vec!["nested/*".into()];
    let root = Path::new(&f.path);
    assert!(rules::matches(
        &r,
        &root.join("nested/a.pdf"),
        root,
        20,
        now()
    ));
    r.exclusions = vec!["nested/*".into()];
    assert!(!rules::matches(
        &r,
        &root.join("nested/a.pdf"),
        root,
        20,
        now()
    ));
}
#[test]
fn first_priority_rule_wins() {
    let (_t, s, f, mut r) = setup();
    fixture(&f, "a.pdf");
    r.id = String::new();
    r.priority = -1;
    r.name = "First".into();
    s.save_rule(r).unwrap();
    assert_eq!(scan(&s, &f).proposals[0].rule_name, "First");
}
#[test]
fn destinations_inside_sources_are_rejected() {
    let (_t, s, f, mut r) = setup();
    r.destination = Path::new(&f.path).join("nested").to_string_lossy().into();
    assert!(s.save_rule(r).is_err());
}
#[cfg(windows)]
#[test]
fn destination_case_alias_cannot_bypass_loop_guard() {
    let (_t, s, f, mut r) = setup();
    r.destination = format!("{}\\new", f.path.to_uppercase());
    assert!(s.save_rule(r).is_err());
}
#[test]
fn overlapping_sources_are_rejected() {
    let (_t, s, f, _) = setup();
    let path = Path::new(&f.path).join("nested");
    fs::create_dir(&path).unwrap();
    let mut next = f;
    next.id = String::new();
    next.path = path.to_string_lossy().into();
    assert!(s.save_folder(next).is_err());
}
#[test]
fn traversal_and_windows_names_are_rejected() {
    assert!(security::guard(Path::new("relative/file")).is_err());
    for name in ["../a", "CON.pdf", "LPT1", "a:b", "hello.", "a\\b"] {
        assert!(security::safe_name(name).is_err(), "{name}");
    }
    assert!(security::safe_name("résumé 2026.pdf").is_ok());
}
#[test]
fn rename_templates_validate_and_execute() {
    let (_t, s, f, mut r) = setup();
    r.action = Action::Rename;
    r.rename_template = "{name}-{counter}.{ext}".into();
    s.save_rule(r).unwrap();
    fixture(&f, "a.pdf");
    let p = scan(&s, &f);
    let op = s
        .execute(&p.token, &[p.proposals[0].id.clone()], |_, _| {})
        .unwrap()
        .remove(0);
    assert_eq!(op.status, "completed");
    assert!(scan(&s, &f).proposals.is_empty());
    assert!(rules::render_name("{unknown}", Path::new("a.pdf"), 1).is_err());
}
#[test]
fn changed_rule_invalidates_preview() {
    let (_t, s, f, mut r) = setup();
    fixture(&f, "a.pdf");
    let p = scan(&s, &f);
    r.enabled = false;
    s.save_rule(r).unwrap();
    assert!(s
        .execute(&p.token, &[p.proposals[0].id.clone()], |_, _| {})
        .is_err());
}
#[test]
fn interrupted_operations_enter_safe_mode_without_deletion() {
    let (t, s, f, _) = setup();
    fixture(&f, "a.pdf");
    let op = Operation {
        action: Action::Move,
        id: id(),
        rule_id: String::new(),
        rule_name: String::new(),
        source: Path::new(&f.path).join("a.pdf").to_string_lossy().into(),
        destination: t.path().join("destination/a.pdf").to_string_lossy().into(),
        size: 17,
        hash: String::new(),
        modified: 0,
        time: now(),
        status: "copying".into(),
        error: None,
    };
    s.db.lock().unwrap().record(&op).unwrap();
    drop(s);
    let reopened = Service::open(&t.path().join("test.sqlite3")).unwrap();
    let state = reopened.snapshot().unwrap();
    assert!(state.safe_mode);
    assert_eq!(state.history[0].status, "reconcile");
    assert!(Path::new(&op.source).exists());
}
#[test]
fn sqlite_migration_and_settings_persist() {
    let (t, s, _, _) = setup();
    let settings = Settings {
        language: "en".into(),
        ..Default::default()
    };
    s.save_settings(settings).unwrap();
    drop(s);
    let s = Service::open(&t.path().join("test.sqlite3")).unwrap();
    assert_eq!(s.snapshot().unwrap().settings.language, "en");
    assert_eq!(
        s.db.lock()
            .unwrap()
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get::<_, u32>(0))
            .unwrap(),
        1
    );
}
#[test]
fn malformed_json_and_unknown_fields_are_rejected() {
    assert!(serde_json::from_str::<Rule>("{}").is_err());
    let (_t, _s, _, r) = setup();
    let mut json = serde_json::to_value(r).unwrap();
    json["shell_command"] = serde_json::json!("bad");
    assert!(serde_json::from_value::<Rule>(json).is_err());
}
#[test]
fn cancellation_stops_scan_without_modifications() {
    let (_t, s, f, _) = setup();
    fixture(&f, "a.pdf");
    let cancelled = AtomicBool::new(true);
    let result = scanner::scan(&s.db.lock().unwrap(), &f, &[], &cancelled, |_| {}).unwrap();
    assert_eq!(result.scanned, 0);
    assert!(Path::new(&f.path).join("a.pdf").exists());
}
#[test]
fn invalid_settings_are_rejected() {
    let (_t, s, _, _) = setup();
    let settings = Settings {
        reconciliation_seconds: 1,
        ..Default::default()
    };
    assert!(s.save_settings(settings).is_err());
}
#[cfg(windows)]
#[test]
fn locked_source_is_not_moved() {
    use std::os::windows::fs::OpenOptionsExt;
    let (_t, s, f, _) = setup();
    fixture(&f, "a.pdf");
    let p = scan(&s, &f);
    let _lock = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&p.proposals[0].source)
        .unwrap();
    let op = s
        .execute(&p.token, &[p.proposals[0].id.clone()], |_, _| {})
        .unwrap()
        .remove(0);
    assert_eq!(op.status, "failed");
    assert!(Path::new(&op.source).exists());
}
#[cfg(windows)]
#[test]
fn junctions_are_rejected() {
    let (t, _s, f, _) = setup();
    let link = t.path().join("junction");
    let status = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&link)
        .arg(&f.path)
        .status()
        .unwrap();
    assert!(status.success());
    assert!(security::directory(&link).is_err());
}
#[test]
#[ignore = "Explicit isolated 10,000-file performance run"]
fn benchmark_10000_files() {
    let (_t, s, f, _) = setup();
    let started = std::time::Instant::now();
    for i in 0..10000 {
        fixture(&f, &format!("document-{i}.pdf"));
    }
    let generated = started.elapsed();
    let result = scan(&s, &f);
    assert_eq!(result.scanned, 10000);
    assert_eq!(result.proposals.len(), 10000);
    println!(
        "BENCH files=10000 generation_ms={} scan_ms={} proposal_bytes={}",
        generated.as_millis(),
        result.elapsed_ms,
        serde_json::to_vec(&result.proposals).unwrap().len()
    );
}
#[cfg(windows)]
#[test]
fn watcher_detects_stable_new_files_once_and_respects_pause() {
    use filepilot_lib::watcher::{start_worker, WorkerEvent};
    let (_t, s, f, r) = setup();
    fixture(&f, "existing.pdf");
    fixture(&f, "existing.txt");
    let mut settings = s.snapshot().unwrap().settings;
    settings.automatic = true;
    settings.stabilization_seconds = 2;
    s.save_settings(settings.clone()).unwrap();
    let (send, receive) = std::sync::mpsc::channel();
    let worker = start_worker(
        s.clone(),
        std::sync::Arc::new(move |event| {
            send.send(event).unwrap();
        }),
    );
    assert!(matches!(
        receive
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap(),
        WorkerEvent::Ready
    ));
    fixture(&f, "new.pdf");
    assert!(matches!(
        receive
            .recv_timeout(std::time::Duration::from_secs(15))
            .unwrap(),
        WorkerEvent::Changed(1)
    ));
    assert!(Path::new(&r.destination).join("new.pdf").exists());
    assert!(Path::new(&f.path).join("existing.pdf").exists());
    let mut text_rule = r.clone();
    text_rule.id = String::new();
    text_rule.name = "Text".into();
    text_rule.conditions.extensions = vec!["txt".into()];
    s.save_rule(text_rule).unwrap();
    fixture(&f, "new.txt");
    assert!(matches!(
        receive
            .recv_timeout(std::time::Duration::from_secs(15))
            .unwrap(),
        WorkerEvent::Changed(1)
    ));
    assert!(Path::new(&f.path).join("existing.txt").exists());
    assert!(Path::new(&r.destination).join("new.txt").exists());
    settings.automatic = false;
    s.save_settings(settings).unwrap();
    fixture(&f, "paused.pdf");
    assert!(receive
        .recv_timeout(std::time::Duration::from_secs(3))
        .is_err());
    assert!(Path::new(&f.path).join("paused.pdf").exists());
    assert_eq!(s.snapshot().unwrap().history.len(), 2);
    drop(worker);
}
#[test]
fn batch_undo_preflights_all_files() {
    let (_t, s, f, _) = setup();
    fixture(&f, "a.pdf");
    fixture(&f, "b.pdf");
    let preview = scan(&s, &f);
    let ops = s
        .execute(
            &preview.token,
            &preview
                .proposals
                .iter()
                .map(|p| p.id.clone())
                .collect::<Vec<_>>(),
            |_, _| {},
        )
        .unwrap();
    fs::write(&ops[0].destination, b"external edit").unwrap();
    assert!(s
        .undo_selection(&ops.iter().map(|o| o.id.clone()).collect::<Vec<_>>())
        .is_err());
    assert!(!Path::new(&ops[1].source).exists());
    assert!(Path::new(&ops[1].destination).exists());
}
#[test]
fn batch_undo_restores_selected_files() {
    let (_t, s, f, _) = setup();
    fixture(&f, "a.pdf");
    fixture(&f, "b.pdf");
    let preview = scan(&s, &f);
    let ops = s
        .execute(
            &preview.token,
            &preview
                .proposals
                .iter()
                .map(|p| p.id.clone())
                .collect::<Vec<_>>(),
            |_, _| {},
        )
        .unwrap();
    let restored = s
        .undo_selection(&ops.iter().map(|o| o.id.clone()).collect::<Vec<_>>())
        .unwrap();
    assert_eq!(restored.len(), 2);
    assert!(restored
        .iter()
        .all(|o| o.status == "undone" && Path::new(&o.source).exists()));
}
#[test]
fn purge_preserves_files_and_suppression() {
    let (_t, s, f, _) = setup();
    fixture(&f, "a.pdf");
    let preview = scan(&s, &f);
    let op = s
        .execute(
            &preview.token,
            &[preview.proposals[0].id.clone()],
            |_, _| {},
        )
        .unwrap()
        .remove(0);
    let db = s.db.lock().unwrap();
    let restored = operations::undo(&db, &op.id).unwrap();
    assert_eq!(db.purge_history(now() + 1).unwrap(), 1);
    assert!(db.history().unwrap().is_empty());
    assert!(Path::new(&restored.source).exists());
    assert!(db
        .suppresses(&restored.source, restored.size, restored.modified)
        .unwrap());
    assert!(operations::undo(&db, &op.id).is_err());
}
#[cfg(windows)]
#[test]
fn move_and_undo_preserve_ntfs_download_marker() {
    let (_t, s, f, _) = setup();
    fixture(&f, "a.pdf");
    let source = Path::new(&f.path).join("a.pdf");
    let marker = b"[ZoneTransfer]\r\nZoneId=3\r\n";
    fs::write(format!("{}:Zone.Identifier", source.display()), marker).unwrap();
    let preview = scan(&s, &f);
    let op = s
        .execute(
            &preview.token,
            &[preview.proposals[0].id.clone()],
            |_, _| {},
        )
        .unwrap()
        .remove(0);
    assert_eq!(op.status, "completed");
    assert_eq!(
        fs::read(format!("{}:Zone.Identifier", op.destination)).unwrap(),
        marker
    );
    let db = s.db.lock().unwrap();
    let restored = operations::undo(&db, &op.id).unwrap();
    assert_eq!(restored.status, "undone");
    assert_eq!(
        fs::read(format!("{}:Zone.Identifier", restored.source)).unwrap(),
        marker
    );
}
#[cfg(windows)]
#[test]
fn system_paths_and_short_aliases_are_protected() {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetShortPathNameW;
    for variable in ["WINDIR", "PROGRAMFILES"] {
        let root = std::env::var_os(variable).unwrap();
        assert!(security::guard(Path::new(&root)).is_err());
        let wide: Vec<u16> = root.encode_wide().chain(std::iter::once(0)).collect();
        let mut output = vec![0u16; 32768];
        let count =
            unsafe { GetShortPathNameW(wide.as_ptr(), output.as_mut_ptr(), output.len() as u32) };
        assert!(count > 0);
        let short = String::from_utf16(&output[..count as usize]).unwrap();
        assert!(security::guard(Path::new(&short)).is_err());
    }
}
#[test]
fn shutdown_rejects_new_operations_without_touching_files() {
    use std::sync::atomic::Ordering;
    let (_t, s, f, _) = setup();
    fixture(&f, "a.pdf");
    let preview = scan(&s, &f);
    s.stopping.store(true, Ordering::SeqCst);
    assert!(s
        .execute(
            &preview.token,
            &[preview.proposals[0].id.clone()],
            |_, _| {}
        )
        .is_err());
    assert!(Path::new(&f.path).join("a.pdf").exists());
    assert!(s.snapshot().unwrap().history.is_empty());
    assert!(!s.busy.load(Ordering::SeqCst));
}
#[test]
fn lifecycle_preferences_are_available_without_database_lock() {
    use std::sync::atomic::Ordering;
    let (_t, s, _, _) = setup();
    let mut settings = s.snapshot().unwrap().settings;
    settings.close_to_tray = false;
    settings.automatic = true;
    s.save_settings(settings.clone()).unwrap();
    let db = s.db.lock().unwrap();
    assert!(!s.close_to_tray.load(Ordering::SeqCst));
    assert!(s.automatic.load(Ordering::SeqCst));
    drop(db);
    settings.automatic = false;
    s.save_settings(settings).unwrap();
    assert!(s.cancel.load(Ordering::SeqCst));
    assert!(!s.automatic.load(Ordering::SeqCst));
}
