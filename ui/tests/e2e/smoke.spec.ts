import { test, expect } from '@playwright/test';

test.describe('smoke tests', () => {
  test('main UI loads', async ({ page }) => {
    // Use baseURL from playwright.config.ts
    await page.goto('/');
    await page.waitForSelector('[data-testid="app-main"]', { timeout: 10000 });
    await expect(page.locator('[data-testid="menu-bar"]')).toBeVisible();
    await expect(page.locator('[data-testid="string-table"]')).toBeVisible();
  });
});
