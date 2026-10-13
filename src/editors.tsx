import { useState } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";
import { zodResolver } from "@hookform/resolvers/zod";
import { useTranslation } from "react-i18next";
import { open } from "@tauri-apps/plugin-dialog";
import { FolderOpen } from "lucide-react";
import { Field, Modal, Toggle } from "./components";
import { api, basename, errorMessage, newRule, templates } from "./lib";
import type { Folder, Rule } from "./types";
export async function pickFolder() {
  const path = await open({ directory: true, multiple: false });
  return typeof path === "string" ? path : null;
}
export function FolderEditor({
  folder,
  close,
  saved,
}: {
  folder?: Folder;
  close: () => void;
  saved: () => void;
}) {
  const { t } = useTranslation();
  const [value, setValue] = useState<Folder>(
    folder || { id: "", name: "", path: "", recursive: false, enabled: true },
  );
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setBusy(true);
    try {
      await api("save_folder", { folder: value });
      saved();
      close();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <Modal title={t(folder ? "edit" : "addFolder")} close={close}>
      <form onSubmit={submit}>
        <Field label={t("name")}>
          <input
            required
            maxLength={120}
            value={value.name}
            onChange={(e) => setValue({ ...value, name: e.target.value })}
          />
        </Field>
        <Field label={t("path")}>
          <div className="input-action">
            <input
              required
              readOnly
              aria-label={t("path")}
              value={value.path}
            />
            <button
              type="button"
              aria-label={t("selectFolder")}
              onClick={() => {
                void pickFolder()
                  .then((path) => {
                    if (path)
                      setValue({
                        ...value,
                        path,
                        name: value.name || basename(path),
                      });
                  })
                  .catch((e) => setError(errorMessage(e)));
              }}
            >
              <FolderOpen size={18} />
            </button>
          </div>
        </Field>
        <Toggle
          label={t("recursive")}
          checked={value.recursive}
          onChange={(recursive) => setValue({ ...value, recursive })}
        />
        <Toggle
          label={t("enabled")}
          checked={value.enabled}
          onChange={(enabled) => setValue({ ...value, enabled })}
        />
        {error && (
          <p role="alert" className="error-text">
            {error}
          </p>
        )}
        <div className="form-actions">
          <button type="button" onClick={close}>
            {t("cancel")}
          </button>
          <button className="primary" disabled={busy}>
            {t("save")}
          </button>
        </div>
      </form>
    </Modal>
  );
}
export function RuleEditor({
  rule,
  folders,
  close,
  saved,
}: {
  rule?: Rule;
  folders: Folder[];
  close: () => void;
  saved: () => void;
}) {
  const { t } = useTranslation();
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const schema = z.object({
    name: z.string().trim().min(1, t("required")).max(120),
    description: z.string(),
    priority: z.number().int(),
    destination: z.string(),
    rename_template: z.string(),
    action: z.enum(["move", "rename"]),
    conflict: z.enum(["skip", "unique"]),
    enabled: z.boolean(),
    sources: z.array(z.string()).min(1, t("required")),
  });
  const initial = rule || newRule();
  const {
    register,
    handleSubmit,
    watch,
    setValue,
    formState: { errors },
  } = useForm<z.infer<typeof schema>>({
    resolver: zodResolver(schema),
    defaultValues: initial,
  });
  const [conditions, setConditions] = useState(initial.conditions);
  const [exclusions, setExclusions] = useState(initial.exclusions);
  const sources = watch("sources");
  const action = watch("action");
  const listKeys = [
    "extensions",
    "names",
    "contains",
    "globs",
    "relative_globs",
  ] as const;
  const numberKeys = [
    "min_size",
    "max_size",
    "min_age_days",
    "max_age_days",
  ] as const;
  const labels = {
    relative_globs: "relative",
    min_size: "minSize",
    max_size: "maxSize",
    min_age_days: "minAge",
    max_age_days: "maxAge",
  };
  return (
    <Modal title={t(rule ? "edit" : "createRule")} close={close}>
      <form
        onSubmit={handleSubmit(async (data) => {
          setBusy(true);
          try {
            await api("save_rule", {
              rule: { ...initial, ...data, conditions, exclusions },
            });
            saved();
            close();
          } catch (e) {
            setError(errorMessage(e));
          } finally {
            setBusy(false);
          }
        })}
      >
        {!rule && (
          <Field label={t("template")}>
            <select
              defaultValue=""
              onChange={(e) => {
                if (e.target.value) {
                  setConditions({
                    ...conditions,
                    extensions: templates[e.target.value],
                  });
                  setValue("name", t(e.target.value));
                }
              }}
            >
              <option value="">{t("custom")}</option>
              {Object.keys(templates).map((key) => (
                <option key={key} value={key}>
                  {t(key)}
                </option>
              ))}
            </select>
          </Field>
        )}
        <div className="form-grid">
          <Field label={t("name")}>
            <input {...register("name")} />
            {errors.name && (
              <small className="error-text">{errors.name.message}</small>
            )}
          </Field>
          <Field label={t("priority")}>
            <input
              type="number"
              {...register("priority", { valueAsNumber: true })}
            />
          </Field>
        </div>
        <Field label={t("description")}>
          <input {...register("description")} />
        </Field>
        <fieldset>
          <legend>{t("source")}</legend>
          {folders.map((f) => (
            <label className="check" key={f.id}>
              <input
                type="checkbox"
                checked={sources.includes(f.id)}
                onChange={(e) =>
                  setValue(
                    "sources",
                    e.target.checked
                      ? [...sources, f.id]
                      : sources.filter((s) => s !== f.id),
                  )
                }
              />
              {f.name}
            </label>
          ))}
          {errors.sources && (
            <small className="error-text">{errors.sources.message}</small>
          )}
        </fieldset>
        <Field label={t("extensions")}>
          <input
            value={conditions.extensions.join(", ")}
            onChange={(e) =>
              setConditions({
                ...conditions,
                extensions: e.target.value
                  .split(",")
                  .map((s) => s.trim())
                  .filter(Boolean),
              })
            }
          />
        </Field>
        <p className="hint">
          {t("commaHint")} {t("rulesSub")}
        </p>
        <details>
          <summary>{t("advanced")}</summary>
          {listKeys
            .filter((k) => k !== "extensions")
            .map((key) => (
              <Field
                key={key}
                label={t(key === "relative_globs" ? labels[key] : key)}
              >
                <input
                  value={conditions[key].join(", ")}
                  onChange={(e) =>
                    setConditions({
                      ...conditions,
                      [key]: e.target.value
                        .split(",")
                        .map((s) => s.trim())
                        .filter(Boolean),
                    })
                  }
                />
              </Field>
            ))}
          <Field label={t("exclusions")}>
            <input
              value={exclusions.join(", ")}
              onChange={(e) =>
                setExclusions(
                  e.target.value
                    .split(",")
                    .map((s) => s.trim())
                    .filter(Boolean),
                )
              }
            />
          </Field>
          <div className="form-grid">
            {numberKeys.map((key) => (
              <Field key={key} label={t(labels[key])}>
                <input
                  type="number"
                  min="0"
                  value={conditions[key] ?? ""}
                  onChange={(e) =>
                    setConditions({
                      ...conditions,
                      [key]:
                        e.target.value === "" ? null : Number(e.target.value),
                    })
                  }
                />
              </Field>
            ))}
          </div>
          {(["modified_after", "modified_before"] as const).map((key) => (
            <Field
              key={key}
              label={t(key === "modified_after" ? "after" : "before")}
            >
              <input
                type="date"
                value={
                  conditions[key]
                    ? new Date(conditions[key]! * 1000)
                        .toISOString()
                        .slice(0, 10)
                    : ""
                }
                onChange={(e) =>
                  setConditions({
                    ...conditions,
                    [key]: e.target.value
                      ? Date.parse(e.target.value + "T00:00:00Z") / 1000
                      : null,
                  })
                }
              />
            </Field>
          ))}
        </details>
        <div className="form-grid">
          <Field label={t("action")}>
            <select {...register("action")}>
              <option value="move">{t("move")}</option>
              <option value="rename">{t("rename")}</option>
            </select>
          </Field>
          <Field label={t("conflict")}>
            <select {...register("conflict")}>
              <option value="unique">{t("unique")}</option>
              <option value="skip">{t("skip")}</option>
            </select>
          </Field>
        </div>
        {action === "move" ? (
          <Field label={t("destination")}>
            <div className="input-action">
              <input
                {...register("destination")}
                aria-label={t("destination")}
                required
              />
              <button
                type="button"
                aria-label={t("selectFolder")}
                onClick={() => {
                  void pickFolder()
                    .then((path) => {
                      if (path) setValue("destination", path);
                    })
                    .catch((e) => setError(errorMessage(e)));
                }}
              >
                <FolderOpen size={18} />
              </button>
            </div>
          </Field>
        ) : (
          <Field label={t("renameTemplate")}>
            <input {...register("rename_template")} required />
            <small>{t("renameHint")}</small>
          </Field>
        )}
        <Toggle
          label={t("enabled")}
          checked={watch("enabled")}
          onChange={(enabled) => setValue("enabled", enabled)}
        />
        {error && (
          <p role="alert" className="error-text">
            {error}
          </p>
        )}
        <div className="form-actions">
          <button type="button" onClick={close}>
            {t("cancel")}
          </button>
          <button className="primary" disabled={busy}>
            {t("save")}
          </button>
        </div>
      </form>
    </Modal>
  );
}
