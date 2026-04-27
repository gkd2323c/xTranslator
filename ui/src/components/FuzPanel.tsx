import { useState, useRef } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useTranslation } from "react-i18next";
import { Volume2, Play, Pause, FolderSearch } from "lucide-react";
import toast from "react-hot-toast";
import { scanFuzDirectory, getFuzAudioData } from "../api/strings";
import type { FuzScanResponse, FuzMapping } from "../api/strings";

export function FuzPanel() {
  const { t } = useTranslation();
  const [scanResult, setScanResult] = useState<FuzScanResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [playingId, setPlayingId] = useState<number | null>(null);
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
          <Volume2 size={36} />
          <p style={{ marginTop: 8 }}>{t("fuz.title")}</p>
          <p className="sidepanel-hint">{t("fuz.subtitle")}</p>
          <button onClick={handleScan} disabled={loading} className="btn btn-primary" style={{ marginTop: 16 }}>
            <FolderSearch size={16} />
            <span>{loading ? t("fuz.scanning") : t("fuz.scanDir")}</span>
          </button>
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
            <button onClick={handleScan} disabled={loading} className="btn btn-sm" style={{ marginTop: 8, width: "100%" }}>
              <FolderSearch size={12} /> {t("fuz.rescan")}
            </button>
          </div>

          <div className="sidepanel-section">
            <h3>{t("fuz.matchedCount", { count: scanResult.fuz_mappings.length })}</h3>
            <div style={{ maxHeight: 400, overflowY: "auto" }}>
              {scanResult.fuz_mappings.map((m) => (
                <div
                  key={m.response_id}
                  className="record-type-row"
                  style={{ display: "flex", alignItems: "center", gap: 6, padding: "8px" }}
                >
                  <button
                    onClick={() => handlePlay(m)}
                    className="btn btn-sm btn-ghost"
                    style={{ padding: 2, flexShrink: 0 }}
                    title={playingId === m.response_id ? t("fuz.stop") : t("fuz.play")}
                  >
                    {playingId === m.response_id ? <Pause size={14} /> : <Play size={14} />}
                  </button>
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div style={{ fontSize: 10, color: "var(--text-muted)", fontFamily: "monospace" }}>
                      {m.response_id.toString(16).toUpperCase()}
                    </div>
                    <div style={{ fontSize: 11, color: "var(--text-primary)", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                      {m.dialog_text || t("fuz.noTextMatch")}
                    </div>
                  </div>
                  <span style={{ fontSize: 10, color: "var(--text-secondary)", flexShrink: 0 }}>
                    {m.duration_secs.toFixed(1)}s
                  </span>
                </div>
              ))}
            </div>
          </div>
        </>
      )}
    </div>
  );
}
