"use client";

import {
  Message,
  MessageContent,
  MessageResponse,
} from "@/components/ai-elements/message";
import {
  Reasoning,
  ReasoningContent,
  ReasoningTrigger,
} from "@/components/ai-elements/reasoning";
import type { Message as UiMessage } from "@/shared/contracts/message";
import { cn } from "@/lib/utils";
import { ChatCheckpoint } from "./ChatCheckpoint";
import { ChatToolCall } from "./ChatToolCall";
import { ChatToolApproval } from "./ChatToolApproval";

export function ChatAssistantMessage({
  message,
  className,
}: {
  message: UiMessage;
  className?: string;
}) {
  const { reasoning, isStreaming, isComplete } = message;
  const reasoningBlocksBody =
    reasoning && reasoning.isStreaming && !reasoning.isComplete;
  const showMessageBody = !!message.content && !reasoningBlocksBody;
  const reasoningDurationSec = reasoning
    ? Math.max(1, Math.ceil(reasoning.durationMs / 1000))
    : undefined;

  return (
    <Message from="assistant" className={cn(className)}>
      <MessageContent>
        {reasoning && (
          <Reasoning
            className="mb-2"
            duration={reasoning.isComplete ? reasoningDurationSec : undefined}
            isStreaming={!!reasoning.isStreaming && !reasoning.isComplete}
          >
            <ReasoningTrigger />
            <ReasoningContent>{reasoning.content}</ReasoningContent>
          </Reasoning>
        )}

        {showMessageBody ? (
          <>
            <MessageResponse>{message.content}</MessageResponse>
            {!isStreaming && isComplete && message.checkpointId ? (
              <ChatCheckpoint checkpointId={message.checkpointId} />
            ) : null}
          </>
        ) : null}

        {message.toolCalls?.map((call) => (
          <div key={call.id} className="mt-2 w-full">
            <ChatToolCall call={call} />
            <ChatToolApproval call={call} />
          </div>
        ))}
      </MessageContent>
    </Message>
  );
}
