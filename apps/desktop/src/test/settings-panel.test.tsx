/**
 * Settings Panel Integration Tests
 * Tests for SettingsPanel component integration
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { SettingsPanel } from '../panels/SettingsPanel';
import { useSettingsStore } from '../store/settings-store';
import { defaultSettings } from '../types/settings';

// Mock fetch
const mockFetch = vi.fn();
global.fetch = mockFetch;

// Mock the store to use real implementation
vi.mock('../store/settings-store', async () => {
  const actual = await vi.importActual('../store/settings-store');
  return {
    ...(actual as any),
    useSettingsStore: (actual as any).useSettingsStore,
  };
});

describe('SettingsPanel Integration', () => {
  beforeEach(() => {
    // Reset store
    useSettingsStore.setState({
      settings: defaultSettings,
      isLoading: false,
      error: null,
      hasUnsavedChanges: false,
    });
    mockFetch.mockClear();
  });

  describe('Panel Rendering', () => {
    it('should render settings panel', () => {
      render(<SettingsPanel />);
      
      expect(screen.getByText('Settings')).toBeInTheDocument();
    });

    it('should render all settings sections', () => {
      render(<SettingsPanel />);
      
      expect(screen.getByText('Model Configuration')).toBeInTheDocument();
      expect(screen.getByText('Token Limits')).toBeInTheDocument();
      expect(screen.getByText('Worker Pool')).toBeInTheDocument();
      expect(screen.getByText('Theme Preferences')).toBeInTheDocument();
      expect(screen.getByText('Advanced Settings')).toBeInTheDocument();
    });

    it('should render model provider dropdown', () => {
      render(<SettingsPanel />);
      
      const providerSelect = screen.getByLabelText('Provider');
      expect(providerSelect).toBeInTheDocument();
      expect(providerSelect).toHaveValue('ollama');
    });

    it('should render model name input', () => {
      render(<SettingsPanel />);
      
      const modelInput = screen.getByLabelText('Model');
      expect(modelInput).toBeInTheDocument();
      expect(modelInput).toHaveValue('qwen2.5-coder:7b');
    });

    it('should render token limit inputs', () => {
      render(<SettingsPanel />);
      
      expect(screen.getByLabelText('Max Tokens per Request')).toBeInTheDocument();
      expect(screen.getByLabelText('Max Context Tokens')).toBeInTheDocument();
      expect(screen.getByLabelText('Token Budget (per session)')).toBeInTheDocument();
    });

    it('should render worker pool inputs', () => {
      render(<SettingsPanel />);
      
      expect(screen.getByLabelText('Min Workers')).toBeInTheDocument();
      expect(screen.getByLabelText('Max Workers')).toBeInTheDocument();
      expect(screen.getByLabelText('Health Check Interval (seconds)')).toBeInTheDocument();
    });

    it('should render theme mode dropdown', () => {
      render(<SettingsPanel />);
      
      const modeSelect = screen.getByLabelText('Theme Mode');
      expect(modeSelect).toBeInTheDocument();
      expect(modeSelect).toHaveValue('dark');
    });

    it('should render advanced toggles', () => {
      render(<SettingsPanel />);
      
      expect(screen.getByText('Enable Logging')).toBeInTheDocument();
      expect(screen.getByText('Auto Update')).toBeInTheDocument();
    });
  });

  describe('Settings Interaction', () => {
    it('should update model provider', () => {
      render(<SettingsPanel />);
      
      const providerSelect = screen.getByLabelText('Provider');
      fireEvent.change(providerSelect, { target: { value: 'openai' } });
      
      expect(providerSelect).toHaveValue('openai');
    });

    it('should update model name', () => {
      render(<SettingsPanel />);
      
      const modelInput = screen.getByLabelText('Model');
      fireEvent.change(modelInput, { target: { value: 'new-model' } });
      
      expect(modelInput).toHaveValue('new-model');
    });

    it('should update token limits', () => {
      render(<SettingsPanel />);
      
      const tokensInput = screen.getByLabelText('Max Tokens per Request');
      fireEvent.change(tokensInput, { target: { value: '8192' } });
      
      expect(tokensInput).toHaveValue('8192');
    });

    it('should update worker count', () => {
      render(<SettingsPanel />);
      
      const minWorkersInput = screen.getByLabelText('Min Workers');
      fireEvent.change(minWorkersInput, { target: { value: '5' } });
      
      expect(minWorkersInput).toHaveValue('5');
    });

    it('should toggle enable logging', () => {
      render(<SettingsPanel />);
      
      const toggle = screen.getByRole('switch', { name: /enable logging/i });
      fireEvent.click(toggle);
      
      // Toggle should change state
      expect(toggle).toBeInTheDocument();
    });

    it('should show unsaved changes indicator', () => {
      render(<SettingsPanel />);
      
      const modelInput = screen.getByLabelText('Model');
      fireEvent.change(modelInput, { target: { value: 'changed' } });
      
      expect(screen.getByText('Unsaved changes')).toBeInTheDocument();
    });
  });

  describe('Save/Reset Functionality', () => {
    it('should disable save button when no changes', () => {
      render(<SettingsPanel />);
      
      const saveButton = screen.getByText('Save Changes');
      expect(saveButton).toBeDisabled();
    });

    it('should enable save button when changes made', () => {
      render(<SettingsPanel />);
      
      const modelInput = screen.getByLabelText('Model');
      fireEvent.change(modelInput, { target: { value: 'changed' } });
      
      const saveButton = screen.getByText('Save Changes');
      expect(saveButton).not.toBeDisabled();
    });

    it('should call saveSettings on save button click', async () => {
      mockFetch.mockResolvedValueOnce({ ok: true });
      
      render(<SettingsPanel />);
      
      const modelInput = screen.getByLabelText('Model');
      fireEvent.change(modelInput, { target: { value: 'new-model' } });
      
      const saveButton = screen.getByText('Save Changes');
      fireEvent.click(saveButton);
      
      await waitFor(() => {
        expect(mockFetch).toHaveBeenCalledWith(
          expect.stringContaining('/api/settings'),
          expect.objectContaining({ method: 'PUT' })
        );
      });
    });

    it('should show import/export button', () => {
      render(<SettingsPanel />);
      
      const importExportButton = screen.getByText('Import/Export');
      expect(importExportButton).toBeInTheDocument();
    });

    it('should toggle import/export section', () => {
      render(<SettingsPanel />);
      
      const importExportButton = screen.getByText('Import/Export');
      fireEvent.click(importExportButton);
      
      expect(screen.getByText('Import/Export Settings')).toBeInTheDocument();
    });
  });

  describe('Conditional Rendering', () => {
    it('should show base URL for Ollama provider', () => {
      render(<SettingsPanel />);
      
      expect(screen.getByLabelText('Base URL')).toBeInTheDocument();
    });

    it('should hide base URL for OpenAI provider', () => {
      render(<SettingsPanel />);
      
      const providerSelect = screen.getByLabelText('Provider');
      fireEvent.change(providerSelect, { target: { value: 'openai' } });
      
      expect(screen.queryByLabelText('Base URL')).not.toBeInTheDocument();
    });

    it('should show API key field for OpenAI provider', () => {
      render(<SettingsPanel />);
      
      const providerSelect = screen.getByLabelText('Provider');
      fireEvent.change(providerSelect, { target: { value: 'openai' } });
      
      expect(screen.getByLabelText('API Key')).toBeInTheDocument();
    });

    it('should show API key field for Anthropic provider', () => {
      render(<SettingsPanel />);
      
      const providerSelect = screen.getByLabelText('Provider');
      fireEvent.change(providerSelect, { target: { value: 'anthropic' } });
      
      expect(screen.getByLabelText('API Key')).toBeInTheDocument();
    });
  });

  describe('Error Handling', () => {
    it('should not show error banner initially', () => {
      render(<SettingsPanel />);
      
      expect(screen.queryByText(/Failed to save settings/i)).not.toBeInTheDocument();
    });

    it('should show error banner on save failure', async () => {
      mockFetch.mockRejectedValueOnce(new Error('Network error'));
      
      render(<SettingsPanel />);
      
      const modelInput = screen.getByLabelText('Model');
      fireEvent.change(modelInput, { target: { value: 'new-model' } });
      
      const saveButton = screen.getByText('Save Changes');
      fireEvent.click(saveButton);
      
      await waitFor(() => {
        expect(screen.getByText(/Failed to save settings/i)).toBeInTheDocument();
      });
    });
  });
});
