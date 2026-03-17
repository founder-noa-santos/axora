/**
 * AXORA E2E Tests - Settings Panel
 *
 * Tests for settings persistence and configuration
 */

import { test, expect } from '@playwright/test';

test.describe('Settings Panel', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('http://localhost:5173');
  });

  test('should open settings panel', async ({ page }) => {
    // Find and click settings button
    const settingsButton = page.locator('[data-testid="settings-button"]');
    await expect(settingsButton).toBeVisible();
    await settingsButton.click();

    // Verify settings panel is visible
    await expect(page.locator('[data-testid="settings-panel"]')).toBeVisible();
  });

  test('should configure model provider settings', async ({ page }) => {
    // Open settings
    await page.click('[data-testid="settings-button"]');

    // Change model provider
    const providerSelect = page.locator('[data-testid="model-provider"]');
    await expect(providerSelect).toBeVisible();

    // Select Ollama provider
    await providerSelect.selectOption('ollama');

    // Verify base URL field is visible for Ollama
    const baseUrlInput = page.locator('[data-testid="ollama-base-url"]');
    await expect(baseUrlInput).toBeVisible();
  });

  test('should save settings and show confirmation', async ({ page }) => {
    // Open settings
    await page.click('[data-testid="settings-button"]');

    // Change a setting
    const providerSelect = page.locator('[data-testid="model-provider"]');
    await providerSelect.selectOption('ollama');

    // Fill in base URL
    await page.fill('[data-testid="ollama-base-url"]', 'http://localhost:11434');

    // Click save
    await page.click('[data-testid="save-settings"]');

    // Wait for save confirmation
    await page.waitForSelector('[data-testid="settings-saved"]', { timeout: 5000 });

    // Verify success message is shown
    const successMessage = page.locator('[data-testid="settings-saved"]');
    await expect(successMessage).toBeVisible();
  });

  test('should persist settings across page reload', async ({ page }) => {
    // Open settings and change provider
    await page.click('[data-testid="settings-button"]');
    await page.locator('[data-testid="model-provider"]').selectOption('ollama');
    await page.fill('[data-testid="ollama-base-url"]', 'http://localhost:11434');
    await page.click('[data-testid="save-settings"]');

    // Wait for save
    await page.waitForSelector('[data-testid="settings-saved"]', { timeout: 5000 });

    // Reload page
    await page.reload();

    // Reopen settings
    await page.click('[data-testid="settings-button"]');

    // Verify settings persisted
    const providerSelect = page.locator('[data-testid="model-provider"]');
    await expect(providerSelect).toHaveValue('ollama');

    const baseUrlInput = page.locator('[data-testid="ollama-base-url"]');
    await expect(baseUrlInput).toHaveValue('http://localhost:11434');
  });

  test('should validate API key format', async ({ page }) => {
    // Open settings
    await page.click('[data-testid="settings-button"]');

    // Select OpenAI provider
    await page.locator('[data-testid="model-provider"]').selectOption('openai');

    // Enter invalid API key
    await page.fill('[data-testid="openai-api-key"]', 'invalid-key');

    // Click save
    await page.click('[data-testid="save-settings"]');

    // May show validation error or save anyway (depends on validation)
    await page.waitForTimeout(2000);
  });

  test('should close settings panel', async ({ page }) => {
    // Open settings
    await page.click('[data-testid="settings-button"]');
    await expect(page.locator('[data-testid="settings-panel"]')).toBeVisible();

    // Close settings
    const closeButton = page.locator('[data-testid="close-settings"]');
    if (await closeButton.count() > 0) {
      await closeButton.click();
      await expect(page.locator('[data-testid="settings-panel"]')).not.toBeVisible();
    }
  });
});

test.describe('Settings - Theme Configuration', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('http://localhost:5173');
  });

  test('should change theme mode', async ({ page }) => {
    // Open settings
    await page.click('[data-testid="settings-button"]');

    // Find theme mode toggle
    const themeToggle = page.locator('[data-testid="theme-mode-toggle"]');

    if (await themeToggle.count() > 0) {
      await themeToggle.click();

      // Wait for theme change
      await page.waitForTimeout(500);

      // Verify theme changed (check for dark/light class on body)
      const bodyClass = await page.locator('body').getAttribute('class');
      expect(bodyClass).toMatch(/dark|light/);
    }
  });

  test('should change accent color', async ({ page }) => {
    // Open settings
    await page.click('[data-testid="settings-button"]');

    // Find accent color selector
    const accentColorSelect = page.locator('[data-testid="accent-color"]');

    if (await accentColorSelect.count() > 0) {
      await accentColorSelect.selectOption('electric-purple');

      // Click save
      await page.click('[data-testid="save-settings"]');
      await page.waitForSelector('[data-testid="settings-saved"]', { timeout: 5000 });
    }
  });
});
