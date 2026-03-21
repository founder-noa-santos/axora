import type { Message as AppMessage } from "@/lib/app-state";
import type { Message as UiMessage } from "@/shared/contracts/message";

export function threadMessageToContract(m: AppMessage): UiMessage {
  return {
    key: m.id,
    role: m.role,
    content: m.content,
    isStreaming: false,
    isComplete: true,
    versions: [],
    timestamp: new Date(m.timestamp).toISOString(),
  };
}
