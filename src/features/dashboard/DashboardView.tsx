import { ArrowRight, FolderOpen, ListFilter, Plus } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Empty, Heading } from "../../components";
import { api, bytes } from "../../lib";
import type { Folder, Page, Rule, Snapshot } from "../../types";
import { HistoryList } from "../activity/HistoryList";

export function DashboardView({
  data,
  setPage,
  setFolderEditor,
  setRuleEditor,
  run,
  busy,
}: {
  data: Snapshot;
  setPage: (page: Page) => void;
  setFolderEditor: (folder: Folder | "new" | null) => void;
  setRuleEditor: (rule: Rule | "new" | null) => void;
  run: (action: () => Promise<unknown>, message?: string) => Promise<void>;
  busy: boolean;
}) {
  const { t } = useTranslation();
  return (
    <>
      <Heading title={t("overview")} sub={t("overviewSub")}>
        <button className="primary" onClick={() => setPage("organize")}>
          {t("organize")}
          <ArrowRight size={16} />
        </button>
      </Heading>
      <div className="stats">
        {[
          {
            label: "folderCount",
            value: data.folders.filter((f) => f.enabled).length,
          },
          {
            label: "ruleCount",
            value: data.rules.filter((r) => r.enabled).length,
          },
          {
            label: "organized",
            value: data.history.filter((o) => o.status === "completed").length,
          },
          {
            label: "volume",
            value: bytes(
              data.history
                .filter((o) => o.status === "completed")
                .reduce((n, o) => n + o.size, 0),
            ),
          },
        ].map((s) => (
          <div key={s.label}>
            <span>{t(s.label)}</span>
            <strong>{s.value}</strong>
          </div>
        ))}
      </div>
      <div className="section-title">
        <h2>{t("recent")}</h2>
        <button className="text-button" onClick={() => setPage("history")}>
          {t("viewAll")}
          <ArrowRight size={14} />
        </button>
      </div>
      {data.history.length ? (
        <HistoryList
          operations={data.history.slice(0, 10)}
          undo={(id) => {
            if (window.confirm(t("undoConfirm")))
              void run(() => api("undo_operation", { id }));
          }}
          disabled={busy}
        />
      ) : (
        <Empty title={t("emptyActivity")} detail={t("emptyActivitySub")}>
          <button
            onClick={() => {
              setPage("folders");
              setFolderEditor("new");
            }}
          >
            <Plus size={16} />
            {t("addFolder")}
          </button>
        </Empty>
      )}
      <div className="quick-links">
        <button onClick={() => setFolderEditor("new")}>
          <FolderOpen size={18} />
          {t("addFolder")}
          <Plus size={16} />
        </button>
        <button
          onClick={() => setRuleEditor("new")}
          disabled={!data.folders.length}
        >
          <ListFilter size={18} />
          {t("createRule")}
          <Plus size={16} />
        </button>
      </div>
    </>
  );
}
