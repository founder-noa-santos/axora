import { ArrowUpRight, Binary, Command, Link2 } from "lucide-react";

import { activityFeed, emptyStates } from "@/lib/mock-shell-data";
import { ICON_METRICS } from "@/lib/ui-metrics";
import type {
  AppInfo,
  DesktopPreferences,
  ShellState,
} from "@/shared/contracts/desktop";

import { ReviewQueueDemo } from "@/components/review/review-queue-demo";
import { EmptyState } from "@/components/empty-state";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Panel } from "@/components/ui/panel";
import { Separator } from "@/components/ui/separator";

export function WorkspaceCanvas({
  info,
  shellState,
  preferences,
  onOpenSettings,
}: {
  info: AppInfo | null;
  shellState: ShellState | null;
  preferences: DesktopPreferences | null;
  onOpenSettings: () => void;
}) {
  return (
    <div className="grid min-w-0 flex-1 grid-cols-[minmax(0,1fr)_320px] gap-4">
      <Panel
        title="Mission workspace"
        eyebrow="Primary"
        actions={
          <Button size="sm" variant="primary" onClick={onOpenSettings}>
            Open settings
          </Button>
        }
      >
        <div className="grid gap-4 xl:grid-cols-[minmax(0,1.2fr)_minmax(320px,0.8fr)]">
          <div className="space-y-4">
            <div className="rounded-[18px] border border-white/8 bg-black/18 p-5">
              <div className="flex items-start justify-between gap-4">
                <div>
                  <Badge>Shell</Badge>
                  <h1 className="mt-3 text-[24px] font-semibold tracking-[-0.03em] text-foreground">
                    Premium desktop foundation with a secure typed bridge
                  </h1>
                </div>
                <div className="rounded-[16px] border border-white/8 bg-white/6 px-3 py-2 text-right text-[12px] text-muted-foreground">
                  <div>
                    {info?.platform ?? "darwin"} · {info?.arch ?? "arm64"}
                  </div>
                  <div>{info?.version ?? "0.2.0"} renderer shell</div>
                </div>
              </div>

              <div className="mt-5 grid gap-3 md:grid-cols-3">
                <div className="rounded-[16px] bg-white/5 p-4">
                  <p className="text-[11px] uppercase tracking-[0.12em] text-muted-foreground">
                    Rust bridge
                  </p>
                  <p className="mt-2 text-[14px] font-semibold capitalize">
                    {shellState?.rustBridge.status ?? "planned"}
                  </p>
                  <p className="mt-1 muted-copy">
                    {shellState?.rustBridge.note ??
                      "The main process owns future daemon and crate transport."}
                  </p>
                </div>
                <div className="rounded-[16px] bg-white/5 p-4">
                  <p className="text-[11px] uppercase tracking-[0.12em] text-muted-foreground">
                    Theme mode
                  </p>
                  <p className="mt-2 text-[14px] font-semibold capitalize">
                    {preferences?.themeMode ?? "dark"}
                  </p>
                  <p className="mt-1 muted-copy">
                    Compact, dark-first chrome with restrained vibrancy.
                  </p>
                </div>
                <div className="rounded-[16px] bg-white/5 p-4">
                  <p className="text-[11px] uppercase tracking-[0.12em] text-muted-foreground">
                    IPC surface
                  </p>
                  <p className="mt-2 text-[14px] font-semibold">4 channels</p>
                  <p className="mt-1 muted-copy">
                    App info, shell state, preference read, preference write.
                  </p>
                </div>
              </div>
            </div>

            <ReviewQueueDemo />

            <div className="grid gap-4 lg:grid-cols-2">
              <EmptyState
                icon={<Command size={ICON_METRICS.emphasis} />}
                title={emptyStates[0].title}
                body={emptyStates[0].body}
              />
              <EmptyState
                icon={<Binary size={ICON_METRICS.emphasis} />}
                title={emptyStates[1].title}
                body={emptyStates[1].body}
              />
            </div>
          </div>

          <div className="rounded-[18px] border border-white/8 bg-black/14 p-5">
            <div className="mb-4 flex items-center justify-between">
              <div>
                <p className="text-[11px] uppercase tracking-[0.12em] text-muted-foreground">
                  Command surface
                </p>
                <h3 className="mt-1 text-[15px] font-semibold">
                  Pinned actions
                </h3>
              </div>
              <ArrowUpRight size={ICON_METRICS.body} />
            </div>
            <div className="space-y-3">
              {[
                "Connect Rust daemon capability",
                "Browse indexed repository map",
                "Open mission command palette",
              ].map((item) => (
                <button
                  key={item}
                  type="button"
                  className="flex w-full items-center justify-between rounded-[14px] border border-white/8 bg-white/5 px-4 py-3 text-left text-[13px] hover:bg-white/8"
                >
                  <span>{item}</span>
                  <Link2 size={ICON_METRICS.body} />
                </button>
              ))}
            </div>
          </div>
        </div>
      </Panel>

      <Panel title="Inspector" eyebrow="Status">
        <div className="space-y-4">
          <div className="space-y-3">
            {activityFeed.map((item) => (
              <div key={item.title} className="rounded-[16px] bg-black/14 p-4">
                <div className="flex items-start justify-between gap-3">
                  <div>
                    <p className="text-[13px] font-medium">{item.title}</p>
                    <p className="mt-1 muted-copy">{item.detail}</p>
                  </div>
                  <span className="text-[11px] text-muted-foreground">
                    {item.time}
                  </span>
                </div>
              </div>
            ))}
          </div>

          <Separator />

          <div className="space-y-3">
            <h3 className="text-[13px] font-semibold uppercase tracking-[0.12em] text-muted-foreground">
              Environment
            </h3>
            <dl className="space-y-3 text-[13px]">
              <div className="flex justify-between gap-4">
                <dt className="text-muted-foreground">App</dt>
                <dd>{info?.name ?? "OPENAKTA"}</dd>
              </div>
              <div className="flex justify-between gap-4">
                <dt className="text-muted-foreground">Renderer</dt>
                <dd>Next.js App Router</dd>
              </div>
              <div className="flex justify-between gap-4">
                <dt className="text-muted-foreground">Desktop shell</dt>
                <dd>Electron with preload bridge</dd>
              </div>
            </dl>
          </div>
        </div>
      </Panel>
    </div>
  );
}
