import React, { useState } from 'react';
import { Modal } from './ui/Modal';
import { Button } from './ui/Button';
import { useAppStore } from '../stores/appStore';
import { applyAddIdToStrings } from '../api/strings';
import toast from 'react-hot-toast';

export const AddIdToStringsDialog: React.FC = () => {
  const activePanel = useAppStore((s) => s.activePanel);
  const setActivePanel = useAppStore((s) => s.setActivePanel);
  const selectedIds = useAppStore((s) => s.selectedIds);
  const loadAllStrings = useAppStore((s) => s.loadAllStrings);
  const addLog = useAppStore((s) => s.addLog);
  const currentGame = useAppStore((s) => s.currentGame);

  const isOpen = activePanel === 'addIdToStrings';
  const onClose = () => setActivePanel(null);

  const [scope, setScope] = useState<'everything' | 'no_trans_valid' | 'selection'>('no_trans_valid');
  const [addStringId, setAddStringId] = useState(false);
  const [addFormId, setAddFormId] = useState(false);
  const [addRecordRef, setAddRecordRef] = useState(false);
  const [addDialRef, setAddDialRef] = useState(false);
  const [loading, setLoading] = useState(false);

  // Delphi DFM：StringID 仅在 localized 模式启用；RecordRef/DialRef 仅在 ESP 模式启用。
  // Rust 简化：StringID 始终可用；RecordRef/DialRef 在有 ESP 上下文时可用（currentGame 非空即表示已加载 ESP）。
  const hasEspContext = currentGame !== null && currentGame !== undefined;

  if (!isOpen) return null;

  const handleApply = async () => {
    const hasAny = addStringId || addFormId || addRecordRef || addDialRef;
    if (!hasAny) {
      toast.error('请至少选择一个要添加的标识');
      return;
    }

    setLoading(true);
    try {
      const res = await applyAddIdToStrings({
        scope,
        selectedIds: Array.from(selectedIds),
        addStringId,
        addFormId,
        addRecordRef,
        addDialRef,
      });

      toast.success(
        `AddIdToStrings 完成: 修改 ${res.modifiedCount} 项 / 共处理 ${res.totalProcessed} 项`,
      );
      addLog(
        'info',
        `AddIdToStrings 完成: 修改 ${res.modifiedCount} 项`,
        'AddIdToStrings',
      );
      await loadAllStrings();
      onClose();
    } catch (e) {
      toast.error('AddIdToStrings 失败: ' + String(e));
      addLog('error', 'AddIdToStrings 失败: ' + String(e), 'AddIdToStrings');
    } finally {
      setLoading(false);
    }
  };

  return (
    <Modal open={isOpen} onClose={onClose} title="AddIdToStrings" size="md">
      <div className="space-y-4 text-sm text-theme-text">
        <p className="text-xs text-theme-text-muted">
          为译文添加标识前缀（Delphi 原版功能）：String ID、FormID、Record/Field 引用、DIAL master 引用。
        </p>

        <div className="rounded border border-theme-border bg-theme-bg-secondary p-3 space-y-3">
          <div>
            <label className="font-medium block mb-1 text-xs">应用范围 (Scope):</label>
            <div className="space-y-1 pl-1 text-xs">
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="radio"
                  name="addIdScope"
                  checked={scope === 'everything'}
                  onChange={() => setScope('everything')}
                />
                全部 (Everything)
              </label>
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="radio"
                  name="addIdScope"
                  checked={scope === 'no_trans_valid'}
                  onChange={() => setScope('no_trans_valid')}
                />
                仅未翻译且未验证 (NoTransValid)
              </label>
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="radio"
                  name="addIdScope"
                  checked={scope === 'selection'}
                  onChange={() => setScope('selection')}
                />
                仅选中项 (Selection) ({selectedIds.size} 项)
              </label>
            </div>
          </div>

          <div className="border-t border-theme-border pt-2">
            <label className="font-medium block mb-1 text-xs">添加标识:</label>
            <div className="space-y-1 pl-1 text-xs">
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={addStringId}
                  onChange={(e) => setAddStringId(e.target.checked)}
                />
                String ID [%.5x]（如 [0002a]）
              </label>
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={addFormId}
                  onChange={(e) => setAddFormId(e.target.checked)}
                />
                FormID [%.8x]（如 [0001a4b2]）
              </label>
              <label className={`flex items-center gap-2 cursor-pointer ${!hasEspContext ? 'opacity-50' : ''}`}>
                <input
                  type="checkbox"
                  checked={addRecordRef}
                  disabled={!hasEspContext}
                  onChange={(e) => setAddRecordRef(e.target.checked)}
                />
                Record/Field [REC:FIELD]（需 ESP 上下文）
              </label>
              <label className={`flex items-center gap-2 cursor-pointer ${!hasEspContext ? 'opacity-50' : ''}`}>
                <input
                  type="checkbox"
                  checked={addDialRef}
                  disabled={!hasEspContext}
                  onChange={(e) => setAddDialRef(e.target.checked)}
                />
                DIAL Master [@%.8x]（仅 INFO 记录，需 ESP 上下文）
              </label>
            </div>
          </div>
        </div>

        <div className="flex justify-end gap-2 pt-2 border-t border-theme-border">
          <Button variant="ghost" onClick={onClose} disabled={loading}>
            取消
          </Button>
          <Button variant="primary" onClick={handleApply} disabled={loading}>
            {loading ? '正在处理...' : '确认添加'}
          </Button>
        </div>
      </div>
    </Modal>
  );
};
