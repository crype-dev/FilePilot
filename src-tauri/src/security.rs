use crate::model::*;
use std::path::{Component, Path, PathBuf};

pub fn guard(path: &Path) -> Result<()> {
    if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(invalid("An absolute path without '..' is required"));
    }
    #[cfg(windows)]
    {
        let text = path.to_string_lossy().replace('/', "\\").to_lowercase();
        if text.starts_with("\\\\")
            || text.starts_with("\\?\\")
            || text.get(2..).unwrap_or("").contains(':')
        {
            return Err(invalid(
                "Network, device and alternate-stream paths are unsupported",
            ));
        }
        for key in ["WINDIR", "PROGRAMFILES", "PROGRAMFILES(X86)", "PROGRAMDATA"] {
            if let Some(root) = std::env::var_os(key) {
                let root = PathBuf::from(root);
                if inside(path, &root) {
                    return Err(invalid("Protected system path"));
                }
            }
        }
        for root in [
            "c:\\windows",
            "c:\\program files",
            "c:\\program files (x86)",
            "c:\\programdata",
            "c:\\$recycle.bin",
            "c:\\system volume information",
        ] {
            if text == root || text.starts_with(&format!("{root}\\")) {
                return Err(invalid("Protected system path"));
            }
        }
    }
    let mut ancestor = Some(path);
    while let Some(p) = ancestor {
        if let Ok(meta) = std::fs::symlink_metadata(p) {
            entry_guard(&meta)?;
        }
        ancestor = p.parent();
    }
    Ok(())
}
pub fn entry_guard(meta: &std::fs::Metadata) -> Result<()> {
    if meta.is_file() && meta.permissions().readonly() {
        return Err(invalid("Read-only files are excluded; original retained"));
    }
    if meta.file_type().is_symlink() {
        return Err(invalid("Symbolic links are unsupported"));
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & (0x400 | 0x1000 | 0x400000) != 0 {
            return Err(invalid(
                "Reparse points and offline/cloud placeholders are unsupported",
            ));
        }
    }
    Ok(())
}
pub fn directory(path: &Path) -> Result<PathBuf> {
    guard(path)?;
    if path.parent().is_none() {
        return Err(invalid("Drive roots cannot be watched"));
    }
    if !path.is_dir() {
        return Err(invalid("Folder does not exist or is inaccessible"));
    }
    Ok(std::fs::canonicalize(path)?)
}
pub fn inside(path: &Path, root: &Path) -> bool {
    let path = path_key(path);
    let root = path_key(root);
    path == root || path.starts_with(&(root + "/"))
}
pub fn path_key(path: &Path) -> String {
    // Resolve the existing ancestor, including Windows short-name aliases, before appending new components.
    let mut ancestor = path;
    let mut suffix = Vec::new();
    while !ancestor.exists() {
        if let (Some(name), Some(parent)) = (ancestor.file_name(), ancestor.parent()) {
            suffix.push(name.to_owned());
            ancestor = parent;
        } else {
            break;
        }
    }
    let mut resolved = std::fs::canonicalize(ancestor).unwrap_or_else(|_| ancestor.to_owned());
    for part in suffix.into_iter().rev() {
        resolved.push(part);
    }
    let resolved: PathBuf = resolved.components().collect();
    let text = resolved.to_string_lossy().replace('\\', "/");
    #[cfg(windows)]
    {
        text.trim_start_matches("//?/")
            .trim_end_matches('/')
            .to_lowercase()
    }
    #[cfg(not(windows))]
    {
        text.trim_end_matches('/').to_owned()
    }
}
pub fn safe_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > 240
        || name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
        || name.ends_with(['.', ' '])
    {
        return Err(invalid("Invalid Windows filename"));
    }
    let stem = name.split('.').next().unwrap_or("").to_uppercase();
    if ["CON", "PRN", "AUX", "NUL"].contains(&stem.as_str())
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.ends_with(['1', '2', '3', '4', '5', '6', '7', '8', '9']))
    {
        return Err(invalid("Reserved Windows filename"));
    }
    Ok(())
}
pub fn temporary(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    ["part", "partial", "tmp", "temp", "crdownload", "download"].contains(&ext.as_str())
        || path
            .file_name()
            .is_some_and(|s| s.to_string_lossy().starts_with(".filepilot-"))
}
