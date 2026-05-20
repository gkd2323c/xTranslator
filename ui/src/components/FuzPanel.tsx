import { useState, useRef } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useTranslation } from "react-i18next";
import { Volume2, Play, Pause, FolderSearch, CheckCircle2, XCircle, AlertTriangle } from "lucide-react";
import toast from "react-hot-toast";
import { scanFuzDirectory, getFuzAudioData } from "../api/strings";
import type { FuzScanResponse, FuzMapping } from "../api/strings";
import { Button, EmptyState } from "./ui";

export function FuzPanel() {
  const { t } = useTranslation();
  const [scanResult, setScanResult] = useState<FuzScanResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [playingId, setPlayingId] = useState<number | null>(null);
  const [filter, setFilter] = useState("");
  const filteredMappings = scanResult
    ? filter
      ? scanResult.fuz_mappings.filter(
          (m) =>
            m.response_id.toString(16).toUpperCase().includes(filter.toUpperCase()) ||
            m.dialog_text.toLowerCase().includes(filter.toLowerCase())
        )
      : scanResult.fuz_mappings
    : [];
  const audioRef = useRef<HTMLAudioElement | null>(null);

  const handleScan = async () => {
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
  };

  const handlePlay = async (mapping: FuzMapping) => {
    if (playingId === mapping.response_id) {
      audioRef.current?.pause();
      setPlayingId(null);
      return;
    }

    try {
      const data = await getFuzAudioData(mapping.fuz_file);
      const blob = new Blob([new Uint8Array(data)], { type: "audio/wav" });
      const url = URL.createObjectURL(blob);

      if (audioRef.current) {
        audioRef.current.pause();
        URL.revokeObjectURL(audioRef.current.src);
      }

      const audio = new Audio(url);
      audio.onended = () => setPlayingId(null);
      audio.onerror = () => toast.error(t("fuz.playbackFailed"));
      audio.play();
      audioRef.current = audio;
      setPlayingId(mapping.response_id);
    } catch {
      toast.error(t("fuz.loadAudioFailed"));
    }
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
          <div className="sidepanel-section">
            <h3>{t("fuz.voiceFiles")}</h3>
            <div className="sidepanel-row">
              <span className="sidepanel-label">{t("fuz.totalFuz")}</span>
              <span className="sidepanel-value">{scanResult.total_fuz_files.toLocaleString()}</span>
            </div>
            <div className="sidepanel-row">
              <span className="sidepanel-label">{t("fuz.matched")}</span>
              <span className="sidepanel-value">{scanResult.fuz_mappings.length.toLocaleString()}</span>
            </div>
            <div className="sidepanel-row">
              <span className="sidepanel-label">{t("fuz.withLip", { defaultValue: "With LIP" })}</span>
              <span className="sidepanel-value">
                {scanResult.fuz_mappings.filter((m) => m.parse_ok && m.has_lip).length.toLocaleString()}
              </span>
            </div>
            <div className="sidepanel-row">
              <span className="sidepanel-label">{t("fuz.withoutLip", { defaultValue: "Without LIP" })}</span>
              <span className="sidepanel-value">
                {scanResult.fuz_mappings.filter((m) => m.parse_ok && !m.has_lip).length.toLocaleString()}
              </span>
            </div>
            <div className="sidepanel-row">
              <span className="sidepanel-label">{t("fuz.parseFailed", { defaultValue: "Parse failed" })}</span>
              <span className="sidepanel-value">
                {scanResult.fuz_mappings.filter((m) => !m.parse_ok).length.toLocaleString()}
              </span>
            </div>
            <Button variant="default" size="sm" onClick={handleScan} disabled={loading} icon={<FolderSearch size={12} />} className="fuz-rescan-btn">
              {t("fuz.rescan")}
            </Button>
          </div>

          {/* Filter */}
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

          <div className="sidepanel-section">
            <h3>{t("fuz.matchedCount", { count: filteredMappings.length })}</h3>
            <div style={{ maxHeight: 360, overflowY: "auto" }}>
              {filteredMappings.map((m) => (
                <div
                  key={m.response_id}
                  className="record-type-row fuz-mapping-row"
                >
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => handlePlay(m)}
                    title={playingId === m.response_id ? t("fuz.stop") : t("fuz.play")}
                    icon={playingId === m.response_id ? <Pause size={14} /> : <Play size={14} />}
                  />
                  <div className="fuz-mapping-info">
                    <div className="fuz-mapping-id">
                      {m.response_id.toString(16).toUpperCase()}
                    </div>
                    <div className="fuz-mapping-text">
                      {m.dialog_text || t("fuz.noTextMatch")}
                    </div>
                  </div>
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
              ))}
              {filteredMappings.length === 0 && (
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
