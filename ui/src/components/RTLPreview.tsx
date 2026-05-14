import { useState, useCallback, useEffect } from "react";
import { rtlPreview } from "../api/strings";
import { Button } from "./ui/Button";
import { X, RotateCcw, Type } from "lucide-react";

interface RTLPreviewProps {
  isOpen: boolean;
  onClose: () => void;
  initialText?: string;
}

export function RTLPreview({ isOpen, onClose, initialText = "" }: RTLPreviewProps) {
  const [input, setInput] = useState(initialText);
  const [lines, setLines] = useState<string[]>([]);
  const [reverse, setReverse] = useState(true);
  const [shape, setShape] = useState(true);
  const [lineWidth, setLineWidth] = useState(60);

  const doPreview = useCallback(async () => {
    if (!input.trim()) {
      setLines([]);
      return;
    }
    try {
      const result = await rtlPreview(input, reverse, shape, lineWidth);
      setLines(result);
    } catch {
      setLines(["Preview failed"]);
    }
  }, [input, reverse, shape, lineWidth]);

  useEffect(() => {
    if (isOpen && input) {
      const timer = setTimeout(doPreview, 300);
      return () => clearTimeout(timer);
    }
  }, [isOpen, input, reverse, shape, lineWidth, doPreview]);

  useEffect(() => {
    if (isOpen) {
      setInput(initialText);
    }
  }, [isOpen, initialText]);

  if (!isOpen) return null;

  return (
    <div style={{
      position: "fixed", top: 0, left: 0, right: 0, bottom: 0,
      backgroundColor: "rgba(0,0,0,0.6)", zIndex: 1000,
      display: "flex", alignItems: "center", justifyContent: "center"
    }}>
      <div style={{
        backgroundColor: "#1e1e2e", borderRadius: 8, padding: 20,
        width: 700, maxHeight: "80vh", overflow: "auto",
        color: "#cdd6f4", fontFamily: "system-ui, sans-serif"
      }}>
        <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 12 }}>
          <h2 style={{ margin: 0, color: "#cba6f7", fontSize: 16 }}>RTL Preview</h2>
          <Button variant="ghost" size="sm" onClick={onClose}><X size={16} /></Button>
        </div>

        {/* Input */}
        <textarea
          value={input}
          onChange={e => setInput(e.target.value)}
          placeholder="Paste Arabic/Hebrew text..."
          style={{
            width: "100%", height: 80, padding: 8, borderRadius: 4,
            backgroundColor: "#313244", color: "#cdd6f4", border: "1px solid #45475a",
            fontFamily: "monospace", fontSize: 14, resize: "vertical"
          }}
        />

        {/* Controls */}
        <div style={{ display: "flex", gap: 12, margin: "8px 0", alignItems: "center", flexWrap: "wrap" }}>
          <label style={{ display: "flex", alignItems: "center", gap: 4, fontSize: 13 }}>
            <input type="checkbox" checked={reverse} onChange={e => setReverse(e.target.checked)} />
            Reverse
          </label>
          <label style={{ display: "flex", alignItems: "center", gap: 4, fontSize: 13 }}>
            <input type="checkbox" checked={shape} onChange={e => setShape(e.target.checked)} />
            Shape
          </label>
          <label style={{ display: "flex", alignItems: "center", gap: 4, fontSize: 13 }}>
            Width:
            <input
              type="number" value={lineWidth} min={20} max={200}
              onChange={e => setLineWidth(Number(e.target.value))}
              style={{ width: 50, padding: "2px 4px", backgroundColor: "#313244", color: "#cdd6f4", border: "1px solid #45475a", borderRadius: 4 }}
            />
          </label>
          <Button size="sm" onClick={doPreview}><RotateCcw size={14} /> Refresh</Button>
        </div>

        {/* Preview */}
        <div style={{
          backgroundColor: "#11111b", borderRadius: 6, padding: 16,
          minHeight: 120, maxHeight: 400, overflow: "auto",
          fontFamily: "'Segoe UI', 'Arabic Typesetting', Tahoma, sans-serif",
          fontSize: 18, lineHeight: 2, direction: "rtl", textAlign: "right",
          border: "1px solid #45475a"
        }}>
          {lines.length === 0 ? (
            <span style={{ color: "#6c7086" }}>Preview will appear here</span>
          ) : (
            lines.map((line, i) => <div key={i}>{line || "\u00A0"}</div>)
          )}
        </div>

        <div style={{ marginTop: 8, fontSize: 11, color: "#6c7086" }}>
          <Type size={12} style={{ verticalAlign: "middle", marginRight: 4 }} />
          RTL direction · Arabic shaping · Mirror symbols
        </div>
      </div>
    </div>
  );
}
