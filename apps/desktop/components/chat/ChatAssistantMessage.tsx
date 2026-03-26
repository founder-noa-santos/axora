"use client";

import {
  Message,
  MessageContent,
  MessageResponse,
  MessageActions,
  MessageAction,
  MessageToolbar,
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
import {
  CopyIcon,
  RotateCcwIcon,
  ThumbsDownIcon,
  ThumbsUpIcon,
} from "lucide-react";

export function ChatAssistantMessage({
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
  const { reasoning, isStreaming, isComplete } = message;
  const reasoningBlocksBody =
    reasoning && reasoning.isStreaming && !reasoning.isComplete;
  const showMessageBody = !!message.content && !reasoningBlocksBody;
  const reasoningDurationSec = reasoning
    ? Math.max(1, Math.ceil(reasoning.durationMs / 1000))
    : undefined;

  const handleCopy = () => {
    if (message.content) {
      navigator.clipboard.writeText(message.content);
      onCopy?.(message.content);
    }
  };

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
            <MessageResponse parseIncompleteMarkdown={!isComplete}>
              {message.content}
            </MessageResponse>
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

      {!isStreaming && message.content && (
        <MessageToolbar>
          <MessageActions>
            {onRetry && (
              <MessageAction
                onClick={onRetry}
                label="Retry"
                tooltip="Regenerate response"
                variant="ghost"
                size="icon-sm"
              >
                <RotateCcwIcon className="size-3" />
              </MessageAction>
            )}
            <MessageAction
              onClick={handleCopy}
              label="Copy"
              tooltip="Copy to clipboard"
              variant="ghost"
              size="icon-sm"
            >
              <CopyIcon className="size-3" />
            </MessageAction>
            {onFeedback && (
              <>
                <MessageAction
                  onClick={() => onFeedback("like")}
                  label="Like"
                  tooltip="Helpful response"
                  variant="ghost"
                  size="icon-sm"
                >
                  <ThumbsUpIcon className="size-3" />
                </MessageAction>
                <MessageAction
                  onClick={() => onFeedback("dislike")}
                  label="Dislike"
                  tooltip="Not helpful"
                  variant="ghost"
                  size="icon-sm"
                >
                  <ThumbsDownIcon className="size-3" />
                </MessageAction>
              </>
            )}
          </MessageActions>
        </MessageToolbar>
      )}
    </Message>
  );
}
