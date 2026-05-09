import { test, expect } from "./fixtures/base";

test.describe("xTranslator App", { tag: "@smoke" }, () => {
  test("app loads and renders the root element", async ({ appPage }) => {
    await appPage.goto();
    await expect(appPage.appRoot).toBeAttached();
  });

  test("app displays the logo/brand in the menu bar", async ({ appPage }) => {
    await appPage.goto();
    await expect(appPage.menuBar).toBeAttached({ timeout: 10_000 });
  });

  test("app shows the string table area", async ({ appPage }) => {
    await appPage.goto();
    // The string table should eventually appear after data loads
    await expect(appPage.stringTable).toBeAttached({ timeout: 10_000 });
  });

  test("app shows the status bar at the bottom", async ({ appPage }) => {
    await appPage.goto();
    await expect(appPage.statusBar).toBeAttached({ timeout: 10_000 });
  });

  test("app has a visible bottom panel with tabs", async ({ appPage }) => {
    await appPage.goto();
    await expect(appPage.bottomPanel).toBeAttached({ timeout: 10_000 });
  });
});

test.describe("xTranslator Theme System", { tag: "@theme" }, () => {
  test("applies theme class to root element", async ({ appPage }) => {
    await appPage.goto();

    // The app has themes: obsidian, dark, light, slate, auto
    const html = appPage.page.locator("html");
    await expect(html).toHaveAttribute("data-theme", /obsidian|dark|light|slate|auto/);
  });
});
