import { FolderOpen, Pencil, Plus, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Empty, Heading, Status } from "../../components";
import { api } from "../../lib";
import type { Folder, Page, Snapshot } from "../../types";

export function FoldersView({
  data,
  busy,
  setFolderEditor,
  run,
  setSource,
  setRuleIds,
  setPage,
  analyze,
}: {
  data: Snapshot;
  busy: boolean;
  setFolderEditor: (folder: Folder | "new" | null) => void;
  run: (action: () => Promise<unknown>, message?: string) => Promise<void>;
  setSource: (value: string) => void;
  setRuleIds: (ids: string[]) => void;
  setPage: (page: Page) => void;
  analyze: (folderId?: string, ids?: string[]) => Promise<void>;
}) {
  const { t } = useTranslation();
  return (
    <>
      <Heading title={t("folders")} sub={t("folderSub")}>
        <button className="primary" onClick={() => setFolderEditor("new")}>
          <Plus size={16} />
          {t("addFolder")}
        </button>
      </Heading>
      {!data.folders.length ? (
        <Empty title={t("emptyFolders")} />
      ) : (
        <div className="rows">
          {data.folders.map((f) => (
            <article className="folder-row" key={f.id}>
              <FolderOpen className="file-icon" size={25} />
              <div className="row-content">
                <h3>{f.name}</h3>
                <p title={f.path}>{f.path}</p>
                <small>{f.recursive ? t("recursive") : ""}</small>
              </div>
              <Status status={f.enabled ? "active" : "inactive"} />
              <div className="row-actions">
                <button
                  aria-label={t("open")}
                  onClick={() => {
                    void run(() => api("open_folder", { id: f.id }));
                  }}
                >
                  <FolderOpen size={16} />
                </button>
                <button
                  aria-label={t("edit")}
                  onClick={() => setFolderEditor(f)}
                >
                  <Pencil size={16} />
                </button>
                <button
                  disabled={busy}
                  onClick={() => {
                    setSource(f.id);
                    setRuleIds([]);
                    setPage("organize");
                    void analyze(f.id, []);
                  }}
                >
                  {t("analyze")}
                </button>
                <button
                  aria-label={t("remove")}
                  onClick={() => {
                    if (window.confirm(t("deleteConfirm")))
                      void run(() =>
                        api("remove_item", {
                          kind: "folder",
                          id: f.id,
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
