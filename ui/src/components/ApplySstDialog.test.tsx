import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ApplySstDialog } from "./ApplySstDialog";

// 模拟 react-i18next
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (_key: string, fallback?: string) => fallback ?? _key,
  }),
}));

describe("ApplySstDialog", () => {
  it("renders when open and displays all overwrite scopes and match modes", () => {
    const onConfirm = vi.fn();
    const onClose = vi.fn();

    render(
      <ApplySstDialog
        open={true}
        sstPath="C:/path/to/Skyrim.sst"
        selectedCount={5}
        filteredCount={100}
        onConfirm={onConfirm}
        onClose={onClose}
      />
    );

    expect(screen.getByText(/Apply SST Options/i)).toBeDefined();
    expect(screen.getByText(/All \(Eligible strings, including locked VMAD\)/i)).toBeDefined();
    expect(screen.getByText(/NoTrans \(Untranslated exclusive\)/i)).toBeDefined();
    expect(screen.getByText(/Selected Strings Only/i)).toBeDefined();
    expect(screen.getByText(/FORMID \+ Strict String Control/i)).toBeDefined();
  });

  it("submits configured options upon confirm button click", () => {
    const onConfirm = vi.fn();
    const onClose = vi.fn();

    render(
      <ApplySstDialog
        open={true}
        sstPath="C:/path/to/Skyrim.sst"
        selectedCount={3}
        filteredCount={50}
        onConfirm={onConfirm}
        onClose={onClose}
      />
    );

    // 选中 Partial only
    const partialRadio = screen.getByDisplayValue("partial_only");
    fireEvent.click(partialRadio);

    // 选中 String only 模式
    const stringOnlyRadio = screen.getByDisplayValue("string_only");
    fireEvent.click(stringOnlyRadio);

    // 勾选 Tag only
    const tagOnlyCheckbox = screen.getByLabelText(/Apply Tag Only/i);
    fireEvent.click(tagOnlyCheckbox);

    // 点击确定按钮
    const confirmBtn = screen.getByRole("button", { name: /Apply/i });
    fireEvent.click(confirmBtn);

    expect(onConfirm).toHaveBeenCalledTimes(1);
    expect(onConfirm).toHaveBeenCalledWith({
      overwrite_scope: "partial_only",
      match_mode: "string_only",
      tag_only: true,
      reset_state: false,
      restrict_to_filter: false,
    });
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
