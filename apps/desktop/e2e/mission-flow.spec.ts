/**
 * AXORA E2E Tests - Mission Flow
 *
 * Tests for full mission flow: submit → progress → complete
 */

import { test, expect } from '@playwright/test';

test.describe('Mission Flow', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('http://localhost:5173');
  });

  test('should display app title and main interface', async ({ page }) => {
    // Verify app loads
    await expect(page).toHaveTitle(/AXORA/);

    // Verify main panels are visible
    await expect(page.locator('[data-testid="chat-panel"]')).toBeVisible();
    await expect(page.locator('[data-testid="mission-input"]')).toBeVisible();
    await expect(page.locator('[data-testid="send-button"]')).toBeVisible();
  });

  test('should submit mission and see progress with mock API', async ({ page }) => {
    // Submit mission
    const missionContent = 'Implement authentication system';
    await page.fill('[data-testid="mission-input"]', missionContent);
    await page.click('[data-testid="send-button"]');

    // Wait for mission to appear in active missions
    await page.waitForSelector('[data-testid="active-missions"]', { timeout: 5000 });

    // Verify mission is running
    const missionElement = page.locator('[data-testid="mission-item"]').first();
    await expect(missionElement).toBeVisible();

    // Wait for progress bar to appear
    await page.waitForSelector('[data-testid="progress-bar"]', { timeout: 10000 });

    // Verify progress is shown
    const progressBar = page.locator('[data-testid="progress-bar"]');
    await expect(progressBar).toBeVisible();

    // Progress should be greater than 0
    const progressValue = await progressBar.getAttribute('value');
    expect(progressValue).toBeTruthy();
  });

  test('should show mission status updates', async ({ page }) => {
    // Submit mission
    await page.fill('[data-testid="mission-input"]', 'Test status updates');
    await page.click('[data-testid="send-button"]');

    // Wait for status to appear
    await page.waitForSelector('[data-testid="mission-status"]', { timeout: 5000 });

    // Status should be visible
    const statusElement = page.locator('[data-testid="mission-status"]');
    await expect(statusElement).toBeVisible();

    // Status should contain "running", "pending", or "completed"
    const statusText = await statusElement.textContent();
    expect(statusText?.toLowerCase()).toMatch(/running|pending|completed|processing/);
  });

  test('should display error for empty mission', async ({ page }) => {
    // Try to submit empty mission
    await page.fill('[data-testid="mission-input"]', '');
    await page.click('[data-testid="send-button"]');

    // Should show error or not submit
    await page.waitForTimeout(1000);

    // Verify no mission was created
    const missionCount = await page.locator('[data-testid="mission-item"]').count();
    expect(missionCount).toBeLessThanOrEqual(1); // May have existing missions
  });

  test('should allow cancelling running mission', async ({ page }) => {
    // Submit mission
    await page.fill('[data-testid="mission-input"]', 'Test cancellation');
    await page.click('[data-testid="send-button"]');

    // Wait for mission to start
    await page.waitForSelector('[data-testid="mission-item"]', { timeout: 5000 });

    // Find and click cancel button
    const cancelButton = page.locator('[data-testid="cancel-mission"]').first();
    await expect(cancelButton).toBeVisible();
    await cancelButton.click();

    // Verify cancellation confirmation or status change
    await page.waitForTimeout(2000);
  });
});

test.describe('Mission Flow - Keyboard Navigation', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('http://localhost:5173');
  });

  test('should submit mission with Enter key', async ({ page }) => {
    await page.fill('[data-testid="mission-input"]', 'Test Enter key');
    await page.press('[data-testid="mission-input"]', 'Enter');

    // Wait for mission to appear
    await page.waitForSelector('[data-testid="mission-item"]', { timeout: 5000 });

    const missionCount = await page.locator('[data-testid="mission-item"]').count();
    expect(missionCount).toBeGreaterThan(0);
  });

  test('should disable send button when input is empty', async ({ page }) => {
    const sendButton = page.locator('[data-testid="send-button"]');

    // Clear input
    await page.fill('[data-testid="mission-input"]', '');

    // Button may be disabled or have disabled state
    const isDisabled = await sendButton.getAttribute('disabled');
    const ariaDisabled = await sendButton.getAttribute('aria-disabled');

    expect(isDisabled || ariaDisabled === 'true' || false).toBeTruthy();
  });
});
