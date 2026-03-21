"use client";

import { Moon, Sun, Monitor } from "lucide-react";
import { useTheme } from "@/hooks/use-theme";
import { cn } from "@/lib/utils";
import type { ThemeMode } from "@/hooks/use-theme";

interface ThemeModeOption {
  value: ThemeMode;
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  description: string;
}

const themeOptions: ThemeModeOption[] = [
  {
    value: "light",
    label: "Light",
    icon: Sun,
    description: "Always use light theme",
  },
  {
    value: "dark",
    label: "Dark",
    icon: Moon,
    description: "Always use dark theme",
  },
  {
    value: "system",
    label: "System",
    icon: Monitor,
    description: "Match your system preference",
  },
];

export function ThemeModeToggle() {
  const { themeMode, setThemeMode, isLoading } = useTheme();

  if (isLoading) {
    return (
      <div className="flex items-center gap-2 bg-muted/50 rounded-lg p-1 animate-pulse">
        <div className="flex-1 h-8 bg-muted rounded" />
        <div className="flex-1 h-8 bg-muted rounded" />
        <div className="flex-1 h-8 bg-muted rounded" />
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex gap-2 bg-muted/50 rounded-xl p-1">
        {themeOptions.map((option) => {
          const Icon = option.icon;
          const isActive = themeMode === option.value;

          return (
            <button
              key={option.value}
              onClick={() => setThemeMode(option.value)}
              className={cn(
                "flex-1 flex flex-col items-center gap-1.5 px-3 py-2.5 rounded-lg text-[13px] transition-all",
                isActive
                  ? "bg-background text-foreground shadow-sm"
                  : "text-muted-foreground hover:text-foreground hover:bg-muted/30",
              )}
              aria-pressed={isActive}
              aria-label={`Switch to ${option.label} theme`}
            >
              <Icon
                className={cn(
                  "w-4 h-4",
                  isActive ? "opacity-100" : "opacity-60",
                )}
              />
              <span className="font-medium">{option.label}</span>
            </button>
          );
        })}
      </div>

      <div className="px-2">
        <p className="text-[12px] text-muted-foreground">
          {themeOptions.find((o) => o.value === themeMode)?.description}
        </p>
      </div>
    </div>
  );
}

/**
 * Compact version for use in limited space
 */
export function ThemeModeToggleCompact() {
  const { themeMode, toggleTheme, isLoading } = useTheme();

  if (isLoading) {
    return (
      <div className="w-[28px] h-[28px] rounded-lg bg-muted animate-pulse" />
    );
  }

  const Icon =
    themeMode === "light" ? Sun : themeMode === "dark" ? Moon : Monitor;

  return (
    <button
      onClick={toggleTheme}
      className="flex items-center justify-center w-[28px] h-[28px] rounded-lg text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
      aria-label="Toggle theme"
      title={`Current: ${themeMode} (click to toggle)`}
    >
      <Icon className="w-4 h-4" />
    </button>
  );
}
