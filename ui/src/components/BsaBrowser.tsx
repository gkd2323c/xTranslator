import { useState, useMemo } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useTranslation } from "react-i18next";
import {
  FolderOpen, FileText, Download, Folder,
  HardDrive, FileArchive, ChevronRight, ChevronDown, Search, X, Eye
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

function isTextExtension(name: string): boolean {
  const ext = name.split(".").pop()?.toLowerCase();
  return ["txt", "xml", "json", "html", "htm", "css", "js", "psc", "pas", "cpp", "h", "py", "lua", "cfg", "ini", "bat", "sh"].includes(ext || "");
}

export function BsaBrowser() {
  const { t } = useTranslation();
  const [archivePath, setArchivePath] = useState<string | null>(null);
  const [archiveType, setArchiveType] = useState<"bsa" | "ba2" | null>(null);
  const [fileList, setFileList] = useState<BsaFileListDto | null>(null);
  const [loading, setLoading] = useState(false);
  const [selectedFolder, setSelectedFolder] = useState<string | null>(null);
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(new Set());
  const [fileSearch, setFileSearch] = useState("");
  const [previewFile, setPreviewFile] = useState<BsaFileEntryDto | null>(null);

  // 搜索过滤 + 文件夹过滤
  const filteredFiles = useMemo(() => {
    let files = fileList?.files ?? [];
    if (selectedFolder) {
      files = files.filter((f) => f.folder === selectedFolder);
    }
    if (fileSearch) {
      const q = fileSearch.toLowerCase();
      files = files.filter((f) => f.path.toLowerCase().includes(q) || f.folder.toLowerCase().includes(q));
    }
    return files;
  }, [fileList, selectedFolder, fileSearch]);

  // 搜索时自动展开匹配文件夹
  const autoExpanded = useMemo(() => {
    if (!fileSearch || !fileList) return new Set<string>();
    const q = fileSearch.toLowerCase();
    const matching = new Set<string>();
    fileList.files.forEach((f) => {
      if (f.path.toLowerCase().includes(q)) matching.add(f.folder);
    });
    return matching;
  }, [fileSearch, fileList]);

  const effectiveExpanded = fileSearch ? autoExpanded : expandedFolders;

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
    setFileSearch("");
    setPreviewFile(null);
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
      if (next.has(folder)) next.delete(folder); else next.add(folder);
      return next;
    });
  };

  return (
    <div className="sidepanel">
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
          {/* Archive Info */}
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
              <span onClick={() => { setSelectedFolder(null); setFileSearch(""); }} style={{ cursor: "pointer" }}>
                {t("bsa.folders")} {selectedFolder && t("bsa.filtered")}
              </span>
            </h3>
            <div className="record-type-row" onClick={() => { setSelectedFolder(null); setFileSearch(""); }}>
              <span className="sidepanel-label">{t("bsa.allFiles")}</span>
              <span className="sidepanel-value">{fileList.files.length}</span>
            </div>
            {fileList.folders.map((folder) => {
              const count = fileList.files.filter((f) => f.folder === folder).length;
              const isExpanded = effectiveExpanded.has(folder);
              const isActive = selectedFolder === folder;
              return (
                <div key={folder}>
                  <div
                    className={`record-type-row bsa-folder-row ${isActive ? "active" : ""}`}
                    onClick={() => {
                      setSelectedFolder(folder);
                      setPreviewFile(null);
                      if (!fileSearch) toggleFolder(folder);
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

          {/* File Search */}
          <div className="sidepanel-section" style={{ padding: "4px 8px" }}>
            <div className="bsa-search-bar">
              <Search size={12} className="bsa-search-icon" />
              <input
                type="text"
                className="bsa-search-input"
                placeholder={t("bsa.searchFiles", { defaultValue: "Search files..." })}
                value={fileSearch}
                onChange={(e) => setFileSearch(e.target.value)}
              />
              {fileSearch && (
                <button className="bsa-search-clear" onClick={() => setFileSearch("")}>
                  <X size={12} />
                </button>
              )}
            </div>
          </div>

          {/* File List + Preview side-by-side */}
          <div className="bsa-file-preview-split">
            {/* File List */}
            <div className="bsa-file-list-panel">
              <div className="bsa-file-list-header">
                {t("bsa.filesCount", { count: filteredFiles.length })}
                {fileSearch && ` (${fileList.files.length})`}
              </div>
              <div className="bsa-file-list-scroll">
                {filteredFiles.length === 0 ? (
                  <div className="bsa-file-list-empty">{t("bsa.noMatch", { defaultValue: "No matching files" })}</div>
                ) : (
                  filteredFiles.map((entry) => {
                    const isSelected = previewFile?.path === entry.path;
                    const name = entry.path.split("/").pop() || entry.path;
                    return (
                      <div
                        key={entry.path}
                        className={`bsa-file-row ${isSelected ? "bsa-file-row-selected" : ""}`}
                        onClick={() => setPreviewFile(isSelected ? null : entry)}
                        onDoubleClick={() => handleExtractFile(entry)}
                        title={`${entry.path}\n${entry.compressed ? t("bsa.compressed") : "Stored"} — ${formatSize(entry.size)}\n${t("bsa.doubleClickExtract", { defaultValue: "Double-click to extract" })}`}
                      >
                        <FileText size={11} className="bsa-file-icon" />
                        <span className="bsa-file-name-cell">
                          {fileSearch ? highlightMatch(name, fileSearch) : name}
                        </span>
                        <span className="bsa-file-size">{formatSize(entry.size)}</span>
                        {entry.compressed && (
                          <HardDrive size={9} className="bsa-compressed-icon" />
                        )}
                      </div>
                    );
                  })
                )}
              </div>
            </div>

            {/* Preview Panel */}
            {previewFile && (
              <div className="bsa-preview-panel">
                <div className="bsa-preview-toolbar">
                  <span className="bsa-preview-title">
                    <Eye size={12} />
                    {t("bsa.preview", { defaultValue: "Preview" })}
                  </span>
                  <button className="bsa-preview-close" onClick={() => setPreviewFile(null)}>
                    <X size={12} />
                  </button>
                </div>
                <div className="bsa-preview-content">
                  <div className="bsa-preview-info-row">
                    <span className="bsa-preview-label">{t("bsa.fileName", { defaultValue: "Name" })}</span>
                    <span className="bsa-preview-value">{previewFile.path.split("/").pop()}</span>
                  </div>
                  <div className="bsa-preview-info-row">
                    <span className="bsa-preview-label">{t("bsa.fullPath", { defaultValue: "Path" })}</span>
                    <span className="bsa-preview-value mono">{previewFile.path}</span>
                  </div>
                  <div className="bsa-preview-info-row">
                    <span className="bsa-preview-label">{t("bsa.fileSize", { defaultValue: "Size" })}</span>
                    <span className="bsa-preview-value">{formatSize(previewFile.size)}</span>
                  </div>
                  <div className="bsa-preview-info-row">
                    <span className="bsa-preview-label">{t("bsa.folder", { defaultValue: "Folder" })}</span>
                    <span className="bsa-preview-value">{previewFile.folder}</span>
                  </div>
                  <div className="bsa-preview-info-row">
                    <span className="bsa-preview-label">{t("bsa.compression", { defaultValue: "Compression" })}</span>
                    <span className="bsa-preview-value">{previewFile.compressed ? t("bsa.compressed") : "Stored"}</span>
                  </div>
                  <div className="bsa-preview-type-hint">
                    {isTextExtension(previewFile.path) ? (
                      <span className="bsa-preview-badge bsa-preview-badge-text">{t("bsa.textFile", { defaultValue: "Text file" })}</span>
                    ) : (
                      <span className="bsa-preview-badge bsa-preview-badge-binary">{t("bsa.binaryFile", { defaultValue: "Binary file" })}</span>
                    )}
                  </div>
                  <div className="bsa-preview-actions">
                    <Button variant="default" size="xs" onClick={() => handleExtractFile(previewFile)} icon={<Download size={10} />}>
                      {t("bsa.extract")}
                    </Button>
                  </div>
                </div>
              </div>
            )}
          </div>
        </>
      )}
    </div>
  );
}

/** 搜索高亮 */
function highlightMatch(text: string, query: string): string {
  if (!query) return text;
  const escaped = query.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const parts = text.split(new RegExp(`(${escaped})`, "gi"));
  return parts
    .map((part) =>
      part.toLowerCase() === query.toLowerCase()
        ? `<mark class="bsa-search-mark">${part}</mark>`
        : part
    )
    .join("");
}
