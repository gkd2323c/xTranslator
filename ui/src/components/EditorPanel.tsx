import { useState, useEffect, useCallback } from "react";
import { useAppStore } from "../stores/appStore";
import { updateTranslation, heuristicSearch, translateString, setApiKey, type HeuristicMatchDTO } from "../api/strings";
import { Save, X, Type, Search, Copy, Languages, Key } from "lucide-react";
import toast from "react-hot-toast";

export function EditorPanel() {
  const selectedItem = useAppStore((s) => s.selectedItem);
  const selectedId = useAppStore((s) => s.selectedId);
  const language = useAppStore((s) => s.language);
  const targetLang = useAppStore((s) => s.targetLang);
  const updateItemTranslation = useAppStore((s) => s.updateItemTranslation);
  const setSelectedById = useAppStore((s) => s.setSelectedById);

  const [localTrans, setLocalTrans] = useState("");
  const [isSaving, setIsSaving] = useState(false);
  const [isSearching, setIsSearching] = useState(false);
  const [isTranslating, setIsTranslating] = useState(false);
  const [matches, setMatches] = useState<HeuristicMatchDTO[]>([]);
  const [showApiKeyDialog, setShowApiKeyDialog] = useState(false);
  const [apiKeyInput, setApiKeyInput] = useState("");

  useEffect(() => {
    setLocalTrans(selectedItem?.translation || "");
    setMatches([]);
  }, [selectedId]);

  const handleSave = useCallback(async () => {
    if (selectedId === null || !selectedItem) return;
    setIsSaving(true);
    try {
      await updateTranslation(selectedItem.id, localTrans);
      updateItemTranslation(selectedItem.id, localTrans);
      toast.success("Translation saved");
    } catch (e: any) {
      toast.error(`Failed to save: ${e}`);
    } finally {
      setIsSaving(false);
    }
  }, [selectedId, selectedItem, localTrans, updateItemTranslation]);

  const handleHeuristicSearch = useCallback(async () => {
    if (!selectedItem || selectedItem.status === "translated") return;
    setIsSearching(true);
    try {
      const results = await heuristicSearch({
        source: selectedItem.source,
        min_similarity: 0.4,
        max_results: 5,
      });
      setMatches(results);
      if (results.length === 0) toast("No similar translations found");
    } catch (e: any) {
      toast.error(`Search failed: ${e}`);
    } finally {
      setIsSearching(false);
    }
  }, [selectedItem]);

  const handleTranslate = useCallback(async () => {
    if (!selectedItem) return;
    setIsTranslating(true);
    try {
      const result = await translateString({
        text: selectedItem.source,
        source_lang: language,
        target_lang: targetLang,
      });
      setLocalTrans(result);
      toast.success("Machine translation completed");
    } catch (e: any) {
      if (e.includes("API key not set")) {
        setShowApiKeyDialog(true);
      } else {
        toast.error(`Translation failed: ${e}`);
      }
    } finally {
      setIsTranslating(false);
    }
  }, [selectedItem]);

  const handleSetApiKey = useCallback(async () => {
    if (!apiKeyInput.trim()) {
      toast.error("API key cannot be empty");
      return;
    }
    try {
      await setApiKey(apiKeyInput.trim());
      toast.success("API key saved");
      setShowApiKeyDialog(false);
      setApiKeyInput("");
    } catch (e: any) {
      toast.error(`Failed to set API key: ${e}`);
    }
  }, [apiKeyInput]);

  const applyMatch = (translation: string) => {
    setLocalTrans(translation);
    toast.success("Translation copied from match");
  };

  // F2 快捷键
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "F2" && selectedItem) {
        const textarea = document.querySelector(".editor-textarea") as HTMLTextAreaElement;
        textarea?.focus();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [selectedItem]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.ctrlKey && e.key === "Enter") handleSave();
  };

  if (!selectedItem) {
    return (
      <div className="editor-panel editor-empty">
        <Type size={24} opacity={0.3} />
        <p>Select a string to edit</p>
      </div>
    );
  }

  return (
    <div className="editor-panel">
      <div className="editor-header">
        <div className="editor-meta">
          <span className="editor-id">#{selectedItem.id}</span>
          <span className="editor-sig">
            {selectedItem.record_sig}:{selectedItem.field_sig}
          </span>
          <span className="editor-formid">{selectedItem.form_id}</span>
          <span className={`editor-status-badge badge-${selectedItem.status}`}>
            {selectedItem.status}
          </span>
        </div>
        <div className="editor-actions">
          <button onClick={() => setShowApiKeyDialog(true)} className="btn btn-ghost btn-sm" title="Set API Key">
            <Key size={14} />
          </button>
          {selectedItem.status !== "translated" && (
            <>
              <button onClick={handleTranslate} disabled={isTranslating} className="btn btn-sm" title="Machine translate (OpenAI)">
                <Languages size={14} />
                <span>{isTranslating ? "Translating..." : "Translate"}</span>
              </button>
              <button onClick={handleHeuristicSearch} disabled={isSearching} className="btn btn-sm" title="Find similar translations">
                <Search size={14} />
                <span>{isSearching ? "Searching..." : "Similar"}</span>
              </button>
            </>
          )}
          <button onClick={handleSave} disabled={isSaving} className="btn btn-primary btn-sm" title="Ctrl+Enter">
            <Save size={14} />
            <span>Save</span>
          </button>
          <button onClick={() => setSelectedById(null)} className="btn btn-ghost btn-sm">
            <X size={14} />
          </button>
        </div>
      </div>

      <div className="editor-body">
        <div className="editor-source">
          <label>Source</label>
          <div className="editor-source-text">{selectedItem.source}</div>
        </div>
        <div className="editor-translation">
          <label>Translation</label>
          <textarea
            value={localTrans}
            onChange={(e) => setLocalTrans(e.target.value)}
            onKeyDown={handleKeyDown}
            rows={3}
            className="editor-textarea"
            placeholder="Enter translation..."
            autoFocus
          />
        </div>
        {matches.length > 0 && (
          <div className="editor-matches">
            <label>Similar Translations</label>
            <div className="matches-list">
              {matches.map((m, i) => (
                <div key={i} className="match-item" onClick={() => applyMatch(m.translation)}>
                  <div className="match-source" title={m.source}>{m.source}</div>
                  <div className="match-translation">{m.translation}</div>
                  <div className="match-meta">
                    <span className="match-sim">{(m.similarity * 100).toFixed(0)}%</span>
                    <Copy size={12} />
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>

      {showApiKeyDialog && (
        <div className="dialog-overlay" onClick={() => setShowApiKeyDialog(false)}>
          <div className="dialog-content" onClick={(e) => e.stopPropagation()}>
            <h3>Set Translation API Key</h3>
            <p className="dialog-hint">
              Supports OpenAI-compatible APIs (OpenAI, DeepSeek, etc.).
              You can also set <code>XT_TRANSLATE_API_KEY</code> environment variable.
            </p>
            <input
              type="password"
              value={apiKeyInput}
              onChange={(e) => setApiKeyInput(e.target.value)}
              placeholder="sk-..."
              className="dialog-input"
              onKeyDown={(e) => { if (e.key === "Enter") handleSetApiKey(); }}
            />
            <div className="dialog-actions">
              <button onClick={() => setShowApiKeyDialog(false)} className="btn btn-ghost">Cancel</button>
              <button onClick={handleSetApiKey} className="btn btn-primary">Save</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
