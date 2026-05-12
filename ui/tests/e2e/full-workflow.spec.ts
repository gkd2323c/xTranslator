import { test, expect } from '@playwright/test';
import path from 'path';

// Test configuration
const TEST_DATA_DIR = path.join(__dirname, '../../test-data');
const SKYRIM_ESM_PATH = 'D:\\SteamLibrary\\steamapps\\common\\Skyrim Special Edition\\Data\\Skyrim.esm';

test.describe('xTranslator E2E Workflow Tests', () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to the app
    await page.goto('http://localhost:5173');
    
    // Wait for the app to load
    await page.waitForSelector('[data-testid="app-main"]', { timeout: 10000 });
  });

  test('should load and display the main interface', async ({ page }) => {
    // Check main components are visible
    await expect(page.locator('[data-testid="menu-bar"]')).toBeVisible();
    await expect(page.locator('[data-testid="string-table"]')).toBeVisible();
    await expect(page.locator('[data-testid="status-bar"]')).toBeVisible();
    
    // Check initial state
    await expect(page.locator('[data-testid="stats-panel"]')).toContainText('Total: 0');
  });

  test('should load Skyrim ESP file successfully', async ({ page }) => {
    // Mock file dialog (in real tests, you'd need to handle actual file selection)
    await page.route('**/load_esp', route => {
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          total: 75000,
          compressed_records: 44000,
          strings_loaded: 75000,
          parse_time_ms: 2500,
          record_counts: { 'INFO': 15000, 'NPC_': 8000, 'QUST': 5000 },
          cached: false,
          esp_hash: 'abc123'
        })
      });
    });

    // Click load ESP button
    await page.click('[data-testid="load-esp-btn"]');
    
    // Wait for loading to complete
    await expect(page.locator('[data-testid="loading-spinner"]')).toBeHidden();
    
    // Verify stats updated
    await expect(page.locator('[data-testid="stats-panel"]')).toContainText('Total: 75000');
    
    // Verify table has data
    const rows = await page.locator('[data-testid="string-table-row"]').count();
    expect(rows).toBeGreaterThan(0);
  });

  test('should filter strings correctly', async ({ page }) => {
    // First load some test data
    await loadTestData(page);
    
    // Test text filtering
    await page.fill('[data-testid="filter-input"]', 'Dragon');
    await expect(page.locator('[data-testid="string-table-row"]')).toHaveCount(10);
    
    // Test record type filtering
    await page.selectOption('[data-testid="record-filter"]', 'INFO');
    await expect(page.locator('[data-testid="string-table-row"]')).toHaveCount(5);
    
    // Test status filtering
    await page.selectOption('[data-testid="status-filter"]', 'translated');
    await expect(page.locator('[data-testid="string-table-row"]')).toHaveCount(3);
  });

  test('should edit translation in modal dialog', async ({ page }) => {
    await loadTestData(page);
    
    // Double-click first row to open editor
    await page.dblclick('[data-testid="string-table-row"]:first-child');
    
    // Check editor dialog opens
    await expect(page.locator('[data-testid="editor-dialog"]')).toBeVisible();
    await expect(page.locator('[data-testid="source-text"]')).toBeVisible();
    await expect(page.locator('[data-testid="translation-textarea"]')).toBeVisible();
    
    // Edit translation
    const testTranslation = '测试翻译内容';
    await page.fill('[data-testid="translation-textarea"]', testTranslation);
    
    // Save with Ctrl+Enter
    await page.press('[data-testid="translation-textarea"]', 'Control+Enter');
    
    // Verify dialog closes
    await expect(page.locator('[data-testid="editor-dialog"]')).toBeHidden();
    
    // Verify translation is updated in table
    const firstRowTranslation = await page.locator('[data-testid="string-table-row"]:first-child [data-testid="translation-cell"]').textContent();
    expect(firstRowTranslation).toBe(testTranslation);
  });

  test('should perform heuristic search', async ({ page }) => {
    await loadTestData(page);
    
    // Open editor for a string
    await page.dblclick('[data-testid="string-table-row"]:first-child');
    
    // Click heuristic search button
    await page.click('[data-testid="heuristic-search-btn"]');
    
    // Wait for search results
    await expect(page.locator('[data-testid="heuristic-results"]')).toBeVisible();
    
    // Verify results are displayed
    const results = await page.locator('[data-testid="heuristic-result-item"]').count();
    expect(results).toBeGreaterThan(0);
    
    // Click a result to apply it
    await page.click('[data-testid="heuristic-result-item"]:first-child');
    
    // Verify translation is filled
    const translation = await page.inputValue('[data-testid="translation-textarea"]');
    expect(translation.length).toBeGreaterThan(0);
  });

  test('should handle batch translation', async ({ page }) => {
    await loadTestData(page);
    
    // Select multiple items
    await page.click('[data-testid="string-table-row"]:nth-child(1) [data-testid="checkbox"]');
    await page.click('[data-testid="string-table-row"]:nth-child(2) [data-testid="checkbox"]');
    await page.click('[data-testid="string-table-row"]:nth-child(3) [data-testid="checkbox"]');
    
    // Open batch translate dialog
    await page.click('[data-testid="batch-translate-btn"]');
    
    // Verify dialog opens
    await expect(page.locator('[data-testid="batch-translate-dialog"]')).toBeVisible();
    await expect(page.locator('[data-testid="batch-translate-dialog"]')).toContainText('3 items selected');
    
    // Start translation
    await page.click('[data-testid="start-batch-translate-btn"]');
    
    // Wait for progress
    await expect(page.locator('[data-testid="batch-progress"]')).toBeVisible();
    
    // Wait for completion (mock success)
    await page.waitForTimeout(2000);
    await expect(page.locator('[data-testid="batch-success-message"]')).toBeVisible();
  });

  test('should export to XML correctly', async ({ page }) => {
    await loadTestData(page);
    
    // Mock file save dialog
    const downloadPromise = page.waitForEvent('download');
    
    // Click export XML button
    await page.click('[data-testid="export-xml-btn"]');
    
    // Wait for download
    const download = await downloadPromise;
    
    // Verify file name
    expect(download.suggestedFilename()).toMatch(/\.xml$/);
    
    // Save to temp location
    const tempPath = path.join(TEST_DATA_DIR, 'test_export.xml');
    await download.saveAs(tempPath);
    
    // Verify file exists and has content
    const fs = require('fs');
    expect(fs.existsSync(tempPath)).toBe(true);
    const content = fs.readFileSync(tempPath, 'utf8');
    expect(content).toContain('<?xml version="1.0"');
    expect(content).toContain('<strings>');
  });

  test('should import from XML correctly', async ({ page }) => {
    await loadTestData(page);
    
    // Mock file dialog for XML import
    await page.setInputFiles('[data-testid="import-xml-input"]', 'test-data/test_import.xml');
    
    // Wait for import to complete
    await expect(page.locator('[data-testid="import-success-message"]')).toBeVisible();
    
    // Verify translations are updated
    const translatedCount = await page.locator('[data-testid="string-table-row"][data-status="translated"]').count();
    expect(translatedCount).toBeGreaterThan(0);
  });

  test('should handle SST dictionary operations', async ({ page }) => {
    await loadTestData(page);
    
    // Open SST panel
    await page.click('[data-testid="sst-panel-btn"]');
    await expect(page.locator('[data-testid="sst-panel"]')).toBeVisible();
    
    // Load SST dictionary
    await page.click('[data-testid="load-sst-btn"]');
    
    // Wait for load to complete
    await expect(page.locator('[data-testid="sst-stats"]')).toBeVisible();
    
    // Apply SST translations
    await page.click('[data-testid="apply-sst-btn"]');
    
    // Verify progress
    await expect(page.locator('[data-testid="sst-progress"]')).toBeVisible();
    
    // Wait for completion
    await expect(page.locator('[data-testid="sst-applied-message"]')).toBeVisible();
  });

  test('should handle ESP comparison', async ({ page }) => {
    await loadTestData(page);
    
    // Open ESP compare panel
    await page.click('[data-testid="esp-compare-btn"]');
    await expect(page.locator('[data-testid="esp-compare-panel"]')).toBeVisible();
    
    // Load comparison ESP (mock)
    await page.setInputFiles('[data-testid="compare-esp-input"]', 'test-data/compare.esp');
    
    // Wait for comparison
    await expect(page.locator('[data-testid="compare-results"]')).toBeVisible();
    
    // Verify comparison stats
    await expect(page.locator('[data-testid="compare-stats"]')).toContainText('Added');
    await expect(page.locator('[data-testid="compare-stats"]')).toContainText('Removed');
    await expect(page.locator('[data-testid="compare-stats"]')).toContainText('Modified');
  });

  test('should handle theme switching', async ({ page }) => {
    // Check default theme
    const body = page.locator('body');
    await expect(body).toHaveClass(/obsidian|dark/);
    
    // Switch to light theme
    await page.click('[data-testid="theme-selector"]');
    await page.click('[data-testid="theme-light"]');
    
    // Verify theme changed
    await expect(body).toHaveClass(/light/);
    
    // Switch back to dark
    await page.click('[data-testid="theme-selector"]');
    await page.click('[data-testid="theme-dark"]');
    
    // Verify theme restored
    await expect(body).toHaveClass(/dark/);
  });

  test('should handle keyboard shortcuts', async ({ page }) => {
    await loadTestData(page);
    
    // Test Ctrl+F for search
    await page.press('body', 'Control+f');
    await expect(page.locator('[data-testid="filter-input"]')).toBeFocused();
    
    // Test Escape to close dialogs
    await page.dblclick('[data-testid="string-table-row"]:first-child');
    await expect(page.locator('[data-testid="editor-dialog"]')).toBeVisible();
    await page.press('body', 'Escape');
    await expect(page.locator('[data-testid="editor-dialog"]')).toBeHidden();
    
    // Test Ctrl+Z for undo
    await page.dblclick('[data-testid="string-table-row"]:first-child');
    await page.fill('[data-testid="translation-textarea"]', 'Test translation');
    await page.press('[data-testid="translation-textarea"]', 'Control+Enter');
    await page.press('body', 'Control+z');
    
    // Verify undo (translation should be empty)
    await page.dblclick('[data-testid="string-table-row"]:first-child');
    const translation = await page.inputValue('[data-testid="translation-textarea"]');
    expect(translation).toBe('');
  });

  test('should handle error states gracefully', async ({ page }) => {
    // Mock API error
    await page.route('**/load_esp', route => {
      route.fulfill({
        status: 500,
        contentType: 'application/json',
        body: JSON.stringify({ error: 'Failed to load ESP file' })
      });
    });

    // Attempt to load ESP
    await page.click('[data-testid="load-esp-btn"]');
    
    // Verify error message is shown
    await expect(page.locator('[data-testid="error-message"]')).toBeVisible();
    await expect(page.locator('[data-testid="error-message"]')).toContainText('Failed to load ESP file');
    
    // Verify app is still functional
    await expect(page.locator('[data-testid="menu-bar"]')).toBeVisible();
  });

  test('should handle large datasets efficiently', async ({ page }) => {
    // Mock large dataset
    await page.route('**/get_strings_chunk', route => {
      const chunk = Array.from({ length: 25000 }, (_, i) => ({
        id: i + 1,
        source: `Test string ${i + 1}`,
        translation: i % 3 === 0 ? `测试字符串 ${i + 1}` : '',
        record_sig: ['INFO', 'NPC_', 'QUST'][i % 3],
        field_sig: 'FULL',
        form_id: `0x${(1000 + i).toString(16)}`,
        status: i % 3 === 0 ? 'translated' : 'untranslated',
        list_index: i,
        str_id: i + 1,
        is_vmad: false,
        ld: 0
      }));
      
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(chunk)
      });
    });

    // Load large dataset
    await loadTestData(page);
    
    // Measure performance
    const startTime = Date.now();
    await expect(page.locator('[data-testid="string-table-row"]')).toHaveCount(25000);
    const loadTime = Date.now() - startTime;
    
    // Should load within reasonable time
    expect(loadTime).toBeLessThan(5000);
    
    // Test filtering performance
    const filterStart = Date.now();
    await page.fill('[data-testid="filter-input"]', 'Test');
    await expect(page.locator('[data-testid="string-table-row"]')).toHaveCount(25000);
    const filterTime = Date.now() - filterStart;
    
    // Filtering should be fast
    expect(filterTime).toBeLessThan(1000);
  });
});

// Helper function to load test data
async function loadTestData(page: any) {
  // Mock ESP loading
  await page.route('**/load_esp', route => {
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        total: 1000,
        compressed_records: 600,
        strings_loaded: 1000,
        parse_time_ms: 500,
        record_counts: { 'INFO': 400, 'NPC_': 300, 'QUST': 300 },
        cached: false,
        esp_hash: 'test123'
      })
    });
  });

  // Mock string chunks
  await page.route('**/get_strings_chunk', route => {
    const chunk = Array.from({ length: 1000 }, (_, i) => ({
      id: i + 1,
      source: `Test string ${i + 1}`,
      translation: i % 4 === 0 ? `测试字符串 ${i + 1}` : '',
      record_sig: ['INFO', 'NPC_', 'QUST'][i % 3],
      field_sig: 'FULL',
      form_id: `0x${(1000 + i).toString(16)}`,
      status: i % 4 === 0 ? 'translated' : 'untranslated',
      list_index: i,
      str_id: i + 1,
      is_vmad: false,
      ld: 0
    }));
    
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(chunk)
    });
  });

  // Trigger load
  await page.click('[data-testid="load-esp-btn"]');
  await expect(page.locator('[data-testid="loading-spinner"]')).toBeHidden();
  await expect(page.locator('[data-testid="string-table-row"]')).toHaveCount(1000);
}
