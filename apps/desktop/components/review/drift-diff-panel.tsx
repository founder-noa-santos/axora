"use client";

import { useState } from "react";

import { cn } from "@/lib/utils";

import { Button } from "@/components/ui/button";

type Tab = "expected" | "actual";

export function DriftDiffPanel({
  expected,
  actual,
  className,
}: {
  expected: string;
  actual: string;
  className?: string;
}) {
  const [tab, setTab] = useState<Tab>("expected");

  return (
    <div className={cn("space-y-3", className)}>
      <div className="flex md:hidden">
        <div className="inline-flex rounded-full border border-white/10 bg-black/25 p-0.5">
          <Button
            type="button"
            size="sm"
            variant={tab === "expected" ? "primary" : "ghost"}
            className="rounded-full px-3"
            onClick={() => setTab("expected")}
          >
            Expected
          </Button>
          <Button
            type="button"
            size="sm"
            variant={tab === "actual" ? "primary" : "ghost"}
            className="rounded-full px-3"
            onClick={() => setTab("actual")}
          >
            Actual
          </Button>
        </div>
      </div>

      <div className="hidden gap-4 md:grid md:grid-cols-2">
        <DiffColumn label="Expected" body={expected} />
        <DiffColumn label="Actual" body={actual} />
      </div>

      <div className="md:hidden">
        {tab === "expected" ? (
          <DiffColumn label="Expected" body={expected} />
        ) : (
          <DiffColumn label="Actual" body={actual} />
        )}
      </div>
    </div>
  );
}

function DiffColumn({ label, body }: { label: string; body: string }) {
  return (
    <div className="flex min-h-[140px] flex-col rounded-[14px] border border-white/8 bg-black/22">
      <div className="border-b border-white/8 px-3 py-2 text-[11px] font-semibold uppercase tracking-[0.12em] text-muted-foreground">
        {label}
      </div>
      <pre className="max-h-[220px] flex-1 overflow-auto whitespace-pre-wrap break-words p-3 font-mono text-[12px] leading-relaxed text-foreground/90">
        {body || "—"}
      </pre>
    </div>
  );
}
