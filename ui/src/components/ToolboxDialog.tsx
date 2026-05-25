import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { toolboxTransform, toolboxLoadExceptionWords, toolboxGetExceptionWords, saveConfig, loadConfig } from "../api/strings";
import toast from "react-hot-toast";
import { Button, Modal, Select } from "./ui";

interface ToolboxDialogProps {
  open: boolean;
  onClose: () => void;
  selectedIds: number[];
  onApplied: () => void;
}

const TOOLS = [
  { id: "uppercase_all", labelKey: "toolbox.uppercaseAll" },
  { id: "lowercase_all", labelKey: "toolbox.lowercaseAll" },
  { id: "uppercase_first", labelKey: "toolbox.uppercaseFirstWord" },
  { id: "title_case", labelKey: "toolbox.titleCase" },
  { id: "fix_alias", labelKey: "toolbox.fixAlias" },
  { id: "add_header", labelKey: "toolbox.addHeader" },
  { id: "trim", labelKey: "toolbox.trim" },
];

const TARGETS = [
  { value: "translation", labelKey: "toolbox.targetTranslation" },
  { value: "source", labelKey: "toolbox.targetSource" },
  { value: "both", labelKey: "toolbox.targetBoth" },
];

export function ToolboxDialog({ open, onClose, selectedIds, onApplied }: ToolboxDialogProps) {
  const { t } = useTranslation();
  const [tool, setTool] = useState("uppercase_all");
  const [target, setTarget] = useState("translation");
  const [headerText, setHeaderText] = useState("");
  const [running, setRunning] = useState(false);
  const [showExceptionEditor, setShowExceptionEditor] = useState(false);
  const [exceptionWords, setExceptionWords] = useState<string[]>([]);
  const [exceptionText, setExceptionText] = useState("");

  // 当对话框打开时加载例外词汇
  useEffect(() => {
    if (open) {
      toolboxGetExceptionWords().then(setExceptionWords).catch(() => setExceptionWords([]));
    }
  }, [open]);

  // 同步例外词汇文本与列表
  useEffect(() => {
    setExceptionText(exceptionWords.join("\n"));
  }, [exceptionWords]);

  const handleApply = async () => {
    setRunning(true);
    try {
      const count = await toolboxTransform(tool, target, selectedIds, tool === "add_header" ? headerText : undefined);
      toast.success(t("toolbox.stringsModified", { count }));
      onApplied();
    } catch (e: any) {
      toast.error(t("toolbox.error", { error: String(e) }));
    } finally {
      setRunning(false);
    }
  };

  const handleSaveExceptions = async () => {
    try {
      const words = exceptionText.split("\n").map((w) => w.trim()).filter((w) => w.length > 0);
      await toolboxLoadExceptionWords(words.join("\n"));
      setExceptionWords(words);

      // 保存到配置
      const currentConfig = await loadConfig();
      await saveConfig({ ...currentConfig, word_exception_list: words.join("\n") });
      toast.success(t("toolbox.exceptionWordsSaved", { count: words.length }));
      setShowExceptionEditor(false);
    } catch (e: any) {
      toast.error(String(e));
    }
  };

  if (!open) return null;

  // 例外词汇编辑器模态框
  if (showExceptionEditor) {
    return (
      <Modal
        open={open}
        onClose={() => setShowExceptionEditor(false)}
        title={t("toolbox.exceptionWordsTitle")}
        size="lg"
        footer={
          <>
            <Button variant="ghost" onClick={() => setShowExceptionEditor(false)}>
              {t("common.cancel")}
            </Button>
            <Button variant="primary" onClick={handleSaveExceptions}>
              {t("common.save")}
            </Button>
          </>
        }
      >
        <div className="dialog-section">
          <p className="ui-modal-hint">{t("toolbox.exceptionWordsHint")}</p>
          <textarea
            className="ui-textarea"
            rows={12}
            value={exceptionText}
            onChange={(e) => setExceptionText(e.target.value)}
            placeholder={"is\na\nthe\nan\nof\nand\nor\nbut\n"}
          />
        </div>
      </Modal>
    );
  }

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={t("menu.toolbox")}
      size="md"
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            {t("common.cancel")}
          </Button>
          <Button variant="primary" onClick={handleApply} loading={running}>
            {running ? t("toolbox.applying") : t("toolbox.apply")}
          </Button>
        </>
      }
    >
      <div className="dialog-section">
        <label className="dialog-label">{t("toolbox.tool")}</label>
        <div style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
          {TOOLS.map((item) => (
            <label key={item.id} className="settings-checkbox-label" style={{ cursor: "pointer" }}>
              <input
                type="radio"
                name="toolbox-tool"
                value={item.id}
                checked={tool === item.id}
                onChange={(e) => setTool(e.target.value)}
              />
              {t(item.labelKey)}
            </label>
          ))}
        </div>
      </div>

      <div className="dialog-section">
        <label className="dialog-label">{t("toolbox.target")}</label>
        <Select
          value={target}
          onChange={(e) => setTarget(e.target.value)}
          options={TARGETS.map((item) => ({ value: item.value, label: t(item.labelKey) }))}
        />
        <p className="ui-modal-hint">
          {selectedIds.length > 0 ? t("toolbox.hintSelected") : t("toolbox.hintAll")}
        </p>
      </div>

      {tool === "add_header" && (
        <div className="dialog-section">
          <label className="dialog-label">{t("toolbox.headerText")}</label>
          <input
            className="ui-input"
            type="text"
            value={headerText}
            onChange={(e) => setHeaderText(e.target.value)}
            placeholder={t("toolbox.headerPlaceholder")}
          />
        </div>
      )}

      {tool === "title_case" && (
        <div className="dialog-section">
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <div>
              <label className="dialog-label">{t("toolbox.exceptionWords")}</label>
              <p className="ui-modal-hint">
                {t("toolbox.exceptionWordsCount", { count: exceptionWords.length })}
              </p>
            </div>
            <Button variant="ghost" size="sm" onClick={() => setShowExceptionEditor(true)}>
              {t("common.edit")}
            </Button>
          </div>
        </div>
      )}
    </Modal>
  );
}