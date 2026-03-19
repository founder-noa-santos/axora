import {
  Activity,
  Boxes,
  FolderKanban,
  LayoutTemplate,
  TerminalSquare,
} from "lucide-react";

import { workspaceCollections } from "@/lib/mock-shell-data";
import { ICON_METRICS } from "@/lib/ui-metrics";

import { Badge } from "@/components/ui/badge";
import { Panel } from "@/components/ui/panel";
import { ScrollArea } from "@/components/ui/scroll-area";

const icons = [LayoutTemplate, FolderKanban, TerminalSquare, Boxes, Activity];

export function NavigationSidebar() {
  return (
    <Panel
      title="Workspace"
      eyebrow="Navigation"
      className="w-[272px] shrink-0 bg-sidebar/75"
    >
      <ScrollArea className="h-full pr-2">
        <div className="space-y-5">
          <div className="space-y-2">
            {workspaceCollections.map((item, index) => {
              const Icon = icons[index] ?? LayoutTemplate;
              return (
                <button
                  key={item.name}
                  type="button"
                  className="sidebar-item w-full"
                >
                  <span className="flex items-center gap-3">
                    <span
                      className={`flex h-8 w-8 items-center justify-center rounded-[12px] ${
                        item.active ? "bg-white/10 text-foreground" : "bg-black/15"
                      }`}
                    >
                      <Icon
                        size={ICON_METRICS.body}
                        strokeWidth={ICON_METRICS.strokeWidth}
                      />
                    </span>
                    <span className="text-left">
                      <span className="block">{item.name}</span>
                      <span className="block text-[11px] text-muted-foreground">
                        {item.active ? "Current surface" : "Pinned collection"}
                      </span>
                    </span>
                  </span>
                  <span className="text-[12px] text-muted-foreground">{item.count}</span>
                </button>
              );
            })}
          </div>

          <div className="rounded-[18px] border border-white/8 bg-black/14 p-4">
            <Badge variant="success">Ready</Badge>
            <h3 className="mt-3 text-[14px] font-semibold tracking-[-0.01em]">
              Renderer isolation is active
            </h3>
            <p className="mt-2 muted-copy">
              React only sees a typed preload bridge. Future Rust features can land
              behind the same IPC contract without UI churn.
            </p>
          </div>
        </div>
      </ScrollArea>
    </Panel>
  );
}
