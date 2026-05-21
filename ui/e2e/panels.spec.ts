import { test, expect } from "./fixtures/base";

// Helper: open a sidebar panel by clicking its toolbar button
async function openPanel(page: any, label: string) {
  const btn = page.locator(
    `[aria-label="${label}"], [title="${label}"], button:has-text("${label}")`
  ).first();
  if (await btn.count() > 0) {
    await btn.click();
    await page.waitForTimeout(400);
  }
}

// ===================================================================
// McmPanel — MCM Translation Editor
// ===================================================================
test.describe("MCM Panel", { tag: "@mcm" }, () => {
  test("loads MCM file and shows entries with status badges", async ({ appPage }) => {
    await appPage.goto();
    await openPanel(appPage.page, "Open MCM Panel");

    // Seed mock data
    await appPage.page.evaluate(() => {
      (window as any).__setMockResult("load_mcm_file", {
        path: "C:/mock/Settings.txt",
        entry_count: 3,
        encoding: "UTF-8",
        entries: [
          { id: "$sGeneral", source: "General", translation: "常规", line_index: 0, byte_offset: 0 },
          { id: "$sAudio", source: "Audio", translation: "", line_index: 1, byte_offset: 20 },
          { id: "$sVideo", source: "Video", translation: "视频", line_index: 2, byte_offset: 40 },
        ],
      });
    });

    // Click the open button in the panel (dialog mock returns a path)
    const openBtn = appPage.page.locator("button:has-text('Open')").first();
    if (await openBtn.count() > 0 && await openBtn.isVisible()) {
      await openBtn.click();
      await appPage.page.waitForTimeout(500);
    }

    // Verify MCM entries exist
    const entries = appPage.page.locator(".mcm-entry");
    expect(await entries.count()).toBeGreaterThanOrEqual(1);

    // Verify entry IDs are shown
    const entryIds = appPage.page.locator(".mcm-entry-id");
    await expect(entryIds.first()).toBeAttached({ timeout: 3_000 });
  });
});

// ===================================================================
// EspComparePanel — ESP Comparison
// ===================================================================
test.describe("ESP Compare Panel", { tag: "@esp-compare" }, () => {
  test("runs comparison and shows sort buttons", async ({ appPage }) => {
    await appPage.goto();
    await openPanel(appPage.page, "Open ESP Compare");

    // Seed mock data
    await appPage.page.evaluate(() => {
      (window as any).__setMockResult("compare_esp_files", {
        identical_count: 2,
        added_count: 1,
        removed_count: 1,
        modified_count: 1,
        identical: [
          { new_id: 0x100, old_id: 0x100, source: "Hello", record_sig: "INFO", field_sig: "FULL", old_source: "Hello", new_source: "Hello" },
          { new_id: 0x101, old_id: 0x101, source: "World", record_sig: "INFO", field_sig: "FULL", old_source: "World", new_source: "World" },
        ],
        added: [{ new_id: 0x200, old_id: 0, source: "New", record_sig: "QUST", field_sig: "FULL", old_source: "", new_source: "New" }],
        removed: [{ new_id: 0, old_id: 0x300, source: "Old", record_sig: "INFO", field_sig: "FULL", old_source: "Old", new_source: "" }],
        modified: [{ new_id: 0x400, old_id: 0x400, source: "Changed", record_sig: "INFO", field_sig: "FULL", old_source: "Original", new_source: "Changed" }],
      });
    });

    // Click compare button (dialog mock returns a path)
    const compareBtn = appPage.page.locator("button:has-text('Compare')").first();
    if (await compareBtn.count() > 0 && await compareBtn.isVisible()) {
      await compareBtn.click();
      await appPage.page.waitForTimeout(500);
    }

    // Verify sorting buttons appear
    const sortBtns = appPage.page.locator(".esp-compare-sort-btn");
    await expect(sortBtns.first()).toBeAttached({ timeout: 3_000 });
  });
});

// ===================================================================
// FuzPanel — Voice File Browser
// ===================================================================
test.describe("FUZ Panel", { tag: "@fuz" }, () => {
  test("scans directory and shows stats", async ({ appPage }) => {
    await appPage.goto();
    await openPanel(appPage.page, "Open Voice Panel");

    // Seed mock data
    await appPage.page.evaluate(() => {
      (window as any).__setMockResult("scan_fuz_directory", {
        fuz_mappings: [
          { response_id: 0x100, dialog_text: "Hello guard", fuz_file: "D:/Voice/guard.fuz", duration_secs: 2.5, has_lip: true, parse_ok: true },
          { response_id: 0x101, dialog_text: "Adventurer", fuz_file: "D:/Voice/adv.fuz", duration_secs: 3.0, has_lip: false, parse_ok: true },
        ],
        total_fuz_files: 10,
      });
    });

    const scanBtn = appPage.page.locator("button:has-text('Scan')").first();
    if (await scanBtn.count() > 0 && await scanBtn.isVisible()) {
      await scanBtn.click();
      await appPage.page.waitForTimeout(500);
    }

    // Verify sort bar is visible
    const sortBar = appPage.page.locator(".fuz-sort-bar");
    await expect(sortBar.first()).toBeAttached({ timeout: 3_000 });
  });
});

// ===================================================================
// BSA Browser
// ===================================================================
test.describe("BSA Browser", { tag: "@bsa" }, () => {
  test("can be opened via toolbar", async ({ appPage }) => {
    await appPage.goto();
    await expect(appPage.stringTable).toBeAttached({ timeout: 10_000 });

    await openPanel(appPage.page, "Open BSA Browser");
    await appPage.page.waitForTimeout(300);

    // BSA panel should render some content
    const sidepanel = appPage.page.locator('[class*="sidepanel"]').first();
    await expect(sidepanel).toBeAttached({ timeout: 5_000 });
  });
});

// ===================================================================
// PEX Panel
// ===================================================================
test.describe("PEX Panel", { tag: "@pex" }, () => {
  test("can be opened via toolbar", async ({ appPage }) => {
    await appPage.goto();
    await expect(appPage.stringTable).toBeAttached({ timeout: 10_000 });

    await openPanel(appPage.page, "Open PEX Panel");
    await appPage.page.waitForTimeout(300);

    const sidepanel = appPage.page.locator('[class*="sidepanel"]').first();
    await expect(sidepanel).toBeAttached({ timeout: 5_000 });
  });
});
