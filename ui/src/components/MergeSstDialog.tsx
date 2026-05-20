import { useState, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { sstMerge, type MergeStatsDto } from "../api/strings";
import { open as dialogOpen } from "@tauri-apps/plugin-dialog";
import toast from "react-hot-toast";
import { Button, Modal } from "./ui";
import { Merge, FileUp, ToggleLeft, ToggleRight } from "lucide-react";

interface MergeSstDialogProps {
  open: boolean;
  onClose: () => void;
  onMergeComplete: () => Promise<void>;
  espPath: string | null;
}

export function MergeSstDialog({ open, onClose, onMergeComplete, espPath }: MergeSstDialogProps) {
  const { t } = useTranslation();
  const [sourcePath, setSourcePath] = useState("");
  const [overwrite, setOverwrite] = useState(false);
  const [merging, setMerging] = useState(false);
  const [result, setResult] = useState<MergeStatsDto | null>(null);

  const handlePickFile = useCallback(async () => {
    const selected = await dialogOpen({
      multiple: false,
      directory: false,
      filters: [{ name: "SST Dictionary", extensions: ["sst"] }],
    });
    const path = Array.isArray(selected) ? selected[0] : selected;
    if (path) {
      setSourcePath(path);
      setResult(null);
    }
  }, []);

  const handleMerge = useCallback(async () => {
    if (!sourcePath) {
      toast.error(t("mergeSst.noSource", { defaultValue: "Please select a source SST file" }));
      return;
    }
    if (!espPath) {
      toast.error(t("mergeSst.noWorkData", { defaultValue: "Load an ESP/ESM file first before merging" }));
      return;
    }

    setMerging(true);
    setResult(null);
    try {
      const stats = await sstMerge(sourcePath, overwrite);
      setResult(stats);
      await onMergeComplete();
      toast.success(
        t("mergeSst.success", {
          defaultValue: "Merge completed: +{{added}} added, {{updated}} updated, {{overwritten}} overwritten, {{skipped}} skipped",
          added: stats.added,
          updated: stats.updated,
          overwritten: stats.overwritten,
          skipped: stats.conflicts_skipped,
        }),
        { duration: 6000 },
      );
    } catch (e: any) {
      toast.error(`${t("mergeSst.failed", { defaultValue: "Merge failed" })}: ${e}`);
    } finally {
      setMerging(false);
    }
  }, [sourcePath, overwrite, espPath, onMergeComplete, t]);

  const handleClose = useCallback(() => {
    setSourcePath("");
    setOverwrite(false);
    setResult(null);
    onClose();
  }, [onClose]);

  const hasResult = result !== null;

  return (
    <Modal
      open={open}
      onClose={handleClose}
      title={t("mergeSst.title", { defaultValue: "Merge SST Dictionary" })}
      size="md"
      footer={
        <>
          <Button variant="ghost" onClick={handleClose}>
            {t("common.close")}
          </Button>
        </>
      }
    >
      <div className="merge-sst-dialog">
        {/* Source file selector */}
        <div className="merge-sst-field">
          <label className="merge-sst-label">
            {t("mergeSst.sourceFile", { defaultValue: "Source SST file" })}:
          </label>
          <div className="merge-sst-field-row">
            <code className="merge-sst-path">{sourcePath || t("mergeSst.noFileSelected", { defaultValue: "No file selected" })}</code>
            <Button variant="ghost" size="sm" onClick={handlePickFile} icon={<FileUp size={14} />}>
              {t("mergeSst.browse", { defaultValue: "Browse" })}
            </Button>
          </div>
        </div>

        {/* Overwrite toggle */}
        <div className="merge-sst-field">
          <label className="merge-sst-label">
            {t("mergeSst.overwrite", { defaultValue: "Overwrite existing translations" })}:
          </label>
          <Button
            variant={overwrite ? "primary" : "ghost"}
            size="sm"
            onClick={() => setOverwrite(!overwrite)}
            icon={overwrite ? <ToggleRight size={14} /> : <ToggleLeft size={14} />}
          >
            {overwrite
              ? t("mergeSst.overwriteOn", { defaultValue: "On" })
              : t("mergeSst.overwriteOff", { defaultValue: "Off" })}
          </Button>
        </div>

        {/* Merge action */}
        <div className="merge-sst-actions">
          <Button
            variant="primary"
            size="sm"
            onClick={handleMerge}
            loading={merging}
            disabled={!sourcePath || !espPath}
            icon={<Merge size={14} />}
          >
            {t("mergeSst.merge", { defaultValue: "Merge" })}
          </Button>
        </div>

        {/* Results */}
        {hasResult && (
          <div className="merge-sst-results">
            <h4>{t("mergeSst.results", { defaultValue: "Merge Results" })}:</h4>
            <table className="merge-sst-stats">
              <tbody>
                <tr>
                  <td className="merge-sst-stat-label">{t("mergeSst.added", { defaultValue: "Added" })}</td>
                  <td className="merge-sst-stat-value">{result.added}</td>
                  <td className="merge-sst-stat-note">
                    {t("mergeSst.addedDesc", { defaultValue: "New entries from source" })}
                  </td>
                </tr>
                <tr>
                  <td className="merge-sst-stat-label">{t("mergeSst.updated", { defaultValue: "Updated" })}</td>
                  <td className="merge-sst-stat-value">{result.updated}</td>
                  <td className="merge-sst-stat-note">
                    {t("mergeSst.updatedDesc", { defaultValue: "Empty translations filled from source" })}
                  </td>
                </tr>
                <tr>
                  <td className="merge-sst-stat-label">{t("mergeSst.overwritten", { defaultValue: "Overwritten" })}</td>
                  <td className="merge-sst-stat-value">{result.overwritten}</td>
                  <td className="merge-sst-stat-note">
                    {t("mergeSst.overwrittenDesc", { defaultValue: "Existing translations replaced (overwrite=on)" })}
                  </td>
                </tr>
                <tr>
                  <td className="merge-sst-stat-label">{t("mergeSst.skipped", { defaultValue: "Conflicts skipped" })}</td>
                  <td className="merge-sst-stat-value">{result.conflicts_skipped}</td>
                  <td className="merge-sst-stat-note">
                    {t("mergeSst.skippedDesc", { defaultValue: "Existing translations kept (overwrite=off)" })}
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        )}
      </div>
    </Modal>
  );
}
