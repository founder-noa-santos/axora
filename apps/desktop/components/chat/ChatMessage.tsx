"use client";

import type { Message as UiMessage } from "@/shared/contracts/message";
import { ChatAssistantMessage } from "./ChatAssistantMessage";
import { ChatUserMessage } from "./ChatUserMessage";
import { ChatToolCall } from "./ChatToolCall";
import { ChatToolApproval } from "./ChatToolApproval";

export function ChatMessage({
  message,
  className,
  onRetry,
  onCopy,
  onFeedback,
}: {
  message: UiMessage;
  className?: string;
  onRetry?: () => void;
  onCopy?: (content: string) => void;
  onFeedback?: (feedback: "like" | "dislike") => void;
}) {
  switch (message.role) {
    case "user":
      return <ChatUserMessage message={message} className={className} />;
    case "assistant":
      return (
        <ChatAssistantMessage
          message={message}
          className={className}
          onRetry={onRetry}
          onCopy={onCopy}
          onFeedback={onFeedback}
        />
      );
    case "tool":
      return (
        <div className={className}>
          {message.toolCalls?.map((call) => (
            <div key={call.id}>
              <ChatToolCall call={call} />
              <ChatToolApproval call={call} />
            </div>
          ))}
        </div>
      );
    case "system":
      return (
        <div
          className={`text-muted-foreground max-w-[95%] text-xs ${className ?? ""}`}
        >
          {message.content}
        </div>
      );
    default:
      return null;
  }
}
