"use client";

import { useEffect, useRef, useState } from "react";

import {
  type ReviewDetail,
  type ReviewQueueItem,
  type ReviewResolutionResponse,
} from "@/shared/contracts/desktop";

import { desktopService } from "@/lib/services/desktop-service";

import {
  ConflictResolverModal,
  type ConflictResolverDetail,
  ReviewQueueBadge,
} from "@/components/review";
import { Button } from "@/components/ui/button";

const POLL_INTERVAL_MS = 5_000;

export function ReviewQueueDemo() {
  const [count, setCount] = useState(0);
  const [items, setItems] = useState<ReviewQueueItem[]>([]);
  const [detail, setDetail] = useState<ConflictResolverDetail | null>(null);
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [loadingDetail, setLoadingDetail] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [banner, setBanner] = useState<string | null>(null);
  const previousCount = useRef<number | null>(null);

  useEffect(() => {
    let cancelled = false;

    const loadQueue = async () => {
      try {
        const [nextCount, list] = await Promise.all([
          desktopService.getPendingReviewCount(),
          desktopService.listPendingReviews(),
        ]);
        if (cancelled) {
          return;
        }
        const safeCount = nextCount ?? 0;
        const safeItems = list?.items ?? [];
        if (
          previousCount.current !== null &&
          safeCount > previousCount.current
        ) {
          setBanner(
            `${safeCount - previousCount.current} new review${safeCount - previousCount.current === 1 ? "" : "s"} waiting for attention.`,
          );
        }
        previousCount.current = safeCount;
        setCount(safeCount);
        setItems(safeItems);
        setError(null);
      } catch (loadError) {
        if (!cancelled) {
          setError(errorMessage(loadError));
        }
      }
    };

    void loadQueue();
    const timer = window.setInterval(() => {
      void loadQueue();
    }, POLL_INTERVAL_MS);

    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, []);

  const openReview = async (reviewId: string) => {
    setOpen(true);
    setLoadingDetail(true);
    try {
      const nextDetail = await desktopService.getReviewDetail(reviewId);
      if (!nextDetail) {
        throw new Error("Review detail is unavailable.");
      }
      setDetail(toModalDetail(nextDetail));
      setError(null);
    } catch (detailError) {
      setError(errorMessage(detailError));
      setOpen(false);
    } finally {
      setLoadingDetail(false);
    }
  };

  const refreshQueue = async () => {
    const [nextCount, list] = await Promise.all([
      desktopService.getPendingReviewCount(),
      desktopService.listPendingReviews(),
    ]);
    setCount(nextCount ?? 0);
    setItems(list?.items ?? []);
  };

  const handleResolve = async (choice: "update_doc" | "update_code") => {
    if (!detail) {
      return;
    }
    setBusy(true);
    try {
      const response = await desktopService.submitReviewResolution({
        reviewId: detail.reviewId,
        choice,
        clientResolutionId:
          globalThis.crypto?.randomUUID?.() ??
          `resolution-${Date.now().toString(36)}`,
      });
      if (!response) {
        throw new Error("Resolution response is unavailable.");
      }
      await refreshQueue();
      setBanner(describeResolution(response));
      if (response.outcome === "ok" || response.outcome === "duplicate") {
        setOpen(false);
        setDetail(null);
      } else {
        setError(describeResolution(response));
      }
    } catch (submitError) {
      setError(errorMessage(submitError));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="rounded-[18px] border border-white/8 bg-black/18 p-5">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <p className="text-[11px] uppercase tracking-[0.12em] text-muted-foreground">
            LivingDocs
          </p>
          <h2 className="mt-1 text-[15px] font-semibold">Review queue (Plan 6)</h2>
          <p className="mt-1 text-[12px] text-muted-foreground">
            Live badge, list, and resolver flow over the daemon review gRPC API.
          </p>
        </div>
        <ReviewQueueBadge
          count={count}
          onClick={() => {
            if (items[0]) {
              void openReview(items[0].reviewId);
            }
          }}
        />
      </div>

      {banner ? (
        <div className="mt-4 flex items-center justify-between gap-3 rounded-[14px] border border-amber-400/20 bg-amber-500/10 px-3 py-2 text-[12px] text-amber-100">
          <span>{banner}</span>
          <Button size="sm" variant="ghost" onClick={() => setBanner(null)}>
            Dismiss
          </Button>
        </div>
      ) : null}

      {error ? (
        <div className="mt-4 rounded-[14px] border border-rose-400/20 bg-rose-500/10 px-3 py-2 text-[12px] text-rose-100">
          {error}
        </div>
      ) : null}

      <div className="mt-4 space-y-3">
        {items.length === 0 ? (
          <div className="rounded-[14px] border border-white/8 bg-white/4 px-4 py-5 text-[13px] text-muted-foreground">
            {count === 0
              ? "No pending LivingDocs reviews."
              : "Loading pending reviews…"}
          </div>
        ) : (
          items.map((item) => (
            <button
              key={item.reviewId}
              type="button"
              onClick={() => void openReview(item.reviewId)}
              className="w-full rounded-[14px] border border-white/8 bg-white/4 px-4 py-3 text-left transition hover:bg-white/8"
            >
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0">
                  <div className="truncate text-[13px] font-medium text-foreground">
                    {item.primaryDocPath || item.reportId}
                  </div>
                  <div className="mt-1 truncate text-[12px] text-muted-foreground">
                    {item.summary ?? item.reportId}
                  </div>
                </div>
                <div className="shrink-0 text-right text-[11px] text-muted-foreground">
                  <div>{item.highestSeverity ?? "pending"}</div>
                  <div>{item.confidenceScore.toFixed(2)}</div>
                </div>
              </div>
            </button>
          ))
        )}
      </div>

      <ConflictResolverModal
        open={open}
        onOpenChange={setOpen}
        detail={loadingDetail ? null : detail}
        onCancel={() => {
          setOpen(false);
          setDetail(null);
        }}
        resolutionInFlight={busy}
        onResolve={handleResolve}
      />
    </div>
  );
}

function toModalDetail(detail: ReviewDetail): ConflictResolverDetail {
  return {
    reviewId: detail.reviewId,
    reportId: detail.reportId,
    workspaceRoot: detail.workspaceRoot,
    createdAtMs: detail.createdAtMs,
    primaryDocPath: detail.primaryDocPath,
    highestSeverity: detail.highestSeverity,
    summary: detail.summary,
    breakdownJson: detail.breakdownJson,
    flags: detail.flags.map((flag) => ({
      fingerprint: flag.fingerprint,
      domain: flag.domain,
      kind: flag.kind,
      severity: flag.severity,
      docPath: flag.docPath,
      codePath: flag.codePath,
      symbolName: flag.symbolName,
      ruleIds: flag.ruleIds,
      message: flag.message,
      expectedExcerpt: flag.expectedExcerpt,
      actualExcerpt: flag.actualExcerpt,
    })),
  };
}

function describeResolution(response: ReviewResolutionResponse) {
  switch (response.outcome) {
    case "ok":
      return response.patchReceiptId
        ? `Code update applied with receipt ${response.patchReceiptId}.`
        : `Documentation update recorded as ${response.toonChangelogEntryId ?? response.serverResolutionId}.`;
    case "duplicate":
      return `Resolution ${response.serverResolutionId} was already processed.`;
    case "conflict":
      return "This review is no longer pending.";
    case "rejected":
      return "The daemon rejected this resolution request.";
    case "internal_error":
      return "The daemon could not complete the follow-up work.";
    default:
      return "The daemon returned an unknown resolution state.";
  }
}

function errorMessage(error: unknown) {
  if (error instanceof Error) {
    return error.message;
  }
  return "The desktop shell could not reach the daemon.";
}
