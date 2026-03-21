"use client";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import type { ToolCall } from "@/shared/contracts/message";
import { cn } from "@/lib/utils";

/**
 * Tool approval strip. AI Elements `Confirmation` is coupled to `ToolUIPart`
 * approval objects; we use shadcn Alert + actions here per implementation spec fallback.
 */
export function ChatToolApproval({
  call,
  className,
  onApprove,
  onDeny,
}: {
  call: ToolCall;
  className?: string;
  onApprove?: (id: string) => void;
  onDeny?: (id: string) => void;
}) {
  if (!call.requiresApproval || call.status !== "pending") {
    return null;
  }

  return (
    <Alert className={cn("mt-2 border-amber-500/40 bg-amber-500/5", className)}>
      <AlertTitle className="text-sm">Approve tool call?</AlertTitle>
      <AlertDescription className="text-xs text-muted-foreground">
        <span className="font-medium text-foreground">{call.name}</span> wants
        to run with the shown parameters.
      </AlertDescription>
      <div className="mt-3 flex justify-end gap-2">
        <Button
          type="button"
          size="sm"
          variant="outline"
          onClick={() => onDeny?.(call.id)}
        >
          Deny
        </Button>
        <Button
          type="button"
          size="sm"
          variant="primary"
          onClick={() => onApprove?.(call.id)}
        >
          Approve
        </Button>
      </div>
    </Alert>
  );
}
