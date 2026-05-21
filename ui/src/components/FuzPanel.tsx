import { useState, useRef, useMemo, useCallback, useEffect } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useTranslation } from "react-i18next";
import { Volume2, Play, Pause, FolderSearch, CheckCircle2, XCircle, AlertTriangle, ArrowUp, ArrowDown, ArrowUpDown } from "lucide-react";
import toast from "react-hot-toast";
import { scanFuzDirectory, getFuzAudioData } from "../api/strings";
import type { FuzScanResponse, FuzMapping } from "../api/strings";
import { Button, EmptyState } from "./ui";

type SortField = "response_id" | "duration_secs" | "status";
type SortDir = "asc" | "desc" | null;

// ── Sort helpers ──────────────────────────────────────────────────

function sortMappings(
  mappings: FuzMapping[],
  field: SortField,
  dir: SortDir
): FuzMapping[] {
  if (!dir || !field) return mappings;
  return [...mappings].sort((a, b) => {
    let cmp: number;
    switch (field) {
      case "response_id":
        cmp = a.response_id - b.response_id;
        break;
      case "duration_secs":
        cmp = a.duration_secs - b.duration_secs;
        break;
      case "status": {
        const aOk = a.parse_ok ? (a.has_lip ? 2 : 1) : 0;
        const bOk = b.parse_ok ? (b.has_lip ? 2 : 1) : 0;
        cmp = aOk - bOk;
        break;
      }
      default:
        cmp = 0;
    }
    return dir === "asc" ? cmp : -cmp;
  });
}

// ── Main component ─────────────────────────────────────────────────

export function FuzPanel() {
  const { t } = useTranslation();
  const [scanResult, setScanResult] = useState<FuzScanResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [playingId, setPlayingId] = useState<number | null>(null);
  const [playProgress, setPlayProgress] = useState(0); // 0–1
  const [playCurrentTime, setPlayCurrentTime] = useState(0);
  const [filter, setFilter] = useState("");
  const [sortField, setSortField] = useState<SortField>("response_id");
  const [sortDir, setSortDir] = useState<SortDir>("asc");

  const audioRef = useRef<HTMLAudioElement | null>(null);
  const animFrameRef = useRef<number | null>(null);

  // ── Scan ──────────────────────────────────────────────────────────

  const handleScan = useCallback(async () => {
    const dir = await open({
      multiple: false,
      directory: true,
    });
    if (!dir) return;

    setLoading(true);
    try {
      const result = await scanFuzDirectory(dir);
      setScanResult(result);
      toast.success(t("fuz.foundMatches", { matched: result.fuz_mappings.length, total: result.total_fuz_files }));
    } catch (e: any) {
      toast.error(`${t("fuz.scanFailed")}: ${e}`);
    } finally {
      setLoading(false);
    }
  }, [t]);

  // ── Stats ────────────────────────────────────────────────────────

  const stats = useMemo(() => {
    if (!scanResult) return null;
    const total = scanResult.fuz_mappings.length;
    const withLip = scanResult.fuz_mappings.filter((m) => m.parse_ok && m.has_lip).length;
    const withoutLip = scanResult.fuz_mappings.filter((m) => m.parse_ok && !m.has_lip).length;
    const parseFailed = scanResult.fuz_mappings.filter((m) => !m.parse_ok).length;
    return { total, withLip, withoutLip, parseFailed };
  }, [scanResult]);

  // ── Filtered & sorted entries ───────────────────────────────────

  const displayMappings = useMemo(() => {
    if (!scanResult) return [];
    let items = scanResult.fuz_mappings;
    if (filter) {
      const q = filter.toUpperCase();
      items = items.filter(
        (m) =>
          m.response_id.toString(16).toUpperCase().includes(q) ||
          m.dialog_text.toLowerCase().includes(filter.toLowerCase())
      );
    }
    return sortMappings(items, sortField, sortDir);
  }, [scanResult, filter, sortField, sortDir]);

  // ── Audio playback ──────────────────────────────────────────────

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (animFrameRef.current) cancelAnimationFrame(animFrameRef.current);
      if (audioRef.current) {
        audioRef.current.pause();
        URL.revokeObjectURL(audioRef.current.src);
      }
    };
  }, []);

  const updateProgress = useCallback(() => {
    const audio = audioRef.current;
    if (!audio || audio.paused || audio.ended) {
      setPlayProgress(0);
      setPlayCurrentTime(0);
      return;
    }
    const progress = audio.currentTime / (audio.duration || 1);
    setPlayProgress(progress);
    setPlayCurrentTime(audio.currentTime);
    animFrameRef.current = requestAnimationFrame(updateProgress);
  }, []);

  const stopPlayback = useCallback(() => {
    if (audioRef.current) {
      audioRef.current.pause();
      URL.revokeObjectURL(audioRef.current.src);
    }
    if (animFrameRef.current) cancelAnimationFrame(animFrameRef.current);
    setPlayingId(null);
    setPlayProgress(0);
    setPlayCurrentTime(0);
  }, []);

  const handlePlay = useCallback(async (mapping: FuzMapping) => {
    if (playingId === mapping.response_id) {
      stopPlayback();
      return;
    }

    try {
      const data = await getFuzAudioData(mapping.fuz_file);
      const blob = new Blob([new Uint8Array(data)], { type: "audio/wav" });
      const url = URL.createObjectURL(blob);

      // Stop previous playback
      if (audioRef.current) {
        audioRef.current.pause();
        URL.revokeObjectURL(audioRef.current.src);
      }
      if (animFrameRef.current) cancelAnimationFrame(animFrameRef.current);

      const audio = new Audio(url);
      audio.onended = () => {
        setPlayingId(null);
        setPlayProgress(0);
        setPlayCurrentTime(0);
      };
      audio.onerror = () => {
        toast.error(t("fuz.playbackFailed"));
        stopPlayback();
      };
      audio.play();
      audioRef.current = audio;
      setPlayingId(mapping.response_id);
      setPlayProgress(0);
      setPlayCurrentTime(0);
      animFrameRef.current = requestAnimationFrame(updateProgress);
    } catch {
      toast.error(t("fuz.loadAudioFailed"));
    }
  }, [playingId, stopPlayback, updateProgress, t]);

  const handleSortToggle = useCallback((field: SortField) => {
    setSortField((prev) => {
      if (prev !== field) return field;
      return field;
    });
    setSortDir((prev) => {
      if (prev === null) return "asc";
      if (prev === "asc") return "desc";
      return null;
    });
  }, []);

  // ── Render ──────────────────────────────────────────────────────

  const SortIcon = sortDir === "asc" ? ArrowUp : sortDir === "desc" ? ArrowDown : ArrowUpDown;

  const formatTime = (secs: number) => {
    const m = Math.floor(secs / 60);
    const s = Math.floor(secs % 60);
    return `${m}:${s.toString().padStart(2, "0")}`;
  };

  return (
    <div className="sidepanel">
      {!scanResult ? (
        <div className="sidepanel-empty">
          <EmptyState
            icon={<Volume2 size={36} />}
            title={t("fuz.title")}
            hint={t("fuz.subtitle")}
          />
          <Button variant="primary" onClick={handleScan} disabled={loading} icon={<FolderSearch size={16} />} className="fuz-scan-btn">
            {loading ? t("fuz.scanning") : t("fuz.scanDir")}
          </Button>
        </div>
      ) : (
        <>
          {/* ── Stats section ───────────────────────────────────── */}
          <div className="sidepanel-section">
            <h3>{t("fuz.voiceFiles")}</h3>
            <div className="sidepanel-row">
              <span className="sidepanel-label">{t("fuz.totalFuz")}</span>
              <span className="sidepanel-value">{scanResult.total_fuz_files.toLocaleString()}</span>
            </div>
            <div className="sidepanel-row">
              <span className="sidepanel-label">{t("fuz.matched")}</span>
              <span className="sidepanel-value">{stats?.total.toLocaleString()}</span>
            </div>

            {/* Stats mini progress bars */}
            {stats && (
              <div className="fuz-stats-bars">
                <div className="fuz-stats-row">
                  <span className="fuz-stats-label">{t("fuz.withLip", { defaultValue: "With LIP" })}</span>
                  <span className="fuz-stats-value">{stats.withLip}</span>
                  <div className="fuz-stats-track">
                    <div
                      className="fuz-stats-fill fuz-stats-fill-ok"
                      style={{ width: `${stats.total > 0 ? (stats.withLip / stats.total) * 100 : 0}%` }}
                    />
                  </div>
                </div>
                <div className="fuz-stats-row">
                  <span className="fuz-stats-label">{t("fuz.withoutLip", { defaultValue: "Without LIP" })}</span>
                  <span className="fuz-stats-value">{stats.withoutLip}</span>
                  <div className="fuz-stats-track">
                    <div
                      className="fuz-stats-fill fuz-stats-fill-warn"
                      style={{ width: `${stats.total > 0 ? (stats.withoutLip / stats.total) * 100 : 0}%` }}
                    />
                  </div>
                </div>
                <div className="fuz-stats-row">
                  <span className="fuz-stats-label">{t("fuz.parseFailed", { defaultValue: "Parse failed" })}</span>
                  <span className="fuz-stats-value">{stats.parseFailed}</span>
                  <div className="fuz-stats-track">
                    <div
                      className="fuz-stats-fill fuz-stats-fill-error"
                      style={{ width: `${stats.total > 0 ? (stats.parseFailed / stats.total) * 100 : 0}%` }}
                    />
                  </div>
                </div>
              </div>
            )}

            <Button variant="default" size="sm" onClick={handleScan} disabled={loading} icon={<FolderSearch size={12} />} className="fuz-rescan-btn">
              {t("fuz.rescan")}
            </Button>
          </div>

          {/* ── Filter ──────────────────────────────────────────── */}
          <div className="sidepanel-section" style={{ padding: "4px 8px" }}>
            <input
              type="text"
              placeholder={t("fuz.filterPlaceholder", { defaultValue: "Filter by ID or text..." })}
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              className="filter-input"
              style={{ width: "100%" }}
            />
          </div>

          {/* ── Sort bar ────────────────────────────────────────── */}
          <div className="fuz-sort-bar">
            <button
              className={`fuz-sort-btn ${sortField === "response_id" ? "fuz-sort-active" : ""}`}
              onClick={() => handleSortToggle("response_id")}
            >
              <SortIcon size={10} />
              ID
            </button>
            <button
              className={`fuz-sort-btn ${sortField === "duration_secs" ? "fuz-sort-active" : ""}`}
              onClick={() => handleSortToggle("duration_secs")}
            >
              <SortIcon size={10} />
              {t("fuz.duration", { defaultValue: "Duration" })}
            </button>
            <button
              className={`fuz-sort-btn ${sortField === "status" ? "fuz-sort-active" : ""}`}
              onClick={() => handleSortToggle("status")}
            >
              <SortIcon size={10} />
              {t("fuz.status", { defaultValue: "Status" })}
            </button>
            <span className="fuz-sort-count">{displayMappings.length} / {scanResult.fuz_mappings.length}</span>
          </div>

          {/* ── Entry list ──────────────────────────────────────── */}
          <div className="sidepanel-section">
            <div style={{ maxHeight: 360, overflowY: "auto" }}>
              {displayMappings.map((m) => {
                const fileName = m.fuz_file.replace(/\\/g, "/").split("/").pop() || "";
                return (
                  <div
                    key={m.response_id}
                    className={`record-type-row fuz-mapping-row ${playingId === m.response_id ? "fuz-mapping-playing" : ""}`}
                  >
                    {/* Play button */}
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handlePlay(m)}
                      title={playingId === m.response_id ? t("fuz.stop") : t("fuz.play")}
                      icon={playingId === m.response_id ? <Pause size={14} /> : <Play size={14} />}
                    />

                    {/* Info */}
                    <div className="fuz-mapping-info">
                      <div className="fuz-mapping-id" title={`#${m.response_id.toString(16).toUpperCase()}`}>
                        #{m.response_id.toString(16).toUpperCase()}
                      </div>
                      <div className="fuz-mapping-filename" title={m.fuz_file}>
                        {fileName}
                      </div>
                      <div className="fuz-mapping-text">
                        {m.dialog_text || <em className="esp-compare-empty">{t("fuz.noTextMatch")}</em>}
                      </div>

                      {/* Playback progress */}
                      {playingId === m.response_id && (
                        <div className="fuz-playback-progress">
                          <div className="fuz-playback-track">
                            <div
                              className="fuz-playback-fill"
                              style={{ width: `${playProgress * 100}%` }}
                            />
                          </div>
                          <span className="fuz-playback-time">
                            {formatTime(playCurrentTime)} / {formatTime(m.duration_secs)}
                          </span>
                        </div>
                      )}
                    </div>

                    {/* Meta */}
                    <div className="fuz-mapping-meta">
                      <span className="fuz-mapping-duration">
                        {m.duration_secs.toFixed(1)}s
                      </span>
                      {!m.parse_ok ? (
                        <span title={t("fuz.parseFailed", { defaultValue: "Parse failed" })}>
                          <AlertTriangle size={12} className="fuz-parse-error" />
                        </span>
                      ) : m.has_lip ? (
                        <span title={t("fuz.hasLip", { defaultValue: "Has LIP data" })}>
                          <CheckCircle2 size={12} className="fuz-lip-yes" />
                        </span>
                      ) : (
                        <span title={t("fuz.noLip", { defaultValue: "No LIP data" })}>
                          <XCircle size={12} className="fuz-lip-no" />
                        </span>
                      )}
                    </div>
                  </div>
                );
              })}
              {displayMappings.length === 0 && (
                <div className="sidepanel-hint" style={{ padding: 16, textAlign: "center" }}>
                  {t("fuz.noMatch", { defaultValue: "No matching entries" })}
                </div>
              )}
            </div>
          </div>
        </>
      )}
    </div>
  );
}
