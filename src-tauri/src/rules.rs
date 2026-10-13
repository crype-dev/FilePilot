use crate::{model::*, security};
use std::path::Path;
pub fn validate(rule: &Rule, folders: &[Folder]) -> Result<()> {
    if rule.name.trim().is_empty()
        || rule.name.len() > 120
        || rule.sources.is_empty()
        || rule
            .sources
            .iter()
            .any(|s| !folders.iter().any(|f| &f.id == s))
    {
        return Err(invalid(
            "Rule name and registered source folders are required",
        ));
    }
    let c = &rule.conditions;
    if c.modified_after
        .zip(c.modified_before)
        .is_some_and(|(a, b)| a > b)
    {
        return Err(invalid("Invalid modification date range"));
    }
    for values in [
        &c.extensions,
        &c.names,
        &c.contains,
        &c.globs,
        &c.relative_globs,
        &rule.exclusions,
    ] {
        if values.len() > 100 || values.iter().any(|s| s.len() > 512) {
            return Err(invalid("Rule conditions exceed allowed limits"));
        }
    }
    if c.min_size.zip(c.max_size).is_some_and(|(a, b)| a > b)
        || c.min_age_days
            .zip(c.max_age_days)
            .is_some_and(|(a, b)| a > b)
        || c.min_age_days
            .into_iter()
            .chain(c.max_age_days)
            .any(|n| n > 365000)
    {
        return Err(invalid("Invalid size or age range"));
    }
    for glob in c
        .globs
        .iter()
        .chain(&c.relative_globs)
        .chain(&rule.exclusions)
    {
        globset::GlobBuilder::new(glob)
            .case_insensitive(true)
            .build()
            .map_err(|e| invalid(e.to_string()))?;
    }
    if rule.action == Action::Move {
        let destination = Path::new(&rule.destination);
        security::guard(destination)?;
        for folder in folders {
            if security::inside(destination, Path::new(&folder.path)) {
                return Err(invalid(
                    "Destination cannot be inside a registered source folder (loop protection)",
                ));
            }
        }
    } else {
        render_name(&rule.rename_template, Path::new("sample.pdf"), 1)?;
    }
    Ok(())
}
fn glob_matches(pattern: &str, name: &str) -> bool {
    globset::GlobBuilder::new(pattern)
        .case_insensitive(true)
        .build()
        .is_ok_and(|g| g.compile_matcher().is_match(name))
}
pub fn matches(rule: &Rule, path: &Path, root: &Path, size: u64, modified: u64) -> bool {
    if !rule.enabled {
        return false;
    }
    let c = &rule.conditions;
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    let ext = path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    let relative = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    let age = now().saturating_sub(modified) / 86400;
    (c.extensions.is_empty()
        || c.extensions
            .iter()
            .any(|e| e.trim_start_matches('.').eq_ignore_ascii_case(&ext)))
        && (c.names.is_empty() || c.names.iter().any(|n| n.eq_ignore_ascii_case(&name)))
        && (c.contains.is_empty() || c.contains.iter().any(|n| name.contains(&n.to_lowercase())))
        && (c.globs.is_empty() || c.globs.iter().any(|g| glob_matches(g, &name)))
        && (c.relative_globs.is_empty()
            || c.relative_globs.iter().any(|g| glob_matches(g, &relative)))
        && c.min_size.is_none_or(|n| size >= n)
        && c.max_size.is_none_or(|n| size <= n)
        && c.min_age_days.is_none_or(|n| age >= n)
        && c.max_age_days.is_none_or(|n| age <= n)
        && c.modified_after.is_none_or(|n| modified >= n)
        && c.modified_before.is_none_or(|n| modified <= n)
        && !rule
            .exclusions
            .iter()
            .any(|g| glob_matches(g, &relative) || glob_matches(g, &name))
}
pub fn render_name(template: &str, path: &Path, counter: usize) -> Result<String> {
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let ext = path.extension().unwrap_or_default().to_string_lossy();
    // UTC Unix day is explicit, stable and independent of locale.
    let result = template
        .replace("{name}", &stem)
        .replace("{ext}", &ext)
        .replace("{date}", &(now() / 86400).to_string())
        .replace("{counter}", &counter.to_string());
    if result.contains(['{', '}']) {
        return Err(invalid("Unknown rename variable"));
    }
    security::safe_name(&result)?;
    Ok(result)
}
