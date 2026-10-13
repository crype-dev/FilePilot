import { useTranslation } from "react-i18next";
import { Heading } from "../../components";
import { api } from "../../lib";
import type { Snapshot } from "../../types";
import { HistoryList } from "../activity/HistoryList";

export function HistoryView({
  data,
  busy,
  exportData,
  run,
  search,
  setSearch,
  filter,
  setFilter,
  actionFilter,
  setActionFilter,
  fromDate,
  setFromDate,
  toDate,
  setToDate,
}: {
  data: Snapshot;
  busy: boolean;
  exportData: (kind: string) => Promise<void>;
  run: (action: () => Promise<unknown>, message?: string) => Promise<void>;
  search: string;
  setSearch: (s: string) => void;
  filter: string;
  setFilter: (s: string) => void;
  actionFilter: string;
  setActionFilter: (s: string) => void;
  fromDate: string;
  setFromDate: (s: string) => void;
  toDate: string;
  setToDate: (s: string) => void;
}) {
  const { t } = useTranslation();
  return (
    <>
      <Heading title={t("history")} sub={t("historySub")}>
        <button
          onClick={() => {
            void exportData("history");
          }}
        >
          {t("export")}
        </button>
      </Heading>
      <div className="toolbar">
        <input
          aria-label={t("search")}
          placeholder={t("search")}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
        <select
          aria-label={t("allStatuses")}
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
        >
          <option value="">{t("allStatuses")}</option>
          {["completed", "failed", "reconcile", "undone"].map((s) => (
            <option key={s} value={s}>
              {t(s)}
            </option>
          ))}
        </select>
        <select
          aria-label={t("allActions")}
          value={actionFilter}
          onChange={(e) => setActionFilter(e.target.value)}
        >
          <option value="">{t("allActions")}</option>
          {["move", "rename"].map((a) => (
            <option key={a} value={a}>
              {t(a)}
            </option>
          ))}
        </select>
        <input
          aria-label={t("fromDate")}
          type="date"
          value={fromDate}
          onChange={(e) => setFromDate(e.target.value)}
        />
        <input
          aria-label={t("toDate")}
          type="date"
          value={toDate}
          onChange={(e) => setToDate(e.target.value)}
        />
      </div>
      <HistoryList
        operations={data.history.filter(
          (o) =>
            (!filter || o.status === filter) &&
            (!actionFilter || o.action === actionFilter) &&
            (!fromDate ||
              o.time * 1000 >= Date.parse(fromDate + "T00:00:00")) &&
            (!toDate || o.time * 1000 <= Date.parse(toDate + "T23:59:59")) &&
            (o.source + o.destination + o.rule_name)
              .toLowerCase()
              .includes(search.toLowerCase()),
        )}
        disabled={busy}
        undoBatch={(ids) => {
          if (window.confirm(t("undoConfirm")))
            void run(() => api("undo_operations", { ids }));
        }}
        undo={(id) => {
          if (window.confirm(t("undoConfirm")))
            void run(() => api("undo_operation", { id }));
        }}
      />
    </>
  );
}
