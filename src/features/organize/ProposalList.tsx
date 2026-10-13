import { useVirtualizer } from "@tanstack/react-virtual";
import { useRef } from "react";
import { useTranslation } from "react-i18next";
import { Status } from "../../components";
import { basename, bytes } from "../../lib";
import type { Proposal } from "../../types";

export function ProposalList({
  proposals,
  selected,
  change,
}: {
  proposals: Proposal[];
  selected: string[];
  change: (ids: string[]) => void;
}) {
  const { t } = useTranslation();
  const parent = useRef<HTMLDivElement>(null);
  const virtual = useVirtualizer({
    count: proposals.length,
    getScrollElement: () => parent.current,
    estimateSize: () => 84,
    overscan: 5,
  });
  return (
    <>
      <label className="check select-all">
        <input
          type="checkbox"
          checked={
            selected.length > 0 &&
            selected.length ===
              proposals.filter((p) => p.status === "ready").length
          }
          onChange={(e) =>
            change(
              e.target.checked
                ? proposals.filter((p) => p.status === "ready").map((p) => p.id)
                : [],
            )
          }
        />
        {t("selected")}
      </label>
      <div className="virtual-list" ref={parent}>
        <div style={{ height: virtual.getTotalSize(), position: "relative" }}>
          {virtual.getVirtualItems().map((item) => {
            const p = proposals[item.index];
            return (
              <div
                className="proposal-row virtual-row"
                key={p.id}
                style={{
                  transform: `translateY(${item.start}px)`,
                  height: item.size,
                }}
              >
                <input
                  type="checkbox"
                  aria-label={`${t("selected")} ${basename(p.source)}`}
                  disabled={p.status !== "ready"}
                  checked={selected.includes(p.id)}
                  onChange={(e) =>
                    change(
                      e.target.checked
                        ? [...selected, p.id]
                        : selected.filter((id) => id !== p.id),
                    )
                  }
                />
                <div className="row-content">
                  <h3 title={p.source}>{basename(p.source)}</h3>
                  <p title={p.destination}>→ {p.destination}</p>
                  <small>
                    {p.reason || p.rule_name} · {bytes(p.size)}
                  </small>
                </div>
                <Status status={p.status} />
              </div>
            );
          })}
        </div>
      </div>
    </>
  );
}
