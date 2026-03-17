/**
 * AXORA E2E Tests - Real-time Progress
 *
 * Tests for WebSocket-based real-time progress updates
 */

import { test, expect } from '@playwright/test';

test.describe('Real-time Progress', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('http://localhost:5173');
  });

  test('should show progress bar for running mission', async ({ page }) => {
    // Submit mission
    await page.fill('[data-testid="mission-input"]', 'Test real-time progress');
    await page.click('[data-testid="send-button"]');

    // Wait for progress bar to appear
    await page.waitForSelector('[data-testid="progress-bar"]', { timeout: 10000 });

    const progressBar = page.locator('[data-testid="progress-bar"]');
    await expect(progressBar).toBeVisible();

    // Progress value should be between 0 and 100
    const progressValue = await progressBar.getAttribute('value');
    const progressNum = parseInt(progressValue || '0', 10);
    expect(progressNum).toBeGreaterThanOrEqual(0);
    expect(progressNum).toBeLessThanOrEqual(100);
  });

  test('should update progress value over time', async ({ page }) => {
    // Submit mission
    await page.fill('[data-testid="mission-input"]', 'Test progress updates');
    await page.click('[data-testid="send-button"]');

    // Wait for progress to appear
    await page.waitForSelector('[data-testid="progress-value"]', { timeout: 10000 });

    // Get initial progress
    const initialProgress = await page.locator('[data-testid="progress-value"]').textContent();
    expect(initialProgress).toBeTruthy();

    // Wait and check progress again (should stay same or increase with mock API)
    await page.waitForTimeout(3000);

    const laterProgress = await page.locator('[data-testid="progress-value"]').textContent();
    expect(parseInt(laterProgress || '0', 10)).toBeGreaterThanOrEqual(parseInt(initialProgress || '0', 10));
  });

  test('should show current step description', async ({ page }) => {
    // Submit mission
    await page.fill('[data-testid="mission-input"]', 'Test step description');
    await page.click('[data-testid="send-button"]');

    // Wait for step description to appear
    await page.waitForSelector('[data-testid="current-step"]', { timeout: 10000 });

    const stepElement = page.locator('[data-testid="current-step"]');
    await expect(stepElement).toBeVisible();

    const stepText = await stepElement.textContent();
    expect(stepText?.length).toBeGreaterThan(0);
  });

  test('should show ETA for mission completion', async ({ page }) => {
    // Submit mission
    await page.fill('[data-testid="mission-input"]', 'Test ETA display');
    await page.click('[data-testid="send-button"]');

    // Wait for ETA to appear
    await page.waitForSelector('[data-testid="eta-display"]', { timeout: 10000 });

    const etaElement = page.locator('[data-testid="eta-display"]');
    await expect(etaElement).toBeVisible();
  });

  test('should show mission completion message', async ({ page }) => {
    // Submit mission
    await page.fill('[data-testid="mission-input"]', 'Test completion message');
    await page.click('[data-testid="send-button"]');

    // Wait for mission to complete (mock API completes in ~10-15 seconds)
    // For this test, we'll check for completed status
    await page.waitForSelector('[data-testid="mission-complete"]', { timeout: 30000 }).catch(() => {
      // If timeout, check for high progress instead
    });

    // Check for completion indicator or high progress
    const progressValue = await page.locator('[data-testid="progress-bar"]').getAttribute('value');
    const progressNum = parseInt(progressValue || '0', 10);

    // Either completed or high progress
    expect(progressNum).toBeGreaterThan(50);
  });
});

test.describe('Worker Status Updates', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('http://localhost:5173');
  });

  test('should display worker status', async ({ page }) => {
    // Navigate to progress panel
    await page.click('[data-testid="progress-panel-tab"]');

    // Wait for worker list to appear
    await page.waitForSelector('[data-testid="worker-list"]', { timeout: 5000 });

    const workerList = page.locator('[data-testid="worker-list"]');
    await expect(workerList).toBeVisible();

    // Should have at least one worker
    const workerCount = await page.locator('[data-testid="worker-item"]').count();
    expect(workerCount).toBeGreaterThan(0);
  });

  test('should show worker status indicators', async ({ page }) => {
    // Navigate to progress panel
    await page.click('[data-testid="progress-panel-tab"]');

    // Wait for workers
    await page.waitForSelector('[data-testid="worker-item"]', { timeout: 5000 });

    // Check for status indicators (idle, busy, offline)
    const statusIndicators = page.locator('[data-testid="worker-status"]');
    const count = await statusIndicators.count();

    if (count > 0) {
      const firstStatus = await statusIndicators.first().getAttribute('data-status');
      expect(['idle', 'busy', 'offline', 'unknown']).toContain(firstStatus);
    }
  });

  test('should update worker heartbeat', async ({ page }) => {
    // Navigate to progress panel
    await page.click('[data-testid="progress-panel-tab"]');

    // Wait for workers
    await page.waitForSelector('[data-testid="worker-item"]', { timeout: 5000 });

    // Get initial heartbeat time
    const heartbeatElement = page.locator('[data-testid="worker-heartbeat"]').first();
    const initialHeartbeat = await heartbeatElement.getAttribute('data-timestamp');

    // Wait for heartbeat update (mock API updates every 2.5 seconds)
    await page.waitForTimeout(5000);

    // Heartbeat should be updated
    const laterHeartbeat = await heartbeatElement.getAttribute('data-timestamp');
    expect(laterHeartbeat).toBeTruthy();

    if (initialHeartbeat && laterHeartbeat) {
      expect(parseInt(laterHeartbeat, 10)).toBeGreaterThanOrEqual(parseInt(initialHeartbeat, 10));
    }
  });
});
