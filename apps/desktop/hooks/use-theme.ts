"use client";

import { useEffect, useState, useCallback } from "react";
import { useTheme as useNextTheme } from "next-themes";
import type {
  DesktopPreferences,
  DesktopPreferencesPatch,
} from "@/shared/contracts/desktop";
import { desktopService } from "@/lib/services/desktop-service";

export type ThemeMode = "dark" | "system" | "light";

interface UseThemeReturn {
  /** Current theme mode preference from user settings */
  themeMode: ThemeMode;
  /** The actual resolved theme ("light" or "dark") */
  resolvedTheme: "light" | "dark" | undefined;
  /** Set theme mode and persist to preferences */
  setThemeMode: (mode: ThemeMode) => void;
  /** Toggle between light and dark */
  toggleTheme: () => void;
  /** Loading state - true while fetching initial preferences */
  isLoading: boolean;
}

/**
 * Hook to manage theme preferences integrated with DesktopPreferences.
 *
 * - When themeMode is "system", automatically detects and responds to OS preference changes
 * - Persists theme preference across app restarts via desktopService
 * - Provides both the user preference (themeMode) and actual applied theme (resolvedTheme)
 */
export function useTheme(): UseThemeReturn {
  const [themeMode, setThemeModeState] = useState<ThemeMode>("system");
  const [isLoading, setIsLoading] = useState(true);
  const { theme: resolvedTheme, setTheme: setNextTheme } = useNextTheme();

  // Fetch initial preferences
  useEffect(() => {
    let mounted = true;

    desktopService
      .getPreferences()
      .then((preferences: DesktopPreferences | null) => {
        if (mounted && preferences) {
          setThemeModeState(preferences.themeMode);
          setIsLoading(false);
        } else if (mounted) {
          setIsLoading(false);
        }
      })
      .catch((error: unknown) => {
        console.error("[use-theme] Failed to fetch preferences:", error);
        if (mounted) {
          setIsLoading(false);
        }
      });

    return () => {
      mounted = false;
    };
  }, []);

  // Update next-themes when themeMode changes
  useEffect(() => {
    if (!isLoading) {
      setNextTheme(themeMode);
    }
  }, [themeMode, isLoading, setNextTheme]);

  const setThemeMode = useCallback((mode: ThemeMode) => {
    setThemeModeState(mode);

    // Persist to preferences
    const patch: DesktopPreferencesPatch = {
      themeMode: mode,
    };

    desktopService.updatePreferences(patch).catch((error: unknown) => {
      console.error("Failed to update theme preference:", error);
    });
  }, []);

  const toggleTheme = useCallback(() => {
    const newMode: ThemeMode = themeMode === "dark" ? "light" : "dark";
    setThemeMode(newMode);
  }, [themeMode, setThemeMode]);

  return {
    themeMode,
    resolvedTheme: resolvedTheme as "light" | "dark" | undefined,
    setThemeMode,
    toggleTheme,
    isLoading,
  };
}

/**
 * @deprecated Use useTheme instead. Kept for backwards compatibility.
 */
export const useThemeMode = useTheme;
