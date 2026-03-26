import type {
  ExecutionTraceItem,
  Message as AppMessage,
  Thread as AppThread,
} from "@/lib/app-state";
import type { Message as UiMessage, ToolCall } from "@/shared/contracts/message";

export function threadMessagesToContracts(thread: AppThread): UiMessage[] {
  return thread.messages.map((message) =>
    threadMessageToContract(message, thread.executionTrace),
  );
}

export function threadMessageToContract(
  m: AppMessage,
  traceItems: ExecutionTraceItem[] = [],
): UiMessage {
  return {
    key: m.id,
    role: m.role,
    content: m.content,
    isStreaming: false,
    isComplete: true,
    toolCalls: deriveToolCalls(traceItems, m.id),
    versions: [],
    timestamp: new Date(m.timestamp).toISOString(),
  };
}

function deriveToolCalls(
  traceItems: ExecutionTraceItem[],
  messageId: string,
): ToolCall[] {
  const grouped = new Map<string, ExecutionTraceItem[]>();

  for (const item of traceItems) {
    if (item.messageId !== messageId) continue;
    const items = grouped.get(item.toolCallId) ?? [];
    items.push(item);
    grouped.set(item.toolCallId, items);
  }

  return Array.from(grouped.values()).map((items) => {
    const sorted = [...items].sort((a, b) => a.timestamp - b.timestamp);
    const first = sorted[0];
    const latest = sorted[sorted.length - 1];

    return {
      id: first.toolCallId,
      name: first.toolName,
      parameters: first.parameters,
      status: latest.status,
      result: latest.result,
      error: latest.error,
      requiresApproval: latest.requiresApproval,
    };
  });
}
