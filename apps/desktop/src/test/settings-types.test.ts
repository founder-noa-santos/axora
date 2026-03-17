/**
 * Settings Types Tests
 * Tests for settings validation and default values
 */

import { describe, it, expect } from 'vitest';
import { 
  defaultSettings, 
  validateSettings, 
  settingsValidation 
} from '../types/settings';

describe('Settings Types', () => {
  describe('defaultSettings', () => {
    it('should have correct default model configuration', () => {
      expect(defaultSettings.model.provider).toBe('ollama');
      expect(defaultSettings.model.model).toBe('qwen2.5-coder:7b');
      expect(defaultSettings.model.baseUrl).toBe('http://localhost:11434');
    });

    it('should have correct default token limits', () => {
      expect(defaultSettings.tokens.maxTokensPerRequest).toBe(4096);
      expect(defaultSettings.tokens.maxContextTokens).toBe(8192);
      expect(defaultSettings.tokens.tokenBudget).toBe(100000);
    });

    it('should have correct default worker pool settings', () => {
      expect(defaultSettings.workers.minWorkers).toBe(2);
      expect(defaultSettings.workers.maxWorkers).toBe(10);
      expect(defaultSettings.workers.healthCheckInterval).toBe(30);
    });

    it('should have correct default theme settings', () => {
      expect(defaultSettings.theme.mode).toBe('dark');
      expect(defaultSettings.theme.accentColor).toBe('electric-purple');
    });

    it('should have correct default advanced settings', () => {
      expect(defaultSettings.advanced.enableLogging).toBe(true);
      expect(defaultSettings.advanced.logLevel).toBe('info');
      expect(defaultSettings.advanced.autoUpdate).toBe(true);
    });
  });

  describe('validateSettings', () => {
    it('should validate correct settings', () => {
      const result = validateSettings({
        model: { provider: 'ollama', model: 'test-model' },
        tokens: { maxTokensPerRequest: 4096, maxContextTokens: 8192, tokenBudget: 100000 },
        workers: { minWorkers: 2, maxWorkers: 10, healthCheckInterval: 30 },
        theme: { mode: 'dark', accentColor: 'blue' },
        advanced: { enableLogging: true, logLevel: 'info', autoUpdate: true },
      });
      
      expect(result.valid).toBe(true);
      expect(Object.keys(result.errors).length).toBe(0);
    });

    it('should reject invalid provider', () => {
      const result = validateSettings({
        model: { provider: 'invalid' as any, model: 'test' },
      });
      
      expect(result.valid).toBe(false);
      expect(result.errors['model.provider']).toBeDefined();
    });

    it('should reject invalid token limits', () => {
      const result = validateSettings({
        tokens: { maxTokensPerRequest: 50, maxContextTokens: 8192, tokenBudget: 100000 },
      });
      
      expect(result.valid).toBe(false);
      expect(result.errors['tokens.maxTokensPerRequest']).toBeDefined();
    });

    it('should reject invalid worker count', () => {
      const result = validateSettings({
        workers: { minWorkers: 15, maxWorkers: 10, healthCheckInterval: 30 },
      });
      
      expect(result.valid).toBe(false);
      expect(result.errors['workers.minWorkers']).toBeDefined();
    });

    it('should reject invalid theme mode', () => {
      const result = validateSettings({
        theme: { mode: 'invalid' as any, accentColor: 'blue' },
      });
      
      expect(result.valid).toBe(false);
      expect(result.errors['theme.mode']).toBeDefined();
    });

    it('should reject invalid log level', () => {
      const result = validateSettings({
        advanced: { enableLogging: true, logLevel: 'invalid' as any, autoUpdate: true },
      });
      
      expect(result.valid).toBe(false);
      expect(result.errors['advanced.logLevel']).toBeDefined();
    });

    it('should validate optional baseUrl', () => {
      const result = validateSettings({
        model: { provider: 'ollama', model: 'test', baseUrl: 'invalid-url' },
      });
      
      expect(result.valid).toBe(false);
      expect(result.errors['model.baseUrl']).toBeDefined();
    });

    it('should accept valid baseUrl', () => {
      const result = validateSettings({
        model: { provider: 'ollama', model: 'test', baseUrl: 'http://localhost:11434' },
      });
      
      expect(result.valid).toBe(true);
    });
  });

  describe('settingsValidation', () => {
    it('should validate provider correctly', () => {
      expect(settingsValidation.model.provider('ollama')).toBe(true);
      expect(settingsValidation.model.provider('openai')).toBe(true);
      expect(settingsValidation.model.provider('anthropic')).toBe(true);
      expect(settingsValidation.model.provider('invalid')).toBe(false);
    });

    it('should validate model name length', () => {
      expect(settingsValidation.model.model('valid-model')).toBe(true);
      expect(settingsValidation.model.model('')).toBe(false);
      expect(settingsValidation.model.model('a'.repeat(101))).toBe(false);
    });

    it('should validate token ranges', () => {
      expect(settingsValidation.tokens.maxTokensPerRequest(4096)).toBe(true);
      expect(settingsValidation.tokens.maxTokensPerRequest(50)).toBe(false);
      expect(settingsValidation.tokens.maxTokensPerRequest(200000)).toBe(false);
    });
  });
});
