import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

export function Panel({
  className,
  title,
  eyebrow,
  actions,
  children,
}: {
  className?: string;
  title: string;
  eyebrow?: string;
  actions?: ReactNode;
  children: ReactNode;
}) {
  return (
    <section
      className={cn("panel-surface flex h-full flex-col p-5", className)}
    >
      <header className="mb-4 flex items-start justify-between gap-4">
        <div className="space-y-1">
          {eyebrow ? (
            <p className="text-[11px] font-semibold uppercase tracking-[0.14em] text-muted-foreground">
              {eyebrow}
            </p>
          ) : null}
          <h2 className="text-[15px] font-semibold tracking-[-0.01em] text-foreground">
            {title}
          </h2>
        </div>
        {actions}
      </header>
      {children}
    </section>
  );
}
