"use client";

import {
  Tool,
  ToolContent,
  ToolHeader,
  ToolInput,
  ToolOutput,
} from "@/components/ai-elements/tool";
import type { DynamicToolUIPart } from "ai";
import type { ToolCall } from "@/shared/contracts/message";

/** Maps contract tool status to AI Elements / AI SDK-style tool state. */
function toolCallToUiState(
  status: ToolCall["status"],
  requiresApproval: boolean,
): DynamicToolUIPart["state"] {
  if (requiresApproval && status === "pending") {
    return "approval-requested";
  }
  switch (status) {
    case "pending":
      return "input-streaming";
    case "running":
    case "input-available":
      return "input-available";
    case "complete":
      return "output-available";
    case "error":
      return "output-error";
    default:
      return "input-available";
  }
}

export function ChatToolCall({ call }: { call: ToolCall }) {
  const state = toolCallToUiState(call.status, call.requiresApproval);

  return (
    <Tool
      defaultOpen={state === "output-error" || state === "approval-requested"}
    >
      <ToolHeader
        type="dynamic-tool"
        state={state}
        title={call.name}
        toolName={call.name}
      />
      <ToolContent>
        <ToolInput input={call.parameters} />
        <ToolOutput
          output={call.result ?? undefined}
          errorText={call.error ?? undefined}
        />
      </ToolContent>
    </Tool>
  );
}
