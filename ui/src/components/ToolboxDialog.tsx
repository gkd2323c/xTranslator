import { useState } from "react";
import { toolboxTransform } from "../api/strings";
import toast from "react-hot-toast";
import { Button, Modal, Select } from "./ui";

interface ToolboxDialogProps {
  open: boolean;
  onClose: () => void;
  selectedIds: number[];
  onApplied: () => void;
}

const TOOLS = [
  { id: "uppercase_all", label: "Uppercase All" },
  { id: "lowercase_all", label: "Lowercase All" },
  { id: "uppercase_first", label: "Uppercase First Word" },
  { id: "title_case", label: "Title Case" },
  { id: "fix_alias", label: "Fix Alias" },
  { id: "add_header", label: "Add Header" },
  { id: "trim", label: "Trim" },
];

const TARGETS = [
  { value: "translation", label: "Translation" },
  { value: "source", label: "Source" },
  { value: "both", label: "Both" },
];

export function ToolboxDialog({ open, onClose, selectedIds, onApplied }: ToolboxDialogProps) {
  const [tool, setTool] = useState("uppercase_all");
  const [target, setTarget] = useState("translation");
  const [headerText, setHeaderText] = useState("");
  const [running, setRunning] = useState(false);

  if (!open) return null;

  const handleApply = async () => {
    setRunning(true);
    try {
      const count = await toolboxTransform(tool, target, selectedIds, tool === "add_header" ? headerText : undefined);
      toast.success(`${count} strings modified`);
      onApplied();
    } catch (e: any) {
      toast.error(`Toolbox error: ${e}`);
    } finally {
      setRunning(false);
    }
  };

  return (
    <Modal
      open={open}
      onClose={onClose}
      title="Toolbox"
      size="md"
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="primary" onClick={handleApply} loading={running}>
            {running ? "Applying..." : "Apply"}
          </Button>
        </>
      }
    >
      <div className="dialog-section">
        <label className="dialog-label">Tool</label>
        <div style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
          {TOOLS.map((t) => (
            <label key={t.id} className="settings-checkbox-label" style={{ cursor: "pointer" }}>
              <input
                type="radio"
                name="toolbox-tool"
                value={t.id}
                checked={tool === t.id}
                onChange={(e) => setTool(e.target.value)}
              />
              {t.label}
            </label>
          ))}
        </div>
      </div>

      <div className="dialog-section">
        <label className="dialog-label">Target</label>
        <Select
          value={target}
          onChange={(e) => setTarget(e.target.value)}
          options={TARGETS}
        />
        <p className="ui-modal-hint">
          Which text to transform: source ({selectedIds.length > 0 ? "selected" : "all"} strings), translation, or both.
        </p>
      </div>

      {tool === "add_header" && (
        <div className="dialog-section">
          <label className="dialog-label">Header Text</label>
          <input
            className="ui-input"
            type="text"
            value={headerText}
            onChange={(e) => setHeaderText(e.target.value)}
            placeholder="Prefix to add..."
          />
        </div>
      )}
    </Modal>
  );
}
