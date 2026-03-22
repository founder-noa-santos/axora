"use client";

import { cn } from "@/lib/utils";

import { Badge } from "@/components/ui/badge";

export function ReviewQueueBadge({
  count,
  maxDisplay = 9,
  onClick,
  className,
}: {
  count: number;
  maxDisplay?: number;
  onClick?: () => void;
  className?: string;
}) {
  const label = count > maxDisplay ? `${maxDisplay}+` : String(count);
  const ariaLabel = `Open review queue, ${count} item${count === 1 ? "" : "s"} pending`;

  return (
    <button
      type="button"
      onClick={onClick}
      aria-label={ariaLabel}
      className={cn(
        "inline-flex items-center gap-2 rounded-full border border-white/10 bg-white/6 px-3 py-1.5 text-[12px] font-medium text-foreground transition hover:bg-white/10 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-white/40",
        className,
      )}
    >
      <span className="text-[11px] font-semibold uppercase tracking-[0.12em] text-muted-foreground">
        Reviews
      </span>
      <Badge variant={count > 0 ? "warning" : "default"}>{label}</Badge>
    </button>
  );
}
