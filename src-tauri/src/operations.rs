use crate::{database::Database, model::*, security};
use std::{
    fs::{self, File, OpenOptions},
    io::Read,
    path::{Path, PathBuf},
};

#[cfg(windows)]
fn copy_staged(source: &Path, destination: &Path, _input: &mut File) -> Result<u64> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        CopyFileExW, COPY_FILE_COPY_SYMLINK, COPY_FILE_FAIL_IF_EXISTS,
    };
    let source = fs::canonicalize(source)?;
    let destination = fs::canonicalize(
        destination
            .parent()
            .ok_or_else(|| invalid("Missing destination parent"))?,
    )?
    .join(
        destination
            .file_name()
            .ok_or_else(|| invalid("Missing destination filename"))?,
    );
    let source_wide: Vec<u16> = source
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let destination_wide: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // Both buffers remain alive for the synchronous call; no callbacks or user pointers are passed.
    let copied = unsafe {
        CopyFileExW(
            source_wide.as_ptr(),
            destination_wide.as_ptr(),
            None,
            std::ptr::null(),
            std::ptr::null_mut(),
            COPY_FILE_FAIL_IF_EXISTS | COPY_FILE_COPY_SYMLINK,
        )
    };
    if copied == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    security::entry_guard(&fs::symlink_metadata(&destination)?)?;
    OpenOptions::new()
        .write(true)
        .open(&destination)?
        .sync_all()?;
    Ok(fs::metadata(destination)?.len())
}
#[cfg(not(windows))]
fn copy_staged(_source: &Path, destination: &Path, input: &mut File) -> Result<u64> {
    use std::io::Write;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?;
    let count = std::io::copy(input, &mut output)?;
    output.flush()?;
    output.sync_all()?;
    Ok(count)
}

pub fn hash_file(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    hash_reader(&mut file)
}
fn move_source_to_stage(source: &Path, staged: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_WRITE_THROUGH};
        let source = fs::canonicalize(source)?;
        let staged = fs::canonicalize(
            staged
                .parent()
                .ok_or_else(|| invalid("Missing staging parent"))?,
        )?
        .join(
            staged
                .file_name()
                .ok_or_else(|| invalid("Missing staging filename"))?,
        );
        let source: Vec<u16> = source
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let staged: Vec<u16> = staged
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        // Omitting REPLACE_EXISTING makes staging atomic without overwriting a racing file.
        if unsafe { MoveFileExW(source.as_ptr(), staged.as_ptr(), MOVEFILE_WRITE_THROUGH) } == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
    }
    #[cfg(not(windows))]
    {
        fs::rename(source, staged)?;
    }
    Ok(())
}
fn hash_reader(file: &mut File) -> Result<String> {
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0u8; 65536];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}
pub fn metadata(path: &Path) -> Result<(u64, u64)> {
    let m = fs::metadata(path)?;
    if !m.is_file() {
        return Err(invalid("Not a regular file"));
    }
    Ok((
        m.len(),
        m.modified()?
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    ))
}
pub fn unique(path: &Path, reserved: &std::collections::HashSet<PathBuf>) -> Result<PathBuf> {
    if !path.exists() && !reserved.contains(path) {
        return Ok(path.to_owned());
    }
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let ext = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    for n in 1..10000 {
        let candidate = path.with_file_name(format!("{stem} ({n}){ext}"));
        if !candidate.exists() && !reserved.contains(&candidate) {
            return Ok(candidate);
        }
    }
    Err(invalid("No available filename"))
}
pub fn staging(op: &Operation, undo: bool) -> PathBuf {
    let source = if undo { &op.destination } else { &op.source };
    Path::new(source).with_file_name(format!(
        ".filepilot-{}-{}",
        op.id,
        if undo { "undo" } else { "source" }
    ))
}
fn transfer(
    source: &Path,
    destination: &Path,
    staged: &Path,
    expected_hash: &str,
    size: u64,
) -> Result<()> {
    security::guard(source)?;
    security::guard(destination)?;
    security::guard(staged)?;
    security::safe_name(
        &destination
            .file_name()
            .unwrap_or_default()
            .to_string_lossy(),
    )?;
    if destination.exists() || staged.exists() {
        return Err(invalid(
            "Destination or staging file is occupied; no overwrite allowed",
        ));
    }
    if metadata(source)?.0 != size || hash_file(source)? != expected_hash {
        return Err(invalid("Source changed after preview"));
    }
    let parent = destination
        .parent()
        .ok_or_else(|| invalid("Missing destination folder"))?;
    fs::create_dir_all(parent)?;
    security::guard(parent)?;
    // Staging detaches the original pathname. A new file at that pathname is never deleted.
    move_source_to_stage(source, staged)?;
    let result = (|| -> Result<()> {
        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.share_mode(1 | 4);
        }
        let mut input = options.open(staged)?;
        if input.metadata()?.len() != size || hash_reader(&mut input)? != expected_hash {
            return Err(invalid("Source changed while being acquired"));
        }
        use std::io::{Seek, SeekFrom};
        input.seek(SeekFrom::Start(0))?;
        // Copy even on one volume: source and destination must be independently lockable.
        {
            let temporary = parent.join(format!(".filepilot-{}.copy", id()));
            let copied = (|| -> Result<()> {
                let count = copy_staged(staged, &temporary, &mut input)?;
                if count != size || hash_file(&temporary)? != expected_hash {
                    return Err(invalid("Copy integrity check failed"));
                }
                fs::hard_link(&temporary, destination)?;
                Ok(())
            })();
            if temporary.exists() {
                let _cleanup = fs::remove_file(&temporary);
            }
            copied?;
        }
        let mut destination_options = OpenOptions::new();
        destination_options.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            destination_options.share_mode(1);
        }
        let mut verified_destination = destination_options.open(destination)?;
        if hash_reader(&mut verified_destination)? != expected_hash {
            return Err(invalid(
                "Destination integrity check failed; source retained",
            ));
        }
        // Keep the exclusive Windows handle until deletion to exclude competing writes.
        fs::remove_file(staged)?;
        Ok(())
    })();
    if result.is_err() && staged.exists() && !source.exists() {
        // Rollback cannot overwrite a newly created source file.
        if fs::hard_link(staged, source).is_ok() {
            let _cleanup = fs::remove_file(staged);
        }
    }
    result
}
pub fn execute(db: &Database, proposal: &Proposal) -> Result<Operation> {
    if proposal.status != "ready" {
        return Err(invalid("Only ready proposals can be executed"));
    }
    let mut op = Operation {
        action: proposal.action.clone(),
        id: id(),
        rule_id: proposal.rule_id.clone(),
        rule_name: proposal.rule_name.clone(),
        source: proposal.source.clone(),
        destination: proposal.destination.clone(),
        size: proposal.size,
        hash: proposal.hash.clone(),
        modified: proposal.modified,
        time: now(),
        status: "prepared".into(),
        error: None,
    };
    db.record(&op)?;
    op.status = "copying".into();
    db.record(&op)?;
    match transfer(
        Path::new(&op.source),
        Path::new(&op.destination),
        &staging(&op, false),
        &op.hash,
        op.size,
    ) {
        Ok(()) => {
            op.status = "completed".into();
            op.modified = metadata(Path::new(&op.destination))?.1;
        }
        Err(e) => {
            op.status = if staging(&op, false).exists() || Path::new(&op.destination).exists() {
                "reconcile"
            } else {
                "failed"
            }
            .into();
            op.error = Some(e.to_string());
        }
    }
    db.record(&op)?;
    Ok(op)
}
pub fn undo_eligible(db: &Database, op_id: &str) -> Result<Operation> {
    let op = db
        .history()?
        .into_iter()
        .find(|o| o.id == op_id)
        .ok_or_else(|| invalid("Unknown operation"))?;
    if op.status != "completed" {
        return Err(invalid("Operation cannot be undone"));
    }
    security::guard(Path::new(&op.destination))?;
    if metadata(Path::new(&op.destination))? != (op.size, op.modified)
        || hash_file(Path::new(&op.destination))? != op.hash
    {
        return Err(invalid("Destination changed; undo refused"));
    }
    if Path::new(&op.source).exists() {
        return Err(invalid("Original path is occupied; undo refused"));
    }
    Ok(op)
}
pub fn undo(db: &Database, op_id: &str) -> Result<Operation> {
    let mut op = undo_eligible(db, op_id)?;
    op.status = "undoing".into();
    db.record(&op)?;
    match transfer(
        Path::new(&op.destination),
        Path::new(&op.source),
        &staging(&op, true),
        &op.hash,
        op.size,
    ) {
        Ok(()) => {
            op.status = "undone".into();
            op.modified = metadata(Path::new(&op.source))?.1;
            op.error = None;
        }
        Err(e) => {
            op.status = "reconcile".into();
            op.error = Some(e.to_string());
        }
    }
    db.record(&op)?;
    Ok(op)
}
#[cfg(test)]
mod tests {
    #[test]
    #[cfg(windows)]
    fn staging_never_overwrites_an_existing_file() {
        let temporary = tempfile::tempdir().unwrap();
        let source = temporary.path().join("source.pdf");
        let staged = temporary.path().join("stage");
        std::fs::write(&source, b"source").unwrap();
        std::fs::write(&staged, b"existing").unwrap();
        assert!(super::move_source_to_stage(&source, &staged).is_err());
        assert_eq!(std::fs::read(source).unwrap(), b"source");
        assert_eq!(std::fs::read(staged).unwrap(), b"existing");
    }
}
