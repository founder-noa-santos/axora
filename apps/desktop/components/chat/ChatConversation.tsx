"use client";

import type { ReactNode } from "react";
import {
  Conversation,
  ConversationContent,
} from "@/components/ai-elements/conversation";
import type { Message as UiMessage } from "@/shared/contracts/message";
import { ChatMessage } from "./ChatMessage";

export function ChatConversation({
  messages,
  emptyState,
}: {
  messages: UiMessage[];
  emptyState?: ReactNode;
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
          <ChatMessage key={m.key} message={m} />
        ))}
      </ConversationContent>
    </Conversation>
  );
}
