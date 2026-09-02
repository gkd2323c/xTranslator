import React, { useState, useEffect } from "react";
import { useAppStore } from "../stores/appStore";
import {
  getDefaultDefUiOptions,
  applyDefUiGenerator,
  type DefUiOptionsDto,
  type DefUiGenerateResultDto,
} from "../api/strings";
import { Modal, Button, Section, Input, Textarea } from "./ui";
import { Sparkles, Play, Eye, RotateCcw } from "lucide-react";
import toast from "react-hot-toast";

interface DefUiGenDialogProps {
  isOpen: boolean;
  onClose: () => void;
}

export const DefUiGenDialog: React.FC<DefUiGenDialogProps> = ({
  isOpen,
  onClose,
}) => {
  const currentGame = useAppStore((s) => s.currentGame);
  const selectedIds = useAppStore((s) => s.selectedIds);
  const loadAllStrings = useAppStore((s) => s.loadAllStrings);

  const [options, setOptions] = useState<DefUiOptionsDto | null>(null);
  const [scope, setScope] = useState<"all" | "only_untranslated" | "only_selected">("all");
  const [isLoading, setIsLoading] = useState(false);
  const [isRunning, setIsRunning] = useState(false);
  const [previewResult, setPreviewResult] = useState<DefUiGenerateResultDto | null>(null);
  const [ignoreListText, setIgnoreListText] = useState("");

  // 加载当前游戏的默认配置
  useEffect(() => {
    if (isOpen) {
      setIsLoading(true);
      getDefaultDefUiOptions(currentGame ?? undefined)
        .then((opts) => {
          setOptions(opts);
          setIgnoreListText(opts.ignore_list.join("\n"));
        })
        .catch((err) => {
          console.error("Failed to load DefUI options:", err);
          toast.error("加载 DefUI 默认配置失败");
        })
        .finally(() => setIsLoading(false));
    }
  }, [isOpen, currentGame]);

  if (!isOpen || !options) return null;

  const handleScopeChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setScope(e.target.value as "all" | "only_untranslated" | "only_selected");
  };

  const handleOptionChange = <K extends keyof DefUiOptionsDto>(
    key: K,
    value: DefUiOptionsDto[K]
  ) => {
    setOptions((prev) => (prev ? { ...prev, [key]: value } : null));
  };

  const handleIgnoreListChange = (val: string) => {
    setIgnoreListText(val);
    const list = val
      .split("\n")
      .map((s) => s.trim())
      .filter(Boolean);
    handleOptionChange("ignore_list", list);
  };

  const handlePreview = async () => {
    if (!options) return;
    setIsRunning(true);
    try {
      const res = await applyDefUiGenerator({
        options,
        scope,
        selected_ids: Array.from(selectedIds),
        dry_run: true,
      });
      setPreviewResult(res);
      toast.success(`预览完成：找到 ${res.matched_count} 条匹配记录`);
    } catch (err: any) {
      toast.error(`预览失败: ${err.message || String(err)}`);
    } finally {
      setIsRunning(false);
    }
  };

  const handleApply = async () => {
    if (!options) return;
    setIsRunning(true);
    try {
      const res = await applyDefUiGenerator({
        options,
        scope,
        selected_ids: Array.from(selectedIds),
        dry_run: false,
      });
      toast.success(`生成成功：更新了 ${res.modified_count} 条译文`);
      await loadAllStrings();
      onClose();
    } catch (err: any) {
      toast.error(`生成失败: ${err.message || String(err)}`);
    } finally {
      setIsRunning(false);
    }
  };

  const handleReset = async () => {
    setIsLoading(true);
    try {
      const opts = await getDefaultDefUiOptions(currentGame ?? undefined);
      setOptions(opts);
      setIgnoreListText(opts.ignore_list.join("\n"));
      setPreviewResult(null);
      toast.success("已重置为默认配置");
    } catch (err) {
      toast.error("重置失败");
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <Modal
      open={isOpen}
      onClose={onClose}
      title="DEF_UI / Component Generator (物品组件标签生成器)"
      size="xl"
    >
      <div className="space-y-4 p-4 text-sm max-h-[80vh] overflow-y-auto">
        {/* 说明 */}
        <div className="bg-[var(--bg-muted)] p-3 rounded text-[var(--text-secondary)] text-xs flex items-start gap-2">
          <Sparkles className="w-4 h-4 text-sky-400 mt-0.5 shrink-0" />
          <span>
            为 Fallout 4 / 76 / Starfield 的垃圾杂项 (MISC) 自动附加拆解材料标签。根据 ESP 的 CVPA 组件定义与正则表达式，生成符合 DEF_UI / FallUI 规范的标签化名称。
          </span>
        </div>

        {/* 作用范围 */}
        <Section title="处理范围 (Scope)">
          <div className="flex gap-4 items-center">
            <label className="flex items-center gap-1.5 cursor-pointer">
              <input
                type="radio"
                name="scope"
                value="all"
                checked={scope === "all"}
                onChange={handleScopeChange}
              />
              全部 MISC 记录 (All)
            </label>
            <label className="flex items-center gap-1.5 cursor-pointer">
              <input
                type="radio"
                name="scope"
                value="only_untranslated"
                checked={scope === "only_untranslated"}
                onChange={handleScopeChange}
              />
              仅未翻译 (Only Untranslated)
            </label>
            <label className="flex items-center gap-1.5 cursor-pointer">
              <input
                type="radio"
                name="scope"
                value="only_selected"
                checked={scope === "only_selected"}
                onChange={handleScopeChange}
                disabled={selectedIds.size === 0}
              />
              仅选中行 (Selected: {selectedIds.size})
            </label>
          </div>
        </Section>

        {/* 模板与格式设置 */}
        <Section title="格式与模板 (Formats & Templates)">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
            <div>
              <label className="text-xs font-semibold block mb-1">
                完整模板 (Full Format, 支持 %BASE%, %COMPOS%, %WEIGHT%)
              </label>
              <Input
                value={options.format_full}
                onChange={(e) => handleOptionChange("format_full", e.target.value)}
                placeholder="%BASE% {{{%COMPOS%}}}"
              />
            </div>
            <div>
              <label className="text-xs font-semibold block mb-1">
                带重量模板 (Weight Format)
              </label>
              <Input
                value={options.format_weight}
                onChange={(e) => handleOptionChange("format_weight", e.target.value)}
                placeholder="%BASE% {{{%WEIGHT%lb, %COMPOS%}}}"
              />
            </div>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-3 mt-3">
            <div>
              <label className="text-xs font-semibold block mb-1">
                基础名清洗正则 (Clean Base RegEx)
              </label>
              <Input
                value={options.clean_base_regex}
                onChange={(e) => handleOptionChange("clean_base_regex", e.target.value)}
                placeholder="^(.+)\{\{\{.*\}\}\}$"
              />
            </div>
            <div>
              <label className="text-xs font-semibold block mb-1">
                组件名清洗正则 (Clean Component RegEx)
              </label>
              <Input
                value={options.clean_compo_regex}
                onChange={(e) => handleOptionChange("clean_compo_regex", e.target.value)}
                placeholder="^\[.+\](.+)$"
              />
            </div>
          </div>
        </Section>

        {/* 规则选项开关 */}
        <Section title="生成选项 (Options)">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-2 text-xs">
            <label className="flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                checked={options.use_source_for_string}
                onChange={(e) => handleOptionChange("use_source_for_string", e.target.checked)}
              />
              使用英文原文作为基础名 (Use Source for Base Name)
            </label>
            <label className="flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                checked={options.use_source_for_components}
                onChange={(e) => handleOptionChange("use_source_for_components", e.target.checked)}
              />
              使用英文原文作为组件名 (Use Source for Components)
            </label>
            <label className="flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                checked={options.clean_base}
                onChange={(e) => handleOptionChange("clean_base", e.target.checked)}
              />
              清洗基础名中已有的标签/后缀 (Clean Base Tag)
            </label>
            <label className="flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                checked={options.clean_components}
                onChange={(e) => handleOptionChange("clean_components", e.target.checked)}
              />
              清洗组件名中已有的标签 (Clean Component Tag)
            </label>
            <label className="flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                checked={options.add_weight}
                onChange={(e) => handleOptionChange("add_weight", e.target.checked)}
              />
              附加重量信息 (Add Weight)
            </label>
            <label className="flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                checked={options.use_custom_indicators}
                onChange={(e) => handleOptionChange("use_custom_indicators", e.target.checked)}
              />
              启用数量等级指示符 (*, **, ***, ****)
            </label>
          </div>

          {options.use_custom_indicators && (
            <div className="mt-2">
              <label className="text-xs font-semibold block mb-1">
                自定义数量等级指示符 (用分号 ; 分隔 1, 2, 3, 4 级)
              </label>
              <Input
                value={options.custom_indicators}
                onChange={(e) => handleOptionChange("custom_indicators", e.target.value)}
                placeholder=";*;**;***;****"
              />
            </div>
          )}
        </Section>

        {/* 忽略关键词列表 */}
        <Section title="忽略列表 (Ignore List, 每行一个关键词/FormID)">
          <Textarea
            rows={3}
            value={ignoreListText}
            onChange={(e) => handleIgnoreListChange(e.target.value)}
            placeholder="Shipment&#10;Bottlecap"
            className="text-xs font-mono"
          />
        </Section>

        {/* 预览窗口 */}
        {previewResult && (
          <Section title={`预览结果 (Preview: 匹配 ${previewResult.matched_count} 条)`}>
            <div className="bg-[var(--bg-primary)] border border-[var(--border-color)] rounded p-2 max-h-40 overflow-y-auto font-mono text-xs space-y-1">
              {previewResult.sample_previews.length === 0 ? (
                <div className="text-[var(--text-secondary)] italic p-2">未找到符合条件的 MISC 条目</div>
              ) : (
                previewResult.sample_previews.map(([orig, target], i) => (
                  <div key={i} className="flex items-center justify-between border-b border-[var(--border-color)] pb-1 last:border-none">
                    <span className="text-[var(--text-secondary)] truncate w-5/12">{orig}</span>
                    <span className="text-[var(--text-accent)] w-1/12 text-center">➔</span>
                    <span className="text-green-400 font-semibold truncate w-6/12">{target}</span>
                  </div>
                ))
              )}
            </div>
          </Section>
        )}

        {/* 操作按钮 */}
        <div className="flex justify-between items-center pt-2 border-t border-[var(--border-color)]">
          <Button
            variant="default"
            onClick={handleReset}
            disabled={isLoading || isRunning}
            className="flex items-center gap-1.5"
          >
            <RotateCcw className="w-3.5 h-3.5" />
            重置默认值
          </Button>

          <div className="flex gap-2">
            <Button
              variant="default"
              onClick={handlePreview}
              disabled={isLoading || isRunning}
              className="flex items-center gap-1.5"
            >
              <Eye className="w-3.5 h-3.5" />
              预览 (Preview)
            </Button>
            <Button
              variant="primary"
              onClick={handleApply}
              disabled={isLoading || isRunning}
              className="flex items-center gap-1.5 bg-sky-600 hover:bg-sky-500"
            >
              <Play className="w-3.5 h-3.5" />
              应用生成 (Apply)
            </Button>
          </div>
        </div>
      </div>
    </Modal>
  );
};
