"use client";

import { Command, PanelLeftOpen, Search, Settings2, Sparkles } from "lucide-react";

import { ICON_METRICS } from "@/lib/ui-metrics";

import { Button } from "@/components/ui/button";

export function TopChrome({
  onOpenSettings,
}: {
  onOpenSettings: () => void;
}) {
  return (
    <header className="app-drag-region flex h-14 items-center justify-between gap-4 rounded-[18px] border border-white/8 bg-black/20 px-4 backdrop-blur-xl">
      <div className="flex items-center gap-3">
        <div className="flex items-center gap-2 rounded-full border border-white/8 bg-white/6 px-3 py-1.5 text-[11px] font-semibold uppercase tracking-[0.12em] text-muted-foreground">
          <Sparkles
            size={ICON_METRICS.body}
            strokeWidth={ICON_METRICS.strokeWidth}
          />
          AXORA Desktop
        </div>
        <div className="hidden text-[13px] text-muted-foreground md:block">
          Mission workspace tuned for macOS-first operation surfaces
        </div>
      </div>

      <div className="flex items-center gap-2">
        <div
          data-no-drag="true"
          className="hidden min-w-[320px] items-center gap-2 rounded-[12px] border border-white/8 bg-white/6 px-3 py-2 text-[13px] text-muted-foreground lg:flex"
        >
          <Search size={ICON_METRICS.body} strokeWidth={ICON_METRICS.strokeWidth} />
          Search code, runs, settings, and future Rust actions
          <span className="ml-auto flex items-center gap-1 rounded-md border border-white/8 bg-black/20 px-2 py-0.5 text-[11px]">
            <Command size={12} />
            K
          </span>
        </div>
        <Button size="icon" variant="ghost" aria-label="Toggle navigation">
          <PanelLeftOpen
            size={ICON_METRICS.toolbar}
            strokeWidth={ICON_METRICS.strokeWidth}
          />
        </Button>
        <Button
          size="icon"
          variant="ghost"
          aria-label="Open settings"
          onClick={onOpenSettings}
        >
          <Settings2
            size={ICON_METRICS.toolbar}
            strokeWidth={ICON_METRICS.strokeWidth}
          />
        </Button>
      </div>
    </header>
  );
}
