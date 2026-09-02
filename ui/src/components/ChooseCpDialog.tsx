import React, { useState, useEffect } from "react";
import { Modal } from "./ui/Modal";
import { Button } from "./ui/Button";
import { useAppStore } from "../stores/appStore";
import { getCodepageInfo, reloadWithCodepage } from "../api/strings";
import toast from "react-hot-toast";
import { Globe, RefreshCw, AlertCircle } from "lucide-react";

const CODEPAGE_DESCRIPTIONS: Record<string, string> = {
  utf8: "UTF-8 (Unicode - 现代标准 / Fallout 4 / Skyrim SE)",
  utf16: "UTF-16 (Unicode)",
  "1250": "Windows-1250 (中欧/东欧 - 捷克、波兰、匈牙利等)",
  "1251": "Windows-1251 (西里尔文 - 俄语、保加利亚语等)",
  "1252": "Windows-1252 (西欧/拉丁 - 英语、法语、德语、西班牙语等)",
  "1253": "Windows-1253 (希腊语)",
  "1254": "Windows-1254 (土耳其语)",
  "1256": "Windows-1256 (阿拉伯语)",
  "932": "CP932 / Shift-JIS (日语)",
  "936": "CP936 / GBK (简体中文 - 上古卷轴5旧版/老滚常用)",
  "950": "CP950 / Big5 (繁体中文)",
};

export const ChooseCpDialog: React.FC = () => {
  const activePanel = useAppStore((s) => s.activePanel);
  const setActivePanel = useAppStore((s) => s.setActivePanel);
  const loadAllStrings = useAppStore((s) => s.loadAllStrings);
  const addLog = useAppStore((s) => s.addLog);

  const isOpen = activePanel === "chooseCp";
  const [currentCp, setCurrentCp] = useState<string>("utf8");
  const [supportedCps, setSupportedCps] = useState<string[]>([]);
  const [selectedCp, setSelectedCp] = useState<string>("utf8");
  const [loading, setLoading] = useState<boolean>(false);

  useEffect(() => {
    if (isOpen) {
      getCepageData();
    }
  }, [isOpen]);

  const getCepageData = async () => {
    try {
      const info = await getCodepageInfo();
      setCurrentCp(info.currentCodepage);
      setSelectedCp(info.currentCodepage);
      setSupportedCps(info.supportedCodepages);
    } catch (e) {
      console.error("Failed to load codepage info", e);
    }
  };

  const handleReload = async () => {
    setLoading(true);
    try {
      toast.loading(`正在以代码页 [${selectedCp}] 重新解析并加载文件...`, { id: "reload-cp" });
      const res = await reloadWithCodepage(selectedCp);
      toast.success(`重新加载完成！共载入 ${res.total} 条字符串`, { id: "reload-cp" });
      addLog("info", `以代码页 [${selectedCp}] 重新加载插件，得到 ${res.total} 条字符串`);
      await loadAllStrings();
      setActivePanel(null);
    } catch (e: any) {
      toast.error(`重新加载失败: ${e}`, { id: "reload-cp" });
      addLog("error", `以代码页 [${selectedCp}] 重新加载插件失败: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  if (!isOpen) return null;

  return (
    <Modal
      open={isOpen}
      onClose={() => setActivePanel(null)}
      title="选择代码页 / 强制编码重载 (Codepage Manager)"
      size="md"
    >
      <div className="space-y-4 text-sm">
        <div className="flex items-start gap-3 p-3 bg-amber-500/10 border border-amber-500/20 rounded-lg text-amber-200/90 text-xs leading-relaxed">
          <AlertCircle className="w-5 h-5 text-amber-400 shrink-0 mt-0.5" />
          <div>
            <div className="font-semibold text-amber-300 mb-1">
              代码页覆盖说明 (Form14/Form15 对齐)
            </div>
            当加载旧版非 UTF-8 编码的插件（如原版天际 CP936 简中、CP950 繁中或西欧 CP1252）发生乱码时，可通过此工具强制指定文本解码页并重新载入。
          </div>
        </div>

        <div className="space-y-1.5">
          <label className="text-xs font-semibold text-zinc-400 flex items-center gap-1.5">
            <Globe className="w-3.5 h-3.5" />
            选择目标文本编码 (Codepage)
          </label>
          <select
            value={selectedCp}
            onChange={(e) => setSelectedCp(e.target.value)}
            className="w-full bg-zinc-900 border border-zinc-700 rounded px-3 py-2 text-sm text-zinc-200 focus:outline-none focus:border-blue-500"
          >
            {supportedCps.map((cp) => (
              <option key={cp} value={cp}>
                {cp} - {CODEPAGE_DESCRIPTIONS[cp] || cp}
              </option>
            ))}
          </select>
          <div className="text-[11px] text-zinc-500 mt-1">
            当前生效代码页: <span className="font-mono text-zinc-300">{currentCp}</span>
          </div>
        </div>

        <div className="flex justify-end gap-2 pt-4 border-t border-zinc-800">
          <Button
            variant="ghost"
            onClick={() => setActivePanel(null)}
            disabled={loading}
          >
            取消
          </Button>
          <Button
            variant="primary"
            onClick={handleReload}
            disabled={loading}
            className="flex items-center gap-1.5"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${loading ? "animate-spin" : ""}`} />
            强制重新加载
          </Button>
        </div>
      </div>
    </Modal>
  );
};
