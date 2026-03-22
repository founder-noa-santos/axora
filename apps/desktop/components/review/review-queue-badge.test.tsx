import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { ReviewQueueBadge } from "./review-queue-badge";

describe("ReviewQueueBadge", () => {
  it("shows capped count and exposes an accessible label", () => {
    const onClick = vi.fn();
    render(<ReviewQueueBadge count={12} maxDisplay={9} onClick={onClick} />);

    expect(screen.getByRole("button", { name: /12 items pending/i })).toBeInTheDocument();
    expect(screen.getByText("9+")).toBeInTheDocument();
  });

  it("fires onClick when activated", async () => {
    const user = userEvent.setup();
    const onClick = vi.fn();
    render(<ReviewQueueBadge count={2} onClick={onClick} />);

    await user.click(screen.getByRole("button", { name: /2 items pending/i }));
    expect(onClick).toHaveBeenCalledTimes(1);
  });
});
