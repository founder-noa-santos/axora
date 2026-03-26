"use client";

import type { ReactNode } from "react";
import {
  Conversation,
  ConversationContent,
  ConversationScrollButton,
} from "@/components/ai-elements/conversation";
import type { Message as UiMessage } from "@/shared/contracts/message";
import { ChatMessage } from "./ChatMessage";

export function ChatConversation({
  messages,
  emptyState,
  onRetry,
  onCopy,
  onFeedback,
}: {
  messages: UiMessage[];
  emptyState?: ReactNode;
  onRetry?: () => void;
  onCopy?: (content: string) => void;
  onFeedback?: (feedback: "like" | "dislike") => void;
}) {
  if (messages.length === 0 && emptyState) {
    return (
      <div className="flex flex-1 flex-col overflow-y-auto">{emptyState}</div>
    );
  }

  return (
    <Conversation className="min-h-0 flex-1">
      <ConversationContent>
        {messages.map((m) => (
          <ChatMessage
            key={m.key}
            message={m}
            onRetry={onRetry}
            onCopy={onCopy}
            onFeedback={onFeedback}
          />
        ))}
      </ConversationContent>
      <ConversationScrollButton />
    </Conversation>
  );
}
