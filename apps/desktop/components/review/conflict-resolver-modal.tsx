"use client";

import { useEffect, useMemo, useState } from "react";

import { cn } from "@/lib/utils";

import { DriftDiffPanel } from "@/components/review/drift-diff-panel";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Spinner } from "@/components/ui/spinner";

export type SsotChoiceUi = "unset" | "update_doc" | "update_code";

export type ConflictResolverFlag = {
  fingerprint: string;
  domain: string;
  kind: string;
  severity: string;
  docPath: string;
  codePath?: string;
  symbolName?: string;
  ruleIds: string[];
  message: string;
  expectedExcerpt: string;
  actualExcerpt: string;
};

export type ConflictResolverDetail = {
  reviewId: string;
  reportId: string;
  workspaceRoot: string;
  createdAtMs: number;
  primaryDocPath: string;
  highestSeverity?: string;
  summary?: string;
  flags: ConflictResolverFlag[];
  breakdownJson: string;
};

function formatTime(ms: number) {
  try {
    return new Intl.DateTimeFormat(undefined, {
      dateStyle: "medium",
      timeStyle: "short",
    }).format(new Date(ms));
  } catch {
    return String(ms);
  }
}

export function ConflictResolverModal({
  open,
  onOpenChange,
  detail,
  onCancel,
  onResolve,
  resolutionInFlight,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  detail: ConflictResolverDetail | null;
  onCancel: () => void;
  onResolve: (choice: "update_doc" | "update_code") => void | Promise<void>;
  resolutionInFlight?: boolean;
}) {
  const [choice, setChoice] = useState<SsotChoiceUi>("unset");

  useEffect(() => {
    if (!open) {
      setChoice("unset");
    }
  }, [open, detail?.reviewId]);

  const firstFlag = detail?.flags[0];
  const expected = firstFlag?.expectedExcerpt ?? "";
  const actual = firstFlag?.actualExcerpt ?? firstFlag?.message ?? "";

  const canConfirm = choice === "update_doc" || choice === "update_code";

  const headerLine = useMemo(() => {
    if (!detail) {
      return "";
    }
    return `${detail.primaryDocPath || "—"} · ${detail.highestSeverity ?? "severity unknown"}`;
  }, [detail]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-3xl border-white/10 bg-panel-elevated">
        <DialogHeader>
          <DialogTitle>Resolve documentation drift</DialogTitle>
          <DialogDescription>
            Choose which side is authoritative before applying an automated follow-up.
          </DialogDescription>
        </DialogHeader>

        {!detail ? (
          <div className="flex items-center gap-2 py-8 text-[13px] text-muted-foreground">
            <Spinner className="size-4" />
            Loading review…
          </div>
        ) : (
          <div className="space-y-5">
            <div className="rounded-[14px] border border-white/8 bg-black/20 px-4 py-3 text-[13px]">
              <div className="grid gap-1 font-mono text-[12px] text-muted-foreground">
                <div>
                  <span className="text-foreground/80">review_id</span> {detail.reviewId}
                </div>
                <div>
                  <span className="text-foreground/80">report_id</span> {detail.reportId}
                </div>
                <div>
                  <span className="text-foreground/80">workspace</span> {detail.workspaceRoot}
                </div>
                <div>
                  <span className="text-foreground/80">created</span>{" "}
                  {formatTime(detail.createdAtMs)}
                </div>
              </div>
              <p className="mt-2 text-[13px] text-foreground/90">{headerLine}</p>
              {detail.summary ? (
                <p className="mt-1 text-[12px] text-muted-foreground">{detail.summary}</p>
              ) : null}
            </div>

            <div className="grid gap-3 sm:grid-cols-2">
              <SsotCard
                title="Code is correct"
                subtitle="Update documentation to match implementation."
                selected={choice === "update_doc"}
                onSelect={() => setChoice("update_doc")}
              />
              <SsotCard
                title="Documentation is correct"
                subtitle="Generate a patch so code matches the documented rules."
                selected={choice === "update_code"}
                onSelect={() => setChoice("update_code")}
              />
            </div>

            <DriftDiffPanel expected={expected} actual={actual} />

            <div className="flex flex-wrap items-center justify-end gap-2 border-t border-white/8 pt-4">
              <Button
                type="button"
                variant="ghost"
                onClick={onCancel}
                disabled={resolutionInFlight}
              >
                Cancel
              </Button>
              <Button
                type="button"
                variant="primary"
                disabled={!canConfirm || resolutionInFlight}
                onClick={() => {
                  if (choice === "update_doc" || choice === "update_code") {
                    void onResolve(choice);
                  }
                }}
              >
                {resolutionInFlight ? (
                  <span className="inline-flex items-center gap-2">
                    <Spinner className="size-4" />
                    Running…
                  </span>
                ) : (
                  "Run"
                )}
              </Button>
            </div>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}

function SsotCard({
  title,
  subtitle,
  selected,
  onSelect,
}: {
  title: string;
  subtitle: string;
  selected: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onSelect}
      className={cn(
        "rounded-[16px] border px-4 py-4 text-left transition focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-white/40",
        selected
          ? "border-amber-400/50 bg-amber-500/10"
          : "border-white/10 bg-white/4 hover:bg-white/8",
      )}
    >
      <div className="text-[14px] font-semibold text-foreground">{title}</div>
      <div className="mt-1 text-[12px] leading-snug text-muted-foreground">{subtitle}</div>
    </button>
  );
}
