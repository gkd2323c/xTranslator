import { useState, useEffect, useCallback } from "react";
import { colabGetLabels, colabAssign } from "../api/strings";
import { Button } from "./ui/Button";
import { X, Tag, Plus } from "lucide-react";

const COLORS = [
  "#f38ba8", "#a6e3a1", "#89b4fa", "#f9e2af", "#cba6f7",
  "#94e2d5", "#fab387", "#b4befe", "#f5c2e7", "#a6adc8",
];

interface ColabPanelProps {
  isOpen: boolean;
  onClose: () => void;
  selectedIds: number[];
}

export function ColabPanel({ isOpen, onClose, selectedIds }: ColabPanelProps) {
  const [labels, setLabels] = useState<[number, string][]>([]);

  const refresh = useCallback(async () => {
    try {
      const l = await colabGetLabels();
      setLabels(l);
    } catch { /* ignore */ }
  }, []);

  useEffect(() => {
    if (isOpen) refresh();
  }, [isOpen, refresh]);

  const handleAssign = async (slotId: number) => {
    if (selectedIds.length === 0) return;
    try {
      await colabAssign(selectedIds, slotId);
      refresh();
    } catch { /* ignore */ }
  };

  if (!isOpen) return null;

  return (
    <div style={{
      position: "fixed", top: 0, left: 0, right: 0, bottom: 0,
      backgroundColor: "rgba(0,0,0,0.6)", zIndex: 1000,
      display: "flex", alignItems: "center", justifyContent: "center"
    }}>
      <div style={{
        backgroundColor: "#1e1e2e", borderRadius: 8, padding: 20,
        width: 380, maxHeight: "60vh", overflow: "auto",
        color: "#cdd6f4", fontFamily: "system-ui, sans-serif"
      }}>
        <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 12 }}>
          <h2 style={{ margin: 0, color: "#cba6f7", fontSize: 16 }}>
            <Tag size={16} style={{ verticalAlign: "middle", marginRight: 6 }} />
            Collab Slots
          </h2>
          <Button variant="ghost" size="sm" onClick={onClose}><X size={16} /></Button>
        </div>

        <p style={{ fontSize: 12, color: "#6c7086", marginBottom: 12 }}>
          {selectedIds.length > 0
            ? `Assign ${selectedIds.length} selected string(s) to a collab slot`
            : "Select strings in the table first, then assign them to a slot below"}
        </p>

        {/* New slot button + existing slots */}
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          {[1, 2, 3, 4, 5, 6, 7, 8].map(slotId => {
            const label = labels.find(([id]) => id === slotId)?.[1] || `Slot ${slotId}`;
            const color = COLORS[(slotId - 1) % COLORS.length];
            return (
              <div key={slotId} style={{
                display: "flex", alignItems: "center", gap: 8,
                padding: "6px 10px", borderRadius: 6,
                backgroundColor: "#313244", cursor: selectedIds.length > 0 ? "pointer" : "default",
                opacity: selectedIds.length > 0 ? 1 : 0.6,
              }} onClick={() => handleAssign(slotId)}>
                <span style={{
                  width: 12, height: 12, borderRadius: 3,
                  backgroundColor: color, flexShrink: 0
                }} />
                <span style={{ fontSize: 13, flex: 1 }}>{label}</span>
                <span style={{ fontSize: 11, color: "#6c7086" }}>Slot {slotId}</span>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
