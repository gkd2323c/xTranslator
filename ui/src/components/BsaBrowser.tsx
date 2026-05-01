import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useTranslation } from "react-i18next";
import {
  FolderOpen, FileText, Download, Folder,
  HardDrive, FileArchive, ChevronRight, ChevronDown
} from "lucide-react";
import toast from "react-hot-toast";
import { listBsaFiles, listBa2Files, extractBsaFile, extractBa2File, extractBsaFolder, extractBa2Folder } from "../api/strings";
import type { BsaFileListDto, BsaFileEntryDto } from "../api/strings";
import { Button, EmptyState } from "./ui";

function formatSize(bytes: number): string {
  if (bytes >= 1_048_576) return `${(bytes / 1_048_576).toFixed(1)} MB`;
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${bytes} B`;
}

export function BsaBrowser() {
  const { t } = useTranslation();
  const [archivePath, setArchivePath] = useState<string | null>(null);
  const [archiveType, setArchiveType] = useState<"bsa" | "ba2" | null>(null);
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
        { name: "BSA Archive (Skyrim)", extensions: ["bsa"] },
        { name: "BA2 Archive (Fallout/Starfield)", extensions: ["ba2"] },
        { name: "All", extensions: ["*"] },
      ],
    });
    if (!path) return;

    setLoading(true);
    setSelectedFolder(null);
    try {
      const ext = path.split('.').pop()?.toLowerCase();
      const list = ext === 'ba2'
        ? await listBa2Files(path)
        : await listBsaFiles(path);
      setFileList(list);
      setArchivePath(path);
      setArchiveType(ext === 'ba2' ? 'ba2' : 'bsa');
      setExpandedFolders(new Set(list.folders));
    } catch (e: any) {
      toast.error(`${t("bsa.failedToOpen")}: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  const handleExtractFile = async (entry: BsaFileEntryDto) => {
    const outputDir = await open({
      multiple: false,
      directory: true,
    });
    if (!outputDir || !archivePath || !archiveType) return;

    try {
      const result = archiveType === 'ba2'
        ? await extractBa2File(archivePath, entry.path, outputDir)
        : await extractBsaFile(archivePath, entry.path, outputDir);
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
    if (!outputDir || !archivePath || !archiveType) return;

    try {
      const results = archiveType === 'ba2'
        ? await extractBa2Folder(archivePath, folderName, outputDir)
        : await extractBsaFolder(archivePath, folderName, outputDir);
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
          <EmptyState
            icon={<FileArchive size={36} />}
            title={t("bsa.title")}
            hint={t("bsa.subtitle")}
          />
          <Button variant="primary" onClick={handleOpen} disabled={loading} icon={<FolderOpen size={16} />} className="bsa-open-btn">
            {loading ? t("bsa.opening") : t("bsa.openBsa")}
          </Button>
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
            <Button
              variant="default"
              size="sm"
              onClick={handleOpen}
              icon={<FolderOpen size={12} />}
              className="bsa-open-another-btn"
            >
              {t("bsa.openAnother")}
            </Button>
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
                    className={`record-type-row bsa-folder-row ${isActive ? "active" : ""}`}
                    onClick={() => {
                      toggleFolder(folder);
                      setSelectedFolder(folder);
                    }}
                  >
                    {isExpanded ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
                    <Folder size={12} className="bsa-folder-icon" />
                    <span className="sidepanel-label bsa-folder-name">{folder}</span>
                    <span className="sidepanel-value">{count}</span>
                  </div>
                  {isExpanded && (
                    <div className="bsa-extract-folder">
                      <Button
                        variant="default"
                        size="xs"
                        onClick={(e) => { e.stopPropagation(); handleExtractFolder(folder); }}
                        icon={<Download size={10} />}
                      >
                        {t("bsa.extractAll")}
                      </Button>
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
                  className="record-type-row bsa-file-row"
                  onClick={() => handleExtractFile(entry)}
                  title={`${entry.path}\n${entry.compressed ? t("bsa.compressed") : "Stored"} — ${formatSize(entry.size)}`}
                >
                  <FileText size={12} className="bsa-file-icon" />
                  <span className="bsa-file-name-cell">
                    {entry.path.split("/").pop()}
                  </span>
                  <span className="bsa-file-size">
                    {formatSize(entry.size)}
                  </span>
                  {entry.compressed && (
                    <span title={t("bsa.compressed")}>
                      <HardDrive size={10} className="bsa-compressed-icon" />
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
