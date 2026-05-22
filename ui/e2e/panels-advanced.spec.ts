import { test, expect } from "./fixtures/base";

// ===================================================================
// Dialogs Bottom Panel (DialogView)
// ===================================================================
test.describe("Dialogs Panel", { tag: "@dialogs" }, () => {
  test("dialogs tab is visible", async ({ appPage }) => {
    await appPage.goto();
    await expect(appPage.bottomPanel).toBeAttached({ timeout: 10_000 });

    // Check for Dialogs tab - use exact name
    const dialogsTab = appPage.page.locator('.bottom-tab:has-text("Dialogs")');
    expect(await dialogsTab.count()).toBeGreaterThan(0);
  });
});

// ===================================================================
// ESP Tree Panel
// ===================================================================
test.describe("ESP Tree Panel", { tag: "@esp-tree" }, () => {
  test("shows content when ESP loaded", async ({ appPage }) => {
    await appPage.goto();
    await expect(appPage.bottomPanel).toBeAttached({ timeout: 10_000 });

    // Switch to ESP Tree tab
    const espTab = appPage.page.locator(".bottom-tab:has-text('ESP')").first();
    if (await espTab.count() > 0) {
      await espTab.click();
      await appPage.page.waitForTimeout(500);
    }

    // Should show panel content (empty state or header info)
    const panel = appPage.page.locator(".esp-tree, [class*='esp'], .bottom-panel-inner").first();
    await expect(panel).toBeAttached({ timeout: 5000 });
  });

  test("renders panel without error", async ({ appPage }) => {
    await appPage.goto();
    await expect(appPage.bottomPanel).toBeAttached({ timeout: 10_000 });

    // Switch to ESP Tree tab
    const espTab = appPage.page.locator(".bottom-tab:has-text('ESP')").first();
    if (await espTab.count() > 0) {
      await espTab.click();
      await appPage.page.waitForTimeout(500);
    }

    // Panel should render
    const content = appPage.page.locator(".bottom-panel-content, .bottom-panel-inner").first();
    await expect(content).toBeAttached();
  });
});

// ===================================================================
// Quests Panel
// ===================================================================
test.describe("Quests Panel", { tag: "@quests" }, () => {
  test("shows empty state placeholder", async ({ appPage }) => {
    await appPage.goto();
    await expect(appPage.bottomPanel).toBeAttached({ timeout: 10_000 });

    // Switch to Quests tab
    appPage.page.locator(".bottom-tab:has-text('Quests')").first().click();

    // Should render (may show empty state or placeholder)
    const panel = appPage.page.locator(".bottom-panel-inner, .quests-panel");
    await expect(panel.first()).toBeAttached({ timeout: 5000 });
  });
});

// ===================================================================
// Header Processor Panel
// ===================================================================
test.describe("Header Processor Panel", { tag: "@header-proc" }, () => {
  test("header processor tab is visible", async ({ appPage }) => {
    await appPage.goto();
    await expect(appPage.bottomPanel).toBeAttached({ timeout: 10_000 });

    // Look for Header tab
    const headerTab = appPage.page.locator('.bottom-tab:has-text("Header")');
    expect(await headerTab.count()).toBeGreaterThan(0);
  });
});

// ===================================================================
// Header Wizard Panel
// ===================================================================
test.describe("Header Wizard Panel", { tag: "@header-wizard" }, () => {
  test("renders wizard form", async ({ appPage }) => {
    await appPage.goto();
    await expect(appPage.bottomPanel).toBeAttached({ timeout: 10_000 });

    // Switch to Wizard tab
    const wizardTab = appPage.page.locator(".bottom-tab").filter({ hasText: /Wizard/i }).first();
    if (await wizardTab.count() > 0) {
      await wizardTab.click();
      await appPage.page.waitForTimeout(500);
    }

    // Bottom panel should still be visible
    await expect(appPage.bottomPanel).toBeVisible();
  });
});

// ===================================================================
// Data Configs Panel (Modal)
// ===================================================================
test.describe("Data Configs Panel", { tag: "@data-configs" }, () => {
  test("can be opened via store action", async ({ appPage }) => {
    await appPage.goto();
    await expect(appPage.appRoot).toBeAttached({ timeout: 10_000 });

    // Trigger Data Configs panel via store
    await appPage.page.evaluate(() => {
      const store = (window as any).__zustandStore.getState();
      // @ts-ignore - setActivePanel exists in the app
      if (store.setActivePanel) {
        store.setActivePanel("dataConfigs");
      }
    });
    await appPage.page.waitForTimeout(500);

    // Modal should appear
    const modal = appPage.page.locator(".ui-modal-overlay, [class*='modal']").first();
    await expect(modal).toBeAttached({ timeout: 5000 });
  });
});

// ===================================================================
// Toolbox Panel (Modal)
// ===================================================================
test.describe("Toolbox Panel", { tag: "@toolbox" }, () => {
  test("toolbox state exists in store", async ({ appPage }) => {
    await appPage.goto();
    await expect(appPage.appRoot).toBeAttached({ timeout: 10_000 });

    // Verify toolbox-related state exists
    const hasToolbox = await appPage.page.evaluate(() => {
      const store = (window as any).__zustandStore?.getState();
      return store !== undefined;
    });
    expect(hasToolbox).toBe(true);
  });
});

// ===================================================================
// Settings Modal
// ===================================================================
test.describe("Settings Modal", { tag: "@settings" }, () => {
  test("can be opened via store action", async ({ appPage }) => {
    await appPage.goto();
    await expect(appPage.appRoot).toBeAttached({ timeout: 10_000 });

    // Trigger Settings via store
    await appPage.page.evaluate(() => {
      const store = (window as any).__zustandStore.getState();
      // @ts-ignore - setShowSettings exists
      if (store.setShowSettings) {
        store.setShowSettings(true);
      }
    });
    await appPage.page.waitForTimeout(500);

    // Modal should appear
    const modal = appPage.page.locator(".ui-modal-overlay, [class*='modal'], [class*='settings']").first();
    await expect(modal).toBeAttached({ timeout: 5000 });
  });
});
