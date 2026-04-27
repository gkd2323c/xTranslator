import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useTranslation } from "react-i18next";
import {
  FolderOpen, FileText, Download, Folder,
  HardDrive, FileArchive, ChevronRight, ChevronDown
} from "lucide-react";
import toast from "react-hot-toast";
import { listBsaFiles, extractBsaFile, extractBsaFolder } from "../api/strings";
import type { BsaFileListDto, BsaFileEntryDto } from "../api/strings";

function formatSize(bytes: number): string {
  if (bytes >= 1_048_576) return `${(bytes / 1_048_576).toFixed(1)} MB`;
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${bytes} B`;
}

export function BsaBrowser() {
  const { t } = useTranslation();
  const [bsaPath, setBsaPath] = useState<string | null>(null);
  const [fileList, setFileList] = useState<BsaFileListDto | null>(null);
  const [loading, setLoading] = useState(false);
  const [selectedFolder, setSelectedFolder] = useState<string | null>(null);
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(new Set());

  const filteredFiles = fileList?.files.filter(
    (f) => !selectedFolder || f.folder === selectedFolder
  ) ?? [];

  const handleOpen = async () => {
    const path = await open({
      multiple: false,
      directory: false,
      filters: [
        { name: "Bethesda Archive", extensions: ["bsa", "ba2"] },
        { name: "All", extensions: ["*"] },
      ],
    });
    if (!path) return;

    setLoading(true);
    setSelectedFolder(null);
    try {
      const list = await listBsaFiles(path);
      setFileList(list);
      setBsaPath(path);
      setExpandedFolders(new Set(list.folders));
    } catch (e: any) {
      toast.error(`${t("bsa.failedToOpen")}: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  const handleExtractFile = async (entry: BsaFileEntryDto) => {
    // Use save dialog to pick output directory
    const outputDir = await open({
      multiple: false,
      directory: true,
    });
    if (!outputDir || !bsaPath) return;

    try {
      const result = await extractBsaFile(bsaPath, entry.path, outputDir);
      toast.success(t("bsa.extractedTo", { path: result }));
    } catch (e: any) {
      toast.error(`${t("bsa.extractFailed")}: ${e}`);
    }
  };

  const handleExtractFolder = async (folderName: string) => {
    const outputDir = await open({
      multiple: false,
      directory: true,
    });
    if (!outputDir || !bsaPath) return;

    try {
      const results = await extractBsaFolder(bsaPath, folderName, outputDir);
      toast.success(t("bsa.extractedFiles", { count: results.length }));
    } catch (e: any) {
      toast.error(`${t("bsa.extractFailed")}: ${e}`);
    }
  };

  const toggleFolder = (folder: string) => {
    setExpandedFolders((prev) => {
      const next = new Set(prev);
      if (next.has(folder)) {
        next.delete(folder);
      } else {
        next.add(folder);
      }
      return next;
    });
  };

  return (
    <div className="sidepanel">
      {/* Header */}
      {!fileList ? (
        <div className="sidepanel-empty">
          <FileArchive size={36} />
          <p style={{ marginTop: 8 }}>{t("bsa.title")}</p>
          <p className="sidepanel-hint">{t("bsa.subtitle")}</p>
          <button onClick={handleOpen} disabled={loading} className="btn btn-primary" style={{ marginTop: 16 }}>
            <FolderOpen size={16} />
            <span>{loading ? t("bsa.opening") : t("bsa.openBsa")}</span>
          </button>
        </div>
      ) : (
        <>
          <div className="sidepanel-section">
            <h3>{t("bsa.archive")}</h3>
            <div className="sidepanel-row">
              <span className="sidepanel-label">{t("bsa.name")}</span>
              <span className="sidepanel-value file-path" title={fileList.archive_name}>
                {fileList.archive_name}
              </span>
            </div>
            <div className="sidepanel-row">
              <span className="sidepanel-label">{t("bsa.version")}</span>
              <span className="sidepanel-value">0x{fileList.version.toString(16).toUpperCase()}</span>
            </div>
            <div className="sidepanel-row">
              <span className="sidepanel-label">{t("bsa.files")}</span>
              <span className="sidepanel-value">{fileList.total_files.toLocaleString()}</span>
            </div>
            <div className="sidepanel-row">
              <span className="sidepanel-label">{t("bsa.folders")}</span>
              <span className="sidepanel-value">{fileList.folders.length}</span>
            </div>
            <button
              onClick={handleOpen}
              className="btn btn-sm"
              style={{ marginTop: 8, width: "100%" }}
            >
              <FolderOpen size={12} /> {t("bsa.openAnother")}
            </button>
          </div>

          {/* Folder Tree */}
          <div className="sidepanel-section">
            <h3>
              <span onClick={() => setSelectedFolder(null)} style={{ cursor: "pointer" }}>
                {t("bsa.folders")} {selectedFolder && t("bsa.filtered")}
              </span>
            </h3>
            <div className="record-type-row" onClick={() => setSelectedFolder(null)}>
              <span className="sidepanel-label">{t("bsa.allFiles")}</span>
              <span className="sidepanel-value">{fileList.files.length}</span>
            </div>
            {fileList.folders.map((folder) => {
              const count = fileList.files.filter((f) => f.folder === folder).length;
              const isExpanded = expandedFolders.has(folder);
              const isActive = selectedFolder === folder;
              return (
                <div key={folder}>
                  <div
                    className={`record-type-row ${isActive ? "active" : ""}`}
                    onClick={() => {
                      toggleFolder(folder);
                      setSelectedFolder(folder);
                    }}
                    style={{ display: "flex", alignItems: "center", gap: 4 }}
                  >
                    {isExpanded ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
                    <Folder size={12} style={{ color: "var(--accent-gold)", flexShrink: 0 }} />
                    <span className="sidepanel-label" style={{ flex: 1 }}>{folder}</span>
                    <span className="sidepanel-value">{count}</span>
                  </div>
                  {isExpanded && (
                    <div style={{ marginLeft: 20, marginBottom: 4 }}>
                      <button
                        className="btn btn-sm"
                        onClick={(e) => { e.stopPropagation(); handleExtractFolder(folder); }}
                        style={{ fontSize: 10, padding: "2px 8px" }}
                      >
                        <Download size={10} /> {t("bsa.extractAll")}
                      </button>
                    </div>
                  )}
                </div>
              );
            })}
          </div>

          {/* File List */}
          <div className="sidepanel-section">
            <h3>
              {t("bsa.filesCount", { count: filteredFiles.length })}
              {selectedFolder && (
                <span style={{ fontSize: 10, fontWeight: 400, marginLeft: 8 }}>
                  {t("bsa.inFolder", { folder: selectedFolder })}
                </span>
              )}
            </h3>
            <div style={{ maxHeight: 300, overflowY: "auto" }}>
              {filteredFiles.map((entry) => (
                <div
                  key={entry.path}
                  className="record-type-row"
                  style={{ display: "flex", alignItems: "center", gap: 6, fontSize: 11 }}
                  onClick={() => handleExtractFile(entry)}
                  title={`${entry.path}\n${entry.compressed ? t("bsa.compressed") : "Stored"} — ${formatSize(entry.size)}`}
                >
                  <FileText size={12} style={{ color: "var(--text-secondary)", flexShrink: 0 }} />
                  <span style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                    {entry.path.split("/").pop()}
                  </span>
                  <span style={{ color: "var(--text-muted)", fontSize: 10, flexShrink: 0 }}>
                    {formatSize(entry.size)}
                  </span>
                  {entry.compressed && (
                    <span title={t("bsa.compressed")}>
                      <HardDrive size={10} style={{ color: "var(--accent-cyan)", flexShrink: 0 }} />
                    </span>
                  )}
                </div>
              ))}
            </div>
          </div>
        </>
      )}
    </div>
  );
}
