import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { ReviewQueueDemo } from "./review-queue-demo";

const { desktopService } = vi.hoisted(() => ({
  desktopService: {
    getPendingReviewCount: vi.fn(() => Promise.resolve(1)),
    listPendingReviews: vi.fn(() =>
      Promise.resolve({
        items: [
          {
            reviewId: "rev-1",
            reportId: "rpt-1",
            workspaceRoot: "/workspace",
            createdAtMs: 1_700_000_000_000,
            confidenceScore: 0.41,
            primaryDocPath: "akta-docs/03-business-logic/rules.md",
            highestSeverity: "warning",
            summary: "Mismatch for alpha.",
          },
        ],
        totalPending: 1,
      }),
    ),
    getReviewDetail: vi.fn(() =>
      Promise.resolve({
        reviewId: "rev-1",
        reportId: "rpt-1",
        workspaceRoot: "/workspace",
        createdAtMs: 1_700_000_000_000,
        confidenceScore: 0.41,
        primaryDocPath: "akta-docs/03-business-logic/rules.md",
        highestSeverity: "warning",
        summary: "Mismatch for alpha.",
        breakdownJson: "{}",
        confidenceAuditActionId: "audit-1",
        flags: [
          {
            fingerprint: "fp-1",
            domain: "api_surface",
            kind: "missing_symbol",
            severity: "warning",
            docPath: "akta-docs/03-business-logic/rules.md",
            codePath: "src/lib/rules.ts",
            symbolName: "alpha",
            ruleIds: ["BR-9"],
            message: "alpha is missing",
            expectedExcerpt: "Rule BR-9",
            actualExcerpt: "export function alpha() {}",
          },
        ],
      }),
    ),
    submitReviewResolution: vi.fn(() =>
      Promise.resolve({
        serverResolutionId: "srv-1",
        outcome: "ok",
        toonChangelogEntryId: "toon-1",
      }),
    ),
  },
}));

vi.mock("@/lib/services/desktop-service", () => ({
  desktopService,
}));

describe("ReviewQueueDemo", () => {
  it("loads review items and submits a resolution", async () => {
    const user = userEvent.setup();
    render(<ReviewQueueDemo />);

    expect(
      await screen.findByText("akta-docs/03-business-logic/rules.md"),
    ).toBeInTheDocument();
    expect(screen.getByText("Mismatch for alpha.")).toBeInTheDocument();

    await user.click(
      screen.getByRole("button", {
        name: /akta-docs\/03-business-logic\/rules\.md/i,
      }),
    );

    expect(
      await screen.findByText("Resolve documentation drift"),
    ).toBeInTheDocument();
    await user.click(
      screen.getByRole("button", { name: /Code is correct/i }),
    );
    await user.click(screen.getByRole("button", { name: "Run" }));

    await waitFor(() => {
      expect(desktopService.submitReviewResolution).toHaveBeenCalledWith(
        expect.objectContaining({
          reviewId: "rev-1",
          choice: "update_doc",
        }),
      );
    });
    expect(
      await screen.findByText(/Documentation update recorded as toon-1/i),
    ).toBeInTheDocument();
  });
});
