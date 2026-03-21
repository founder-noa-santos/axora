"use client";

import { Message, MessageContent } from "@/components/ai-elements/message";
import type { Message as UiMessage } from "@/shared/contracts/message";
import { cn } from "@/lib/utils";

export function ChatUserMessage({
  message,
  className,
}: {
  message: UiMessage;
  className?: string;
}) {
  return (
    <Message from="user" className={cn(className)}>
      <MessageContent>
        <p className="whitespace-pre-wrap">{message.content}</p>
      </MessageContent>
    </Message>
  );
}
