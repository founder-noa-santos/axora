/**
 * Settings Store Tests
 * Tests for settings store functionality
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { useSettingsStore } from '../store/settings-store';
import { defaultSettings } from '../types/settings';

// Mock fetch
const mockFetch = vi.fn();
global.fetch = mockFetch;

describe('Settings Store', () => {
  beforeEach(() => {
    // Reset store to default state
    useSettingsStore.setState({
      settings: defaultSettings,
      isLoading: false,
      error: null,
      hasUnsavedChanges: false,
    });
    mockFetch.mockClear();
  });

  describe('Initial State', () => {
    it('should have correct initial state', () => {
      const state = useSettingsStore.getState();
      
      expect(state.settings).toEqual(defaultSettings);
      expect(state.isLoading).toBe(false);
      expect(state.error).toBe(null);
      expect(state.hasUnsavedChanges).toBe(false);
    });
  });

  describe('updateSetting', () => {
    it('should update model settings', () => {
      const { updateSetting } = useSettingsStore.getState();
      
      updateSetting('model', 'provider', 'openai');
      
      const updatedSettings = useSettingsStore.getState().settings;
      expect(updatedSettings.model.provider).toBe('openai');
      expect(useSettingsStore.getState().hasUnsavedChanges).toBe(true);
    });

    it('should update token settings', () => {
      const { updateSetting } = useSettingsStore.getState();
      
      updateSetting('tokens', 'maxTokensPerRequest', 8192);
      
      const updatedSettings = useSettingsStore.getState().settings;
      expect(updatedSettings.tokens.maxTokensPerRequest).toBe(8192);
    });

    it('should update worker settings', () => {
      const { updateSetting } = useSettingsStore.getState();
      
      updateSetting('workers', 'minWorkers', 5);
      
      const updatedSettings = useSettingsStore.getState().settings;
      expect(updatedSettings.workers.minWorkers).toBe(5);
    });

    it('should update theme settings', () => {
      const { updateSetting } = useSettingsStore.getState();
      
      updateSetting('theme', 'mode', 'light');
      
      const updatedSettings = useSettingsStore.getState().settings;
      expect(updatedSettings.theme.mode).toBe('light');
    });

    it('should update advanced settings', () => {
      const { updateSetting } = useSettingsStore.getState();
      
      updateSetting('advanced', 'logLevel', 'debug');
      
      const updatedSettings = useSettingsStore.getState().settings;
      expect(updatedSettings.advanced.logLevel).toBe('debug');
    });
  });

  describe('resetSettings', () => {
    it('should reset to default settings', () => {
      const { updateSetting, resetSettings } = useSettingsStore.getState();
      
      // Modify settings
      updateSetting('model', 'provider', 'openai');
      updateSetting('theme', 'mode', 'light');
      
      // Reset
      resetSettings();
      
      const state = useSettingsStore.getState();
      expect(state.settings).toEqual(defaultSettings);
      expect(state.hasUnsavedChanges).toBe(true);
    });
  });

  describe('exportSettings', () => {
    it('should export settings as JSON string', () => {
      const { exportSettings } = useSettingsStore.getState();
      
      const json = exportSettings();
      const parsed = JSON.parse(json);
      
      expect(parsed).toEqual(defaultSettings);
    });

    it('should export modified settings', () => {
      const { updateSetting, exportSettings } = useSettingsStore.getState();
      
      updateSetting('model', 'provider', 'anthropic');
      
      const json = exportSettings();
      const parsed = JSON.parse(json);
      
      expect(parsed.model.provider).toBe('anthropic');
    });
  });

  describe('importSettings', () => {
    it('should import valid settings', async () => {
      const { importSettings } = useSettingsStore.getState();
      
      const validJson = JSON.stringify({
        model: { provider: 'anthropic', model: 'claude-3' },
        tokens: { maxTokensPerRequest: 4096, maxContextTokens: 8192, tokenBudget: 100000 },
        workers: { minWorkers: 2, maxWorkers: 10, healthCheckInterval: 30 },
        theme: { mode: 'dark', accentColor: 'blue' },
        advanced: { enableLogging: true, logLevel: 'info', autoUpdate: true },
      });
      
      mockFetch.mockResolvedValueOnce({ ok: true });
      
      await importSettings(validJson);
      
      const state = useSettingsStore.getState();
      expect(state.settings.model.provider).toBe('anthropic');
      expect(state.settings.model.model).toBe('claude-3');
    });

    it('should reject invalid JSON', async () => {
      const { importSettings } = useSettingsStore.getState();
      
      await expect(importSettings('invalid-json')).rejects.toThrow();
    });

    it('should reject invalid settings', async () => {
      const { importSettings } = useSettingsStore.getState();
      
      const invalidJson = JSON.stringify({
        model: { provider: 'invalid-provider', model: 'test' },
      });
      
      await expect(importSettings(invalidJson)).rejects.toThrow();
    });
  });

  describe('saveSettings', () => {
    it('should save settings successfully', async () => {
      const { saveSettings, updateSetting } = useSettingsStore.getState();
      
      updateSetting('model', 'provider', 'openai');
      
      mockFetch.mockResolvedValueOnce({ ok: true });
      
      await saveSettings({ model: { provider: 'openai' } as any });
      
      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/settings'),
        expect.objectContaining({ method: 'PUT' })
      );
      
      const state = useSettingsStore.getState();
      expect(state.isLoading).toBe(false);
      expect(state.hasUnsavedChanges).toBe(false);
    });

    it('should handle save error', async () => {
      const { saveSettings } = useSettingsStore.getState();
      
      mockFetch.mockRejectedValueOnce(new Error('Network error'));
      
      await saveSettings({ model: { provider: 'openai' } as any });
      
      const state = useSettingsStore.getState();
      expect(state.error).toBe('Failed to save settings');
      expect(state.isLoading).toBe(false);
    });

    it('should validate settings before saving', async () => {
      const { saveSettings } = useSettingsStore.getState();
      
      await saveSettings({ 
        tokens: { maxTokensPerRequest: 50 } as any // Invalid value
      });
      
      const state = useSettingsStore.getState();
      expect(state.error).toContain('Validation failed');
    });
  });

  describe('loadSettings', () => {
    it('should load settings from backend', async () => {
      const { loadSettings } = useSettingsStore.getState();
      
      const mockSettings = {
        ...defaultSettings,
        model: { ...defaultSettings.model, provider: 'openai' as const },
      };
      
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => mockSettings,
      });
      
      await loadSettings();
      
      const state = useSettingsStore.getState();
      expect(state.settings.model.provider).toBe('openai');
      expect(state.isLoading).toBe(false);
    });

    it('should handle load error gracefully', async () => {
      const { loadSettings } = useSettingsStore.getState();
      
      mockFetch.mockRejectedValueOnce(new Error('Network error'));
      
      await loadSettings();
      
      const state = useSettingsStore.getState();
      expect(state.isLoading).toBe(false);
      // Should keep local settings
      expect(state.settings).toEqual(defaultSettings);
    });
  });

  describe('clearError', () => {
    it('should clear error state', () => {
      useSettingsStore.setState({ error: 'Test error' });
      const { clearError } = useSettingsStore.getState();
      
      clearError();
      
      expect(useSettingsStore.getState().error).toBe(null);
    });
  });

  describe('markAsSaved', () => {
    it('should mark settings as saved', () => {
      const { updateSetting, markAsSaved } = useSettingsStore.getState();
      
      updateSetting('model', 'provider', 'openai');
      expect(useSettingsStore.getState().hasUnsavedChanges).toBe(true);
      
      markAsSaved();
      expect(useSettingsStore.getState().hasUnsavedChanges).toBe(false);
    });
  });
});
