import { useQuery, useQueryClient } from "@tanstack/react-query";
import { isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { save } from "@tauri-apps/plugin-dialog";
import {
  ArrowRight,
  ArrowRightLeft,
  Check,
  CodeXml,
  FolderOpen,
  History,
  LayoutDashboard,
  ListFilter,
  Pause,
  Play,
  Settings as SettingsIcon,
  ShieldCheck,
} from "lucide-react";
import { lazy, Suspense, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { HistoryView } from "./features/activity/HistoryView";
import { DashboardView } from "./features/dashboard/DashboardView";
import { FoldersView } from "./features/folders/FoldersView";
import { OrganizeView } from "./features/organize/OrganizeView";
import { RulesView } from "./features/rules/RulesView";
import { SettingsView } from "./features/settings/SettingsView";
import { api, errorMessage, useNavigation } from "./lib";
import type { Folder, Page, Rule, Scan, Settings, Snapshot } from "./types";
const FolderEditor = lazy(() =>
  import("./editors").then((module) => ({ default: module.FolderEditor })),
);
const RuleEditor = lazy(() =>
  import("./editors").then((module) => ({ default: module.RuleEditor })),
);

const nav = [
  { id: "dashboard", icon: LayoutDashboard },
  { id: "folders", icon: FolderOpen },
  { id: "rules", icon: ListFilter },
  { id: "organize", icon: ArrowRightLeft },
  { id: "history", icon: History },
  { id: "settings", icon: SettingsIcon },
] as const;
export function App() {
  const { t, i18n } = useTranslation();
  const client = useQueryClient();
  const { page, setPage } = useNavigation();
  const query = useQuery({
    queryKey: ["snapshot"],
    queryFn: () => api<Snapshot>("snapshot"),
    refetchInterval: isTauri() ? 10000 : false,
  });
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<{
    phase: string;
    done: number;
    total: number | null;
  } | null>(null);
  const [folderEditor, setFolderEditor] = useState<Folder | "new" | null>(null);
  const [ruleEditor, setRuleEditor] = useState<Rule | "new" | null>(null);
  const [source, setSource] = useState("");
  const [ruleIds, setRuleIds] = useState<string[]>([]);
  const [scan, setScan] = useState<Scan | null>(null);
  const [selected, setSelected] = useState<string[]>([]);
  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState("");
  const [actionFilter, setActionFilter] = useState("");
  const [fromDate, setFromDate] = useState("");
  const [toDate, setToDate] = useState("");
  const [version, setVersion] = useState("0.1.0");
  const [update, setUpdate] = useState("");
  const data = query.data;
  const refresh = () => {
    void client.invalidateQueries({ queryKey: ["snapshot"] });
  };
  async function run(action: () => Promise<unknown>, message?: string) {
    setBusy(true);
    setError("");
    setNotice("");
    try {
      await action();
      if (message) setNotice(message);
      refresh();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
      setProgress(null);
    }
  }
  useEffect(() => {
    if (!isTauri()) return;
    const subscriptions = [
      listen("data-changed", () => {
        void client.invalidateQueries({ queryKey: ["snapshot"] });
      }),
      listen<string>("engine-error", (e) => setError(e.payload)),
      listen<Page>("navigate", (e) => setPage(e.payload)),
      listen<{ phase: string; done: number; total: number | null }>(
        "progress",
        (e) => setProgress(e.payload),
      ),
      listen<{ chunk: number; total: number | null }>("update-progress", (e) =>
        setProgress((p) => ({
          phase: "update",
          done: (p?.done ?? 0) + e.payload.chunk,
          total: e.payload.total,
        })),
      ),
    ];
    void api<string>("app_version").then(setVersion);
    return () => {
      for (const s of subscriptions) void s.then((unlisten) => unlisten());
    };
  }, [client, setPage]);
  useEffect(() => {
    if (data) {
      void i18n.changeLanguage(data.settings.language);
      document.documentElement.dataset.theme = data.settings.theme;
      document.documentElement.dataset.reduced = String(
        data.settings.reduced_motion,
      );
    }
  }, [data, i18n]);
  useEffect(() => {
    if (!data?.settings.update_checks || !isTauri()) return;
    const check = () => {
      void api<string | null>("check_update")
        .then((v) => {
          if (v) setUpdate(v);
        })
        .catch(() => {
          /* Offline and unsigned local builds remain usable. Manual check reports the error. */
        });
    };
    check();
    const timer = setInterval(check, 86400000);
    return () => clearInterval(timer);
  }, [data?.settings.update_checks]);
  async function preferences(patch: Partial<Settings>) {
    if (!data) return;
    if (patch.automatic && !window.confirm(t("automationConfirm"))) return;
    await run(
      () => api("save_settings", { settings: { ...data.settings, ...patch } }),
      t("success"),
    );
  }
  async function analyze(folderId = source, ids = ruleIds) {
    setScan(null);
    await run(async () => {
      const result = await api<Scan>("scan_folder", { folderId, ruleIds: ids });
      setScan(result);
      setSelected(
        result.proposals.filter((p) => p.status === "ready").map((p) => p.id),
      );
    });
  }
  async function exportData(kind: string) {
    await run(async () => {
      const path = await save({
        defaultPath: `FilePilot-${kind}.json`,
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (path) await api("export_data", { path, kind });
    }, t("success"));
  }
  const onboarding = data && !data.settings.onboarding_done;
  return (
    <div className="app-shell" aria-busy={busy}>
      <aside className="sidebar">
        <div className="brand">
          <span className="logo">
            <ArrowRightLeft size={22} />
          </span>
          <strong>
            FilePilot<span>LOCAL FILE ORGANIZER</span>
          </strong>
        </div>
        <nav aria-label="FilePilot">
          {nav.map(({ id, icon: Icon }) => (
            <button
              key={id}
              className={page === id ? "nav active-nav" : "nav"}
              onClick={() => setPage(id)}
              aria-current={page === id ? "page" : undefined}
              aria-label={t(id)}
              title={t(id)}
            >
              <Icon size={18} />
              <span>{t(id)}</span>
            </button>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <div className="engine">
            <span
              className={`dot ${data?.settings.automatic && !data?.safe_mode ? "running" : ""}`}
            />
            <span>
              {t(
                data?.settings.automatic && !data?.safe_mode
                  ? "watching"
                  : "manual",
              )}
            </span>
          </div>
          <button
            className="text-button"
            disabled={!data || busy}
            onClick={() => {
              void preferences({ automatic: !data?.settings.automatic });
            }}
          >
            {data?.settings.automatic ? (
              <Pause size={14} />
            ) : (
              <Play size={14} />
            )}{" "}
            {t(data?.settings.automatic ? "pause" : "resume")}
          </button>
          <div className="sidebar-meta">
            <span>v{version}</span>
            <button
              className="icon-button"
              aria-label={t("github")}
              onClick={() => {
                void run(() => api("open_project"));
              }}
            >
              <CodeXml size={16} />
            </button>
          </div>
        </div>
      </aside>
      <main>
        <header className="topbar">
          <span>{t(page)}</span>
          <span className="privacy">
            <ShieldCheck size={14} />
            {t("local")}
          </span>
        </header>
        <div className="workspace">
          {(error || query.error) && (
            <div role="alert" className="banner error-banner">
              <strong>{t("errorTitle")}</strong>
              <span>
                {error ||
                  (isTauri() ? errorMessage(query.error) : t("desktopOnly"))}
              </span>
              <button
                onClick={() => {
                  setError("");
                  void query.refetch();
                }}
              >
                {t("retry")}
              </button>
            </div>
          )}
          {notice && (
            <div role="status" className="banner success-banner">
              <Check size={16} />
              {notice}
            </div>
          )}
          {update && (
            <div className="banner">
              <span>
                {t("updates")} · {update}
              </span>
              <button
                disabled={busy}
                onClick={() => {
                  if (window.confirm(t("installUpdate") + " ?"))
                    void run(() => api("install_update"));
                }}
              >
                {t("installUpdate")}
              </button>
            </div>
          )}
          {data?.safe_mode && (
            <div className="banner error-banner">
              <span>{t("safeMode")}</span>
              <button
                onClick={() => {
                  if (window.confirm(t("acknowledge")))
                    void run(() => api("acknowledge_recovery"));
                }}
              >
                {t("acknowledge")}
              </button>
            </div>
          )}
          {busy && (
            <div role="status" className="banner">
              <span>
                {t("working")}{" "}
                {progress &&
                  `${progress.done}${progress.total ? " / " + progress.total : ""}`}
              </span>
              <button
                onClick={() => {
                  void api("cancel_operation");
                }}
              >
                {t("stop")}
              </button>
            </div>
          )}
          {query.isPending && <p role="status">{t("loading")}</p>}
          {onboarding && (
            <section className="welcome">
              <span className="eyebrow">FILEPILOT / 01</span>
              <h1>{t("welcome")}</h1>
              <p>{t("intro")}</p>
              <div className="actions">
                <button
                  className="primary"
                  onClick={() => {
                    setPage("folders");
                    setFolderEditor("new");
                  }}
                >
                  {t("begin")}
                  <ArrowRight size={16} />
                </button>
                <button
                  onClick={() => {
                    void preferences({
                      onboarding_done: true,
                      automatic: false,
                    });
                  }}
                >
                  {t("later")}
                </button>
              </div>
              <ol className="steps">
                {["folders", "rules", "preview", "automatic"].map(
                  (key, index) => (
                    <li key={key}>
                      <span>{String(index + 1).padStart(2, "0")}</span>
                      {t(key)}
                    </li>
                  ),
                )}
              </ol>
              {data.folders.length > 0 && (
                <button
                  onClick={() => {
                    void preferences({ onboarding_done: true });
                  }}
                >
                  {t("save")}
                </button>
              )}
            </section>
          )}
          {data && page === "dashboard" && (
            <DashboardView
              data={data}
              setPage={setPage}
              setFolderEditor={setFolderEditor}
              setRuleEditor={setRuleEditor}
              run={run}
              busy={busy}
            />
          )}
          {data && page === "folders" && (
            <FoldersView
              data={data}
              busy={busy}
              setFolderEditor={setFolderEditor}
              run={run}
              setSource={setSource}
              setRuleIds={setRuleIds}
              setPage={setPage}
              analyze={analyze}
            />
          )}
          {data && page === "rules" && (
            <RulesView
              data={data}
              busy={busy}
              setRuleEditor={setRuleEditor}
              run={run}
              setSource={setSource}
              setRuleIds={setRuleIds}
              setPage={setPage}
              analyze={analyze}
            />
          )}
          {data && page === "organize" && (
            <OrganizeView
              data={data}
              source={source}
              setSource={setSource}
              ruleIds={ruleIds}
              setRuleIds={setRuleIds}
              scan={scan}
              setScan={setScan}
              selected={selected}
              setSelected={setSelected}
              busy={busy}
              analyze={analyze}
              run={run}
              setNotice={setNotice}
              setPage={setPage}
            />
          )}
          {data && page === "history" && (
            <HistoryView
              data={data}
              busy={busy}
              exportData={exportData}
              run={run}
              search={search}
              setSearch={setSearch}
              filter={filter}
              setFilter={setFilter}
              actionFilter={actionFilter}
              setActionFilter={setActionFilter}
              fromDate={fromDate}
              setFromDate={setFromDate}
              toDate={toDate}
              setToDate={setToDate}
            />
          )}
          {data && page === "settings" && (
            <SettingsView
              data={data}
              busy={busy}
              preferences={preferences}
              run={run}
              exportData={exportData}
              setUpdate={setUpdate}
              setNotice={setNotice}
              version={version}
            />
          )}
        </div>
      </main>
      <Suspense fallback={<p role="status">{t("loading")}</p>}>
        {data && folderEditor && (
          <FolderEditor
            folder={folderEditor === "new" ? undefined : folderEditor}
            close={() => setFolderEditor(null)}
            saved={refresh}
          />
        )}{" "}
        {data && ruleEditor && (
          <RuleEditor
            rule={ruleEditor === "new" ? undefined : ruleEditor}
            folders={data.folders}
            close={() => setRuleEditor(null)}
            saved={refresh}
          />
        )}
      </Suspense>
    </div>
  );
}
