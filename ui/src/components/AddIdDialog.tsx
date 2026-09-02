import React, { useState } from 'react';
import { Modal } from './ui/Modal';
import { Button } from './ui/Button';
import { useAppStore } from '../stores/appStore';
import { applyAddIdOffset } from '../api/strings';
import toast from 'react-hot-toast';

export const AddIdDialog: React.FC = () => {
  const activePanel = useAppStore((s) => s.activePanel);
  const setActivePanel = useAppStore((s) => s.setActivePanel);
  const selectedIds = useAppStore((s) => s.selectedIds);
  const loadAllStrings = useAppStore((s) => s.loadAllStrings);
  const addLog = useAppStore((s) => s.addLog);

  const isOpen = activePanel === 'addId';
  const onClose = () => setActivePanel(null);

  const [offsetStr, setOffsetStr] = useState<string>('0');
  const [isHex, setIsHex] = useState<boolean>(true);
  const [scope, setScope] = useState<'all' | 'only_untranslated' | 'only_selected'>('all');
  const [loading, setLoading] = useState<boolean>(false);

  if (!isOpen) return null;

  const handleApply = async () => {
    let offsetVal = 0;
    try {
      const clean = offsetStr.trim().replace(/^0x/i, '');
      if (isHex) {
        offsetVal = parseInt(clean, 16);
      } else {
        offsetVal = parseInt(clean, 10);
      }
      if (isNaN(offsetVal)) {
        toast.error('无效的偏移数值');
        return;
      }
    } catch {
      toast.error('解析偏移值失败');
      return;
    }

    setLoading(true);
    try {
      const res = await applyAddIdOffset({
        offsetValue: offsetVal,
        applyToFormId: true,
        scope,
        selectedIds: Array.from(selectedIds),
      });

      toast.success('FormID 偏移应用完成: 修改 ' + res.modifiedCount + ' 项 / 共处理 ' + res.totalProcessed + ' 项');
      addLog('info', 'FormID 批量偏移完成: 偏移值 ' + offsetStr + ', 修改 ' + res.modifiedCount + ' 项', 'AddId');
      await loadAllStrings();
      onClose();
    } catch (e) {
      toast.error('应用 FormID 偏移失败: ' + String(e));
      addLog('error', 'FormID 偏移失败: ' + String(e), 'AddId');
    } finally {
      setLoading(false);
    }
  };

  return (
    <Modal
      open={isOpen}
      onClose={onClose}
      title='FormID 批量偏移工具 (AddId / Re-indexing)'
      size='md'
    >
      <div className="space-y-4 text-sm text-theme-text">
        <p className="text-xs text-theme-text-muted">
          用于将插件中的 FormID 统一增加或减去一个固定偏移量（例如将独立 Mod 的 FormID 空间向后推移以避免冲突）。
        </p>

        <div className="rounded border border-theme-border bg-theme-bg-secondary p-3 space-y-3">
          <div className="flex items-center gap-3">
            <label className="font-medium min-w-20">偏移数值:</label>
            <input
              type="text"
              className="flex-1 rounded border border-theme-border bg-theme-bg px-2 py-1 font-mono text-sm"
              value={offsetStr}
              onChange={(e) => setOffsetStr(e.target.value)}
              placeholder="例如: 0x1000 或 4096"
            />
            <label className="flex items-center gap-1 text-xs cursor-pointer select-none">
              <input
                type="checkbox"
                checked={isHex}
                onChange={(e) => setIsHex(e.target.checked)}
              />
              十六进制 (Hex)
            </label>
          </div>

          <div className="border-t border-theme-border pt-2">
            <label className="font-medium block mb-1 text-xs">应用范围 (Scope):</label>
            <div className="space-y-1 pl-1 text-xs">
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="radio"
                  name="addIdScope"
                  checked={scope === 'all'}
                  onChange={() => setScope('all')}
                />
                所有记录 (All Records)
              </label>
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="radio"
                  name="addIdScope"
                  checked={scope === 'only_untranslated'}
                  onChange={() => setScope('only_untranslated')}
                />
                仅未翻译条目 (Only Untranslated)
              </label>
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="radio"
                  name="addIdScope"
                  checked={scope === 'only_selected'}
                  onChange={() => setScope('only_selected')}
                />
                仅选中的行 (Only Selected Rows) ({selectedIds.size} 项)
              </label>
            </div>
          </div>
        </div>

        <div className="flex justify-end gap-2 pt-2 border-t border-theme-border">
          <Button variant="ghost" onClick={onClose} disabled={loading}>
            取消
          </Button>
          <Button variant="primary" onClick={handleApply} disabled={loading}>
            {loading ? '正在处理...' : '确认应用偏移'}
          </Button>
        </div>
      </div>
    </Modal>
  );
};
