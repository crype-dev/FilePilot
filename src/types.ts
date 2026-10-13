export interface Folder {
  id: string;
  name: string;
  path: string;
  recursive: boolean;
  enabled: boolean;
}
export interface Conditions {
  extensions: string[];
  names: string[];
  contains: string[];
  globs: string[];
  min_size: number | null;
  max_size: number | null;
  min_age_days: number | null;
  max_age_days: number | null;
  modified_after: number | null;
  modified_before: number | null;
  relative_globs: string[];
}
export interface Rule {
  id: string;
  name: string;
  description: string;
  enabled: boolean;
  priority: number;
  sources: string[];
  conditions: Conditions;
  exclusions: string[];
  action: "move" | "rename";
  destination: string;
  rename_template: string;
  conflict: "skip" | "unique";
  created_at: number;
  updated_at: number;
}
export interface Settings {
  history_days: number;
  language: "fr" | "en";
  theme: "light" | "dark" | "system";
  automatic: boolean;
  close_to_tray: boolean;
  notifications: boolean;
  stabilization_seconds: number;
  reconciliation_seconds: number;
  scan_on_start: boolean;
  onboarding_done: boolean;
  reduced_motion: boolean;
  update_checks: boolean;
}
export interface Operation {
  action: "move" | "rename";
  id: string;
  rule_id: string;
  rule_name: string;
  source: string;
  destination: string;
  size: number;
  hash: string;
  modified: number;
  time: number;
  status: string;
  error: string | null;
}
export interface Proposal {
  action: "move" | "rename";
  id: string;
  source: string;
  destination: string;
  rule_id: string;
  rule_name: string;
  hash: string;
  size: number;
  modified: number;
  status: string;
  reason: string;
}
export interface Scan {
  token: string;
  proposals: Proposal[];
  scanned: number;
  elapsed_ms: number;
  errors: string[];
}
export interface Snapshot {
  folders: Folder[];
  rules: Rule[];
  settings: Settings;
  history: Operation[];
  busy: boolean;
  safe_mode: boolean;
}
export type Page =
  "dashboard" | "folders" | "rules" | "organize" | "history" | "settings";
