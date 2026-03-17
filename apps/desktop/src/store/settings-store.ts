/**
 * Settings Store
 * 
 * Manages application settings with Zustand, including:
 * - Local storage persistence
 * - Backend API sync
 * - Settings import/export
 */

import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { AppSettings, defaultSettings, validateSettings } from '../types/settings';

interface SettingsState {
  settings: AppSettings;
  isLoading: boolean;
  error: string | null;
  hasUnsavedChanges: boolean;
}

interface SettingsActions {
  loadSettings: () => Promise<void>;
  saveSettings: (settings: Partial<AppSettings>) => Promise<void>;
  updateSetting: <K extends keyof AppSettings>(
    section: K,
    key: keyof AppSettings[K],
    value: AppSettings[K][keyof AppSettings[K]]
  ) => void;
  resetSettings: () => void;
  exportSettings: () => string;
  importSettings: (json: string) => Promise<void>;
  clearError: () => void;
  markAsSaved: () => void;
}

export type SettingsStore = SettingsState & SettingsActions;

// API base URL (configured via environment or default)
const API_BASE_URL = import.meta.env.VITE_API_URL || '/api';

export const useSettingsStore = create<SettingsStore>()(
  persist(
    (set, get) => ({
      settings: defaultSettings,
      isLoading: false,
      error: null,
      hasUnsavedChanges: false,

      loadSettings: async () => {
        set({ isLoading: true, error: null });
        try {
          // Try to load from backend API
          const response = await fetch(`${API_BASE_URL}/settings`);
          if (response.ok) {
            const settings = await response.json();
            set({ settings, isLoading: false });
          } else {
            // Use persisted settings from local storage
            set({ isLoading: false });
          }
        } catch (error) {
          // Use persisted settings from local storage
          set({ 
            error: 'Failed to load settings from backend. Using local settings.',
            isLoading: false 
          });
        }
      },

      saveSettings: async (newSettings) => {
        set({ isLoading: true, error: null });
        try {
          // Validate settings
          const validation = validateSettings(newSettings);
          if (!validation.valid) {
            const errorMessages = Object.values(validation.errors).join(', ');
            throw new Error(`Validation failed: ${errorMessages}`);
          }

          const settings = { ...get().settings, ...newSettings };
          
          // Save to backend API
          const response = await fetch(`${API_BASE_URL}/settings`, {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(settings),
          });
          
          if (!response.ok) {
            throw new Error('Failed to save settings to backend');
          }
          
          set({ settings, isLoading: false, hasUnsavedChanges: false });
        } catch (error) {
          set({ 
            error: error instanceof Error ? error.message : 'Failed to save settings',
            isLoading: false 
          });
        }
      },

      updateSetting: (section, key, value) => {
        set((state) => ({
          settings: {
            ...state.settings,
            [section]: {
              ...state.settings[section],
              [key]: value,
            },
          },
          hasUnsavedChanges: true,
        }));
      },

      resetSettings: () => {
        set({ settings: defaultSettings, hasUnsavedChanges: true });
      },

      exportSettings: () => {
        return JSON.stringify(get().settings, null, 2);
      },

      importSettings: async (json: string) => {
        try {
          const settings = JSON.parse(json) as Partial<AppSettings>;
          const validation = validateSettings(settings);
          if (!validation.valid) {
            const errorMessages = Object.values(validation.errors).join(', ');
            throw new Error(`Invalid settings: ${errorMessages}`);
          }
          await get().saveSettings(settings);
        } catch (error) {
          set({ 
            error: error instanceof Error ? error.message : 'Invalid settings JSON',
          });
          throw error;
        }
      },

      clearError: () => {
        set({ error: null });
      },

      markAsSaved: () => {
        set({ hasUnsavedChanges: false });
      },
    }),
    {
      name: 'axora-settings',
      partialize: (state) => ({ settings: state.settings }),
    }
  )
);
