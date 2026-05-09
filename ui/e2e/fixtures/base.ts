import { test as base, expect, Page } from "@playwright/test";

/**
 * Extended test fixture with shared helpers for xTranslator E2E tests.
 */

// ---- Page Objects ----

export class AppPage {
  constructor(public readonly page: Page) {}

  /** Navigate to the app and wait for it to load */
  async goto() {
    await this.page.goto("/");
    // The app initializes i18n and attempts Tauri IPC — wait for the menu bar to appear
    await this.page.waitForLoadState("networkidle");
  }

  /** Get the main application container */
  get appRoot() {
    return this.page.locator("#root");
  }

  /** Menu bar container */
  get menuBar() {
    // The top-level .menubar div is the root, but there are nested .menubar-* elements.
    // Use the first/brand element as a reliable anchor.
    return this.page.locator('.menubar-brand');
  }

  /** Side panel (left sidebar) */
  get sidePanel() {
    return this.page.locator('.sidepanel');
  }

  /** The main string table / list area */
  get stringTable() {
    return this.page.locator('[class*="stringTable"], [class*="string-table"], [class*="StringTable"]');
  }

  /** Status bar at the bottom */
  get statusBar() {
    return this.page.locator('.statusbar');
  }

  /** Bottom panel area */
  get bottomPanel() {
    return this.page.locator('.app-bottom-panel');
  }

  /** Panel buttons in the sidebar */
  get panelButtons() {
    return this.page.locator('button:has-text("Batch"), button:has-text("BSA"), button:has-text("PEX"), button:has-text("FUZ")');
  }

  /** Language switcher */
  get languageSwitcher() {
    return this.page.locator('[class*="langSwitch"], [class*="lang-switch"], button:has-text("English"), button:has-text("中文")');
  }

  /** Translation editor dialog */
  get editorDialog() {
    return this.page.locator('[class*="editorDialog"], [class*="editor-dialog"], [class*="EditorDialog"]');
  }
}

// ---- Extended Fixture ----

export const test = base.extend<{ appPage: AppPage }>({
  appPage: async ({ page }, use) => {
    const appPage = new AppPage(page);
    await use(appPage);
  },
});

export { expect };
