import { isTauri } from "@tauri-apps/api/core";
import * as autostart from "@tauri-apps/plugin-autostart";
import { open } from "@tauri-apps/plugin-dialog";
import { CodeXml } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Field, Heading, Toggle } from "../../components";
import { api } from "../../lib";
import type { Settings, Snapshot } from "../../types";

export function SettingsView({
  data,
  busy,
  preferences,
  run,
  exportData,
  setUpdate,
  setNotice,
  version,
}: {
  data: Snapshot;
  busy: boolean;
  preferences: (patch: Partial<Settings>) => Promise<void>;
  run: (action: () => Promise<unknown>, message?: string) => Promise<void>;
  exportData: (kind: string) => Promise<void>;
  setUpdate: (version: string) => void;
  setNotice: (notice: string) => void;
  version: string;
}) {
  const { t } = useTranslation();
  return (
    <>
      <Heading title={t("settings")} sub={t("local")} />
      <SettingsPanel
        value={data.settings}
        change={preferences}
        disabled={busy}
        run={run}
      />
      <section className="settings-section">
        <h2>{t("data")}</h2>
        <Field label={t("historyDays")}>
          <input
            type="number"
            min={0}
            max={36500}
            defaultValue={data.settings.history_days}
            key={data.settings.history_days}
            onBlur={(e) => {
              if (
                Number(e.target.value) !== data.settings.history_days &&
                window.confirm(t("purgeConfirm"))
              )
                void preferences({
                  history_days: Number(e.target.value),
                });
            }}
          />
        </Field>
        <p className="hint">{t("importHint")}</p>
        <div className="actions">
          <button
            onClick={() => {
              void exportData("rules");
            }}
          >
            {t("exportRules")}
          </button>
          <button
            onClick={() => {
              void exportData("history");
            }}
          >
            {t("exportHistory")}
          </button>
          <button
            onClick={() => {
              void exportData("settings");
            }}
          >
            {t("exportSettings")}
          </button>
          <button
            onClick={() => {
              void run(async () => {
                const path = await open({
                  multiple: false,
                  filters: [{ name: "JSON", extensions: ["json"] }],
                });
                if (typeof path === "string")
                  await api("import_rules", { path });
              }, t("success"));
            }}
          >
            {t("import")}
          </button>
          <button
            disabled={busy || data.settings.automatic}
            onClick={() => {
              if (window.confirm(t("purgeConfirm")))
                void run(() => api("purge_history"), t("success"));
            }}
          >
            {t("purgeHistory")}
          </button>
          <button
            disabled={busy}
            onClick={() => {
              if (window.confirm(t("resetConfirm")))
                void run(async () => {
                  await autostart.disable();
                  await api("reset_preferences");
                }, t("success"));
            }}
          >
            {t("resetPreferences")}
          </button>
          <button
            disabled={busy}
            onClick={() => {
              void exportData("diagnostics");
            }}
          >
            {t("diagnostics")}
          </button>
        </div>
      </section>
      <section className="settings-section">
        <h2>{t("updates")}</h2>
        <Toggle
          label={t("autoUpdates")}
          checked={data.settings.update_checks}
          onChange={(update_checks) => {
            void preferences({ update_checks });
          }}
        />
        <button
          disabled={busy}
          onClick={() => {
            void run(async () => {
              const version = await api<string | null>("check_update");
              if (version) setUpdate(version);
              else setNotice(t("noUpdate"));
            });
          }}
        >
          {t("checkUpdate")}
        </button>
      </section>
      <section className="settings-section">
        <h2>{t("about")}</h2>
        <p>FilePilot {version} · MIT</p>
        <button
          onClick={() => {
            void run(() => api("open_project"));
          }}
        >
          <CodeXml size={16} />
          {t("github")}
        </button>
      </section>
    </>
  );
}

function SettingsPanel({
  value,
  change,
  disabled,
  run,
}: {
  value: Settings;
  change: (patch: Partial<Settings>) => Promise<void>;
  disabled: boolean;
  run: (action: () => Promise<unknown>, message?: string) => Promise<void>;
}) {
  const { t } = useTranslation();
  const [start, setStart] = useState(false);
  useEffect(() => {
    if (isTauri()) void autostart.isEnabled().then(setStart);
  }, []);
  return (
    <>
      <section className="settings-section">
        <h2>{t("general")}</h2>
        <div className="form-grid">
          <Field label={t("language")}>
            <select
              value={value.language}
              disabled={disabled}
              onChange={(e) => {
                void change({
                  language: e.target.value as Settings["language"],
                });
              }}
            >
              <option value="fr">Français</option>
              <option value="en">English</option>
            </select>
          </Field>
          <Field label={t("theme")}>
            <select
              value={value.theme}
              disabled={disabled}
              onChange={(e) => {
                void change({ theme: e.target.value as Settings["theme"] });
              }}
            >
              {["system", "light", "dark"].map((v) => (
                <option key={v} value={v}>
                  {t(v)}
                </option>
              ))}
            </select>
          </Field>
        </div>
        <Toggle
          label={t("closeTray")}
          checked={value.close_to_tray}
          disabled={disabled}
          onChange={(close_to_tray) => {
            void change({ close_to_tray });
          }}
        />
        <Toggle
          label={t("reduced")}
          checked={value.reduced_motion}
          disabled={disabled}
          onChange={(reduced_motion) => {
            void change({ reduced_motion });
          }}
        />
        <Toggle
          label={t("autostart")}
          checked={start}
          disabled={disabled}
          onChange={(checked) => {
            void run(async () => {
              if (checked) await autostart.enable();
              else await autostart.disable();
              setStart(await autostart.isEnabled());
            });
          }}
        />
      </section>
      <section className="settings-section">
        <h2>{t("automation")}</h2>
        <p className="hint">{t("automaticHint")}</p>
        <Toggle
          label={t("automatic")}
          checked={value.automatic}
          disabled={disabled}
          onChange={(automatic) => {
            void change({ automatic });
          }}
        />
        <Toggle
          label={t("scanStart")}
          checked={value.scan_on_start}
          disabled={disabled}
          onChange={(scan_on_start) => {
            void change({ scan_on_start });
          }}
        />
        <div className="form-grid">
          <Field label={t("stabilization")}>
            <input
              key={value.stabilization_seconds}
              type="number"
              min={2}
              max={300}
              defaultValue={value.stabilization_seconds}
              disabled={disabled}
              onBlur={(e) => {
                if (Number(e.target.value) !== value.stabilization_seconds)
                  void change({
                    stabilization_seconds: Number(e.target.value),
                  });
              }}
            />
          </Field>
          <Field label={t("reconciliation")}>
            <input
              key={value.reconciliation_seconds}
              type="number"
              min={30}
              max={86400}
              defaultValue={value.reconciliation_seconds}
              disabled={disabled}
              onBlur={(e) => {
                if (Number(e.target.value) !== value.reconciliation_seconds)
                  void change({
                    reconciliation_seconds: Number(e.target.value),
                  });
              }}
            />
          </Field>
        </div>
        <Toggle
          label={t("notifications")}
          checked={value.notifications}
          disabled={disabled}
          onChange={(notifications) => {
            void change({ notifications });
          }}
        />
      </section>
    </>
  );
}
