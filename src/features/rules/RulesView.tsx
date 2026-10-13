import { ArrowDown, ArrowUp, Pencil, Plus, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Empty, Heading, Toggle } from "../../components";
import { api } from "../../lib";
import type { Page, Rule, Snapshot } from "../../types";

export function RulesView({
  data,
  busy,
  setRuleEditor,
  run,
  setSource,
  setRuleIds,
  setPage,
  analyze,
}: {
  data: Snapshot;
  busy: boolean;
  setRuleEditor: (rule: Rule | "new" | null) => void;
  run: (action: () => Promise<unknown>, message?: string) => Promise<void>;
  setSource: (value: string) => void;
  setRuleIds: (ids: string[]) => void;
  setPage: (page: Page) => void;
  analyze: (folderId?: string, ids?: string[]) => Promise<void>;
}) {
  const { t } = useTranslation();
  return (
    <>
      <Heading title={t("rules")} sub={t("rulesSub")}>
        <button
          className="primary"
          disabled={!data.folders.length}
          onClick={() => setRuleEditor("new")}
        >
          <Plus size={16} />
          {t("createRule")}
        </button>
      </Heading>
      {!data.rules.length ? (
        <Empty title={t("emptyRules")} />
      ) : (
        <div className="rows">
          {[...data.rules]
            .sort((a, b) => a.priority - b.priority)
            .map((r, index, sorted) => (
              <article key={r.id} className="rule-row">
                <span className="rule-order">
                  {String(index + 1).padStart(2, "0")}
                </span>
                <div className="row-content">
                  <h3>{r.name}</h3>
                  <p>
                    {r.conditions.extensions.map((e) => "." + e).join(" · ") ||
                      r.description}
                  </p>
                  <p className="destination">
                    → {r.action === "move" ? r.destination : r.rename_template}
                  </p>
                </div>
                <Toggle
                  label={t("enabled")}
                  checked={r.enabled}
                  disabled={busy}
                  onChange={(enabled) => {
                    void run(() =>
                      api("save_rule", { rule: { ...r, enabled } }),
                    );
                  }}
                />
                <div className="row-actions">
                  <button
                    aria-label={t("priority") + " ↑"}
                    disabled={busy || index === 0}
                    onClick={() => {
                      void run(async () => {
                        for (let i = 0; i < sorted.length; i++)
                          await api("save_rule", {
                            rule: {
                              ...sorted[i],
                              priority:
                                i === index
                                  ? index - 1
                                  : i === index - 1
                                    ? index
                                    : i,
                            },
                          });
                      });
                    }}
                  >
                    <ArrowUp size={16} />
                  </button>
                  <button
                    aria-label={t("priority") + " ↓"}
                    disabled={busy || index === sorted.length - 1}
                    onClick={() => {
                      void run(async () => {
                        for (let i = 0; i < sorted.length; i++)
                          await api("save_rule", {
                            rule: {
                              ...sorted[i],
                              priority:
                                i === index
                                  ? index + 1
                                  : i === index + 1
                                    ? index
                                    : i,
                            },
                          });
                      });
                    }}
                  >
                    <ArrowDown size={16} />
                  </button>
                  <button
                    aria-label={t("edit")}
                    onClick={() => setRuleEditor(r)}
                  >
                    <Pencil size={16} />
                  </button>
                  <button
                    disabled={busy || !r.enabled}
                    onClick={() => {
                      setSource(r.sources[0]);
                      setRuleIds([r.id]);
                      setPage("organize");
                      void analyze(r.sources[0], [r.id]);
                    }}
                  >
                    {t("testRule")}
                  </button>
                  <button
                    aria-label={t("remove")}
                    onClick={() => {
                      if (window.confirm(t("deleteConfirm")))
                        void run(() =>
                          api("remove_item", {
                            kind: "rule",
                            id: r.id,
                          }),
                        );
                    }}
                  >
                    <Trash2 size={16} />
                  </button>
                </div>
              </article>
            ))}
        </div>
      )}
    </>
  );
}
