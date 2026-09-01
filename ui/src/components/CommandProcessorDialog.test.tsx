import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { CommandProcessorDialog } from "./CommandProcessorDialog";

// 模拟 react-i18next：正确处理 { defaultValue } 选项
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: { defaultValue?: string }) =>
      options?.defaultValue ?? key,
  }),
}));

// 模拟 tauri API：Dialog 不使用它们渲染，但导入时会被引用
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn().mockResolvedValue(null),
  save: vi.fn().mockResolvedValue(null),
}));
vi.mock("../api/strings", async (importOriginal) => {
  const original = await importOriginal<typeof import("../api/strings")>();
  return {
    ...original,
    readTextFile: vi.fn(),
    writeTextFile: vi.fn(),
    runCommandProcessor: vi.fn(),
  };
});

// 模拟 zustand store
vi.mock("../stores/appStore", () => ({
  useAppStore: (selector: (s: any) => any) =>
    selector({
      currentGame: "SkyrimSE",
      setEspLoaded: vi.fn(),
      clearEspLoaded: vi.fn(),
      loadAllStrings: vi.fn(),
    }),
}));

describe("CommandProcessorDialog", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("renders the processor editor with a default Delphi-compatible script", () => {
    render(<CommandProcessorDialog />);

    // 标题区
    expect(screen.getByText(/Processor script/i)).toBeDefined();
    expect(screen.getByText(/Execution log/i)).toBeDefined();

    // 默认脚本包含 StartRule/EndRule 和核心命令
    const editor = screen.getByText(/StartRule/i) as HTMLElement;
    expect(editor).toBeDefined();
  });

  it("persists draft to localStorage when the script changes", () => {
    const { container } = render(<CommandProcessorDialog />);
    const textarea = container.querySelector("textarea");
    expect(textarea).not.toBeNull();

    if (textarea) {
      fireEvent.change(textarea, {
        target: { value: "StartRule\nCommand=CloseAll\nEndRule\n" },
      });
    }

    const saved = localStorage.getItem("xtranslator-command-processor-draft");
    expect(saved).toContain("CloseAll");
  });
});
