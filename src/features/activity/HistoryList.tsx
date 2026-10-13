import { useVirtualizer } from "@tanstack/react-virtual";
import { File, Undo2 } from "lucide-react";
import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Empty, Field, Modal, Status } from "../../components";
import { api, basename, bytes, errorMessage } from "../../lib";
import type { Operation } from "../../types";

export function HistoryList({
  operations,
  undo,
  disabled,
  undoBatch,
}: {
  operations: Operation[];
  undo: (id: string) => void;
  disabled: boolean;
  undoBatch?: (ids: string[]) => void;
}) {
  const { t, i18n } = useTranslation();
  const [detail, setDetail] = useState<Operation | null>(null);
  const [detailError, setDetailError] = useState("");
  const [selection, setSelection] = useState<string[]>([]);
  const parent = useRef<HTMLDivElement>(null);
  const virtual = useVirtualizer({
    count: operations.length,
    getScrollElement: () => parent.current,
    estimateSize: () => 94,
    overscan: 5,
  });
  if (!operations.length) return <Empty title={t("emptyActivity")} />;
  return (
    <>
      {detail && (
        <Modal title={t("details")} close={() => setDetail(null)}>
          <Field label={t("source")}>
            <input readOnly value={detail.source} />
          </Field>
          <Field label={t("destination")}>
            <input readOnly value={detail.destination} />
          </Field>
          <Field label={t("integrity")}>
            <input readOnly value={detail.hash} />
          </Field>
          <p>
            {t(detail.action)} · {detail.rule_name} · {bytes(detail.size)}
          </p>
          <Status status={detail.status} />
          {detail.error && <p className="error-text">{detail.error}</p>}
          <div className="actions">
            {["source", "destination"].map((side) => (
              <button
                key={side}
                onClick={() => {
                  void api("open_history_location", {
                    id: detail.id,
                    side,
                  }).catch((e) => setDetailError(errorMessage(e)));
                }}
              >
                {t("open")} · {t(side)}
              </button>
            ))}
          </div>
          {detailError && (
            <p role="alert" className="error-text">
              {detailError}
            </p>
          )}
        </Modal>
      )}
      {undoBatch && selection.length > 0 && (
        <div className="execution-bar">
          <span>{selection.length}</span>
          <button
            disabled={disabled || selection.length > 100}
            onClick={() => {
              undoBatch(selection);
              setSelection([]);
            }}
          >
            {t("undoSelection")}
          </button>
        </div>
      )}
      <div className="virtual-list" ref={parent}>
        <div style={{ height: virtual.getTotalSize(), position: "relative" }}>
          {virtual.getVirtualItems().map((item) => {
            const op = operations[item.index];
            return (
              <article
                key={op.id}
                className="history-row virtual-row"
                style={{
                  transform: `translateY(${item.start}px)`,
                  height: item.size,
                }}
              >
                {undoBatch && op.status === "completed" ? (
                  <input
                    type="checkbox"
                    aria-label={`${t("selected")} ${basename(op.source)}`}
                    checked={selection.includes(op.id)}
                    onChange={(e) =>
                      setSelection(
                        e.target.checked
                          ? [...selection, op.id]
                          : selection.filter((id) => id !== op.id),
                      )
                    }
                  />
                ) : (
                  <File size={19} />
                )}
                <div className="row-content">
                  <h3 title={op.source}>
                    <button
                      className="row-title"
                      onClick={() => {
                        setDetailError("");
                        setDetail(op);
                      }}
                    >
                      {basename(op.source)}
                    </button>
                  </h3>
                  <p title={op.destination}>{op.destination}</p>
                  <small>
                    {op.rule_name} ·{" "}
                    {new Date(op.time * 1000).toLocaleString(i18n.language)} ·{" "}
                    {bytes(op.size)}
                  </small>
                  {op.error && (
                    <small className="error-text" title={op.error}>
                      {op.error}
                    </small>
                  )}
                </div>
                <Status status={op.status} />
                {op.status === "completed" && (
                  <button
                    disabled={disabled}
                    className="icon-button"
                    aria-label={t("undo")}
                    onClick={() => undo(op.id)}
                  >
                    <Undo2 size={17} />
                  </button>
                )}
              </article>
            );
          })}
        </div>
      </div>
    </>
  );
}
