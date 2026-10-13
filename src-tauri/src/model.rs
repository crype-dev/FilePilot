use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Db(#[from] rusqlite::Error),
    #[error("{0}")]
    Json(#[from] serde_json::Error),
}
pub type Result<T> = std::result::Result<T, Error>;
pub fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
pub fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Folder {
    pub id: String,
    pub name: String,
    pub path: String,
    pub recursive: bool,
    pub enabled: bool,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Conditions {
    pub extensions: Vec<String>,
    pub names: Vec<String>,
    pub contains: Vec<String>,
    pub globs: Vec<String>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub min_age_days: Option<u64>,
    pub max_age_days: Option<u64>,
    pub modified_after: Option<u64>,
    pub modified_before: Option<u64>,
    pub relative_globs: Vec<String>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    #[default]
    Move,
    Rename,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Conflict {
    Skip,
    Unique,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub priority: i32,
    pub sources: Vec<String>,
    pub conditions: Conditions,
    pub exclusions: Vec<String>,
    pub action: Action,
    pub destination: String,
    pub rename_template: String,
    pub conflict: Conflict,
    pub created_at: u64,
    pub updated_at: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub language: String,
    pub theme: String,
    pub automatic: bool,
    pub close_to_tray: bool,
    pub notifications: bool,
    pub stabilization_seconds: u64,
    pub reconciliation_seconds: u64,
    pub scan_on_start: bool,
    pub onboarding_done: bool,
    pub reduced_motion: bool,
    pub update_checks: bool,
    pub history_days: u64,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            language: "fr".into(),
            theme: "system".into(),
            automatic: false,
            close_to_tray: true,
            notifications: true,
            stabilization_seconds: 5,
            reconciliation_seconds: 300,
            scan_on_start: false,
            onboarding_done: false,
            reduced_motion: false,
            update_checks: true,
            history_days: 0,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Operation {
    #[serde(default)]
    pub action: Action,
    pub id: String,
    pub rule_id: String,
    pub rule_name: String,
    pub source: String,
    pub destination: String,
    pub size: u64,
    pub hash: String,
    pub modified: u64,
    pub time: u64,
    pub status: String,
    pub error: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Proposal {
    pub action: Action,
    pub id: String,
    pub source: String,
    pub destination: String,
    pub rule_id: String,
    pub rule_name: String,
    pub hash: String,
    pub size: u64,
    pub modified: u64,
    pub status: String,
    pub reason: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct Scan {
    #[serde(skip_serializing)]
    pub inventory: Vec<ObservedFile>,
    pub token: String,
    pub proposals: Vec<Proposal>,
    pub scanned: usize,
    pub elapsed_ms: u128,
    pub errors: Vec<String>,
}
#[derive(Clone, Debug)]
pub struct ObservedFile {
    pub source: String,
    pub size: u64,
    pub modified_ns: u128,
}
#[derive(Clone, Debug, Serialize)]
pub struct Snapshot {
    pub folders: Vec<Folder>,
    pub rules: Vec<Rule>,
    pub settings: Settings,
    pub history: Vec<Operation>,
    pub busy: bool,
    pub safe_mode: bool,
}
