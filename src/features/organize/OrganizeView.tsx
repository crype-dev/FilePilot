import { ArrowRight } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Empty, Field, Heading } from "../../components";
import { api } from "../../lib";
import type { Operation, Page, Scan, Snapshot } from "../../types";
import { ProposalList } from "../organize/ProposalList";

export function OrganizeView({
  data,
  source,
  setSource,
  ruleIds,
  setRuleIds,
  scan,
  setScan,
  selected,
  setSelected,
  busy,
  analyze,
  run,
  setNotice,
  setPage,
}: {
  data: Snapshot;
  source: string;
  setSource: (value: string) => void;
  ruleIds: string[];
  setRuleIds: (ids: string[]) => void;
  scan: Scan | null;
  setScan: (scan: Scan | null) => void;
  selected: string[];
  setSelected: (ids: string[]) => void;
  busy: boolean;
  analyze: (folderId?: string, ids?: string[]) => Promise<void>;
  run: (action: () => Promise<unknown>, message?: string) => Promise<void>;
  setNotice: (notice: string) => void;
  setPage: (page: Page) => void;
}) {
  const { t } = useTranslation();
  return (
    <>
      <Heading title={t("organize")} sub={t("organizeSub")} />
      <div className="toolbar">
        <Field label={t("source")}>
          <select
            value={source}
            onChange={(e) => {
              setSource(e.target.value);
              setScan(null);
            }}
          >
            <option value="">{t("chooseSource")}</option>
            {data.folders.map((f) => (
              <option key={f.id} value={f.id}>
                {f.name}
              </option>
            ))}
          </select>
        </Field>
        <Field label={t("rules")}>
          <select
            value={ruleIds[0] || ""}
            onChange={(e) => {
              setRuleIds(e.target.value ? [e.target.value] : []);
              setScan(null);
            }}
          >
            <option value="">{t("allRules")}</option>
            {data.rules
              .filter((r) => r.enabled && r.sources.includes(source))
              .map((r) => (
                <option key={r.id} value={r.id}>
                  {r.name}
                </option>
              ))}
          </select>
        </Field>
        <button
          className="primary"
          disabled={!source || busy}
          onClick={() => {
            void analyze();
          }}
        >
          {t("preview")}
        </button>
      </div>
      {scan ? (
        <>
          <div className="scan-summary">
            <span>
              <strong>{scan.scanned}</strong> {t("scanned")}
            </span>
            <span>
              <strong>{scan.proposals.length}</strong> {t("matched")}
            </span>
            <span>{scan.elapsed_ms} ms</span>
          </div>
          {scan.errors.map((e, index) => (
            <p className="error-text" key={index}>
              {e}
            </p>
          ))}
          {scan.proposals.length ? (
            <ProposalList
              proposals={scan.proposals}
              selected={selected}
              change={setSelected}
            />
          ) : (
            <Empty title={t("noMatches")} />
          )}
          <div className="execution-bar">
            <span>
              {selected.length} / {scan.proposals.length}
            </span>
            <button
              className="primary"
              disabled={!selected.length || busy}
              onClick={() => {
                if (window.confirm(t("executeConfirm")))
                  void run(async () => {
                    const result = await api<Operation[]>("execute_preview", {
                      token: scan.token,
                      ids: selected,
                    });
                    setScan(null);
                    setNotice(
                      `${result.filter((o) => o.status === "completed").length} ${t("completed")} · ${result.filter((o) => o.status !== "completed").length} ${t("error")}`,
                    );
                    setPage("history");
                  });
              }}
            >
              {t("execute")}
              <ArrowRight size={16} />
            </button>
          </div>
        </>
      ) : (
        <Empty title={t("noPreview")} detail={t("previewSub")} />
      )}
    </>
  );
}
