import { invoke, isTauri } from "@tauri-apps/api/core";
import { create } from "zustand";
import type { Page, Rule } from "./types";
export function api<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (!isTauri())
    return Promise.reject(
      new Error("FilePilot requires the Windows desktop application."),
    );
  return invoke<T>(command, args);
}
export const useNavigation = create<{
  page: Page;
  setPage: (page: Page) => void;
}>((set) => ({ page: "dashboard", setPage: (page) => set({ page }) }));
export const baseConditions = {
  extensions: [],
  names: [],
  contains: [],
  globs: [],
  min_size: null,
  max_size: null,
  min_age_days: null,
  max_age_days: null,
  modified_after: null,
  modified_before: null,
  relative_globs: [],
};
export const newRule = (): Rule => ({
  id: "",
  name: "",
  description: "",
  enabled: false,
  priority: 10,
  sources: [],
  conditions: { ...baseConditions },
  exclusions: [],
  action: "move",
  destination: "",
  rename_template: "{name}.{ext}",
  conflict: "unique",
  created_at: 0,
  updated_at: 0,
});
export const templates: Record<string, string[]> = {
  documents: ["pdf", "doc", "docx", "txt", "rtf", "odt"],
  images: ["jpg", "jpeg", "png", "gif", "webp", "bmp", "svg"],
  videos: ["mp4", "mkv", "mov", "avi", "webm"],
  audio: ["mp3", "wav", "flac", "aac", "ogg"],
  archives: ["zip", "rar", "7z", "tar", "gz"],
  code: ["js", "ts", "py", "java", "html", "css", "json"],
};
export const basename = (path: string) => path.split(/[\\/]/).pop() || path;
export const bytes = (n: number) =>
  n < 1024
    ? `${n} B`
    : n < 1048576
      ? `${(n / 1024).toFixed(1)} KB`
      : n < 1073741824
        ? `${(n / 1048576).toFixed(1)} MB`
        : `${(n / 1073741824).toFixed(1)} GB`;
export const errorMessage = (e: unknown) =>
  e instanceof Error ? e.message : String(e);
