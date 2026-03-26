/**
 * AI SDK Elements - Message Components Usage Examples
 * 
 * This file demonstrates how to use the complete AI SDK Elements message components
 * based on: https://elements.ai-sdk.dev/components/message
 * 
 * IMPORTANT: Make sure to add the following to your globals.css:
 * @source "../node_modules/streamdown/dist/*.js";
 */

"use client";

import { useState } from "react";
import {
  Message,
  MessageActions,
  MessageAction,
  MessageContent,
  MessageResponse,
  MessageToolbar,
  MessageBranch,
  MessageBranchContent,
  MessageBranchSelector,
  MessageBranchPrevious,
  MessageBranchNext,
  MessageBranchPage,
} from "@/components/ai-elements/message";
import {
  Conversation,
  ConversationContent,
  ConversationScrollButton,
} from "@/components/ai-elements/conversation";
import { RefreshCcwIcon, CopyIcon, ThumbsUpIcon, ThumbsDownIcon, ShareIcon } from "lucide-react";

// ============================================================================
// EXAMPLE 1: Basic Chat UI with Actions (Copy, Retry)
// ============================================================================

export function BasicChatExample() {
  const [messages, setMessages] = useState([
    { id: "1", role: "user" as const, content: "Hello!" },
    { id: "2", role: "assistant" as const, content: "Hi there! How can I help you today?" },
  ]);

  const handleSendMessage = (content: string) => {
    setMessages((prev) => [...prev, { id: Date.now().toString(), role: "user" as const, content }]);
    // Simulate AI response
    setTimeout(() => {
      setMessages((prev) => [
        ...prev,
        { id: Date.now().toString(), role: "assistant" as const, content: "Thanks for your message!" },
      ]);
    }, 1000);
  };

  const handleCopy = (content: string) => {
    console.log("Copied:", content);
  };

  const handleRetry = () => {
    console.log("Retrying...");
  };

  const handleFeedback = (feedback: "like" | "dislike") => {
    console.log("Feedback:", feedback);
  };

  return (
    <div className="max-w-4xl mx-auto p-6 size-full rounded-lg border h-[600px]">
      <div className="flex flex-col h-full">
        <Conversation>
          <ConversationContent>
            {messages.map((message) => (
              <Message key={message.id} from={message.role}>
                <MessageContent>
                  <MessageResponse parseIncompleteMarkdown={false}>
                    {message.content}
                  </MessageResponse>
                </MessageContent>
                
                {message.role === "assistant" && (
                  <MessageToolbar>
                    <MessageActions>
                      <MessageAction
                        onClick={handleRetry}
                        label="Retry"
                        tooltip="Regenerate response"
                      >
                        <RefreshCcwIcon className="size-3" />
                      </MessageAction>
                      
                      <MessageAction
                        onClick={() => handleCopy(message.content)}
                        label="Copy"
                        tooltip="Copy to clipboard"
                      >
                        <CopyIcon className="size-3" />
                      </MessageAction>
                      
                      <MessageAction
                        onClick={() => handleFeedback("like")}
                        label="Like"
                        tooltip="Helpful response"
                      >
                        <ThumbsUpIcon className="size-3" />
                      </MessageAction>
                      
                      <MessageAction
                        onClick={() => handleFeedback("dislike")}
                        label="Dislike"
                        tooltip="Not helpful"
                      >
                        <ThumbsDownIcon className="size-3" />
                      </MessageAction>
                      
                      <MessageAction
                        onClick={() => console.log("Share")}
                        label="Share"
                        tooltip="Share this response"
                      >
                        <ShareIcon className="size-3" />
                      </MessageAction>
                    </MessageActions>
                  </MessageToolbar>
                )}
              </Message>
            ))}
          </ConversationContent>
          <ConversationScrollButton />
        </Conversation>
      </div>
    </div>
  );
}

// ============================================================================
// EXAMPLE 2: Message Branching (Multiple Response Versions)
// ============================================================================

export function BranchingChatExample() {
  const [messages] = useState([
    { id: "1", role: "user" as const, content: "Explain React hooks" },
    { 
      id: "2", 
      role: "assistant" as const, 
      branches: [
        "React Hooks are functions that let you use state and other React features in functional components.",
        "Hooks are a powerful feature in React that enable state management and lifecycle methods in function components.",
        "With React Hooks, you can write cleaner, more maintainable code by using built-in functions like useState and useEffect.",
      ]
    },
  ]);

  return (
    <div className="max-w-4xl mx-auto p-6 size-full rounded-lg border h-[600px]">
      <div className="flex flex-col h-full">
        <Conversation>
          <ConversationContent>
            {messages.map((message, messageIndex) => (
              <Message key={message.id} from={message.role}>
                <MessageContent>
                  {message.role === "assistant" && "branches" in message ? (
                    <MessageBranch defaultBranch={0}>
                      <MessageBranchContent>
                        {message.branches.map((branch, i) => (
                          <MessageResponse key={i}>
                            {branch}
                          </MessageResponse>
                        ))}
                      </MessageBranchContent>
                      
                      <MessageToolbar>
                        <MessageBranchSelector>
                          <MessageBranchPrevious />
                          <MessageBranchPage />
                          <MessageBranchNext />
                        </MessageBranchSelector>
                        
                        <MessageActions>
                          <MessageAction
                            onClick={() => console.log("Retry branching")}
                            label="Retry"
                          >
                            <RefreshCcwIcon className="size-3" />
                          </MessageAction>
                          <MessageAction
                            onClick={() => navigator.clipboard.writeText(message.branches[0])}
                            label="Copy"
                          >
                            <CopyIcon className="size-3" />
                          </MessageAction>
                        </MessageActions>
                      </MessageToolbar>
                    </MessageBranch>
                  ) : (
                    <MessageResponse>{message.content}</MessageResponse>
                  )}
                </MessageContent>
              </Message>
            ))}
          </ConversationContent>
          <ConversationScrollButton />
        </Conversation>
      </div>
    </div>
  );
}

// ============================================================================
// EXAMPLE 3: Streaming Response with Actions
// ============================================================================

export function StreamingChatExample() {
  const [isStreaming, setIsStreaming] = useState(false);
  const [content, setContent] = useState("");

  const startStreaming = () => {
    setIsStreaming(true);
    setContent("");
    
    const fullText = "This is a streaming response that appears character by character...";
    let index = 0;
    
    const interval = setInterval(() => {
      if (index < fullText.length) {
        setContent((prev) => prev + fullText[index]);
        index++;
      } else {
        clearInterval(interval);
        setIsStreaming(false);
      }
    }, 50);
  };

  return (
    <div className="max-w-4xl mx-auto p-6 size-full rounded-lg border h-[600px]">
      <div className="flex flex-col h-full gap-4">
        <button 
          onClick={startStreaming}
          disabled={isStreaming}
          className="px-4 py-2 bg-primary text-primary-foreground rounded-md hover:bg-primary/90 disabled:opacity-50"
        >
          {isStreaming ? "Streaming..." : "Start Streaming"}
        </button>

        <Conversation>
          <ConversationContent>
            <Message from="assistant">
              <MessageContent>
                <MessageResponse parseIncompleteMarkdown={isStreaming}>
                  {content}
                </MessageResponse>
              </MessageContent>
              
              {!isStreaming && content && (
                <MessageToolbar>
                  <MessageActions>
                    <MessageAction
                      onClick={startStreaming}
                      label="Retry"
                      tooltip="Regenerate response"
                    >
                      <RefreshCcwIcon className="size-3" />
                    </MessageAction>
                    
                    <MessageAction
                      onClick={() => navigator.clipboard.writeText(content)}
                      label="Copy"
                      tooltip="Copy to clipboard"
                    >
                      <CopyIcon className="size-3" />
                    </MessageAction>
                  </MessageActions>
                </MessageToolbar>
              )}
            </Message>
          </ConversationContent>
          <ConversationScrollButton />
        </Conversation>
      </div>
    </div>
  );
}

// ============================================================================
// EXAMPLE 4: Markdown Rendering with Code Blocks
// ============================================================================

export function MarkdownChatExample() {
  const markdownContent = `
# Welcome to AI SDK Elements!

This is a **bold** text and this is *italic*.

## Code Example

\`\`\`typescript
function greet(name: string) {
  return \`Hello, \${name}!\`;
}

console.log(greet("World"));
\`\`\`

## Math Equations

Inline math: $E = mc^2$

Block math:
$$
\\int_{-\\infty}^{\\infty} e^{-x^2} dx = \\sqrt{\\pi}
$$

## Task List

- [x] Install AI SDK Elements
- [x] Add Streamdown CSS source
- [ ] Build amazing chat UI
- [ ] Deploy to production

## Table

| Feature | Status | Priority |
|---------|--------|----------|
| Chat UI | ✅ Done | High |
| Actions | ✅ Done | High |
| Branching |  WIP | Medium |
`;

  return (
    <div className="max-w-4xl mx-auto p-6 size-full rounded-lg border h-[600px]">
      <Conversation>
        <ConversationContent>
          <Message from="assistant">
            <MessageContent>
              <MessageResponse>{markdownContent}</MessageResponse>
            </MessageContent>
            
            <MessageToolbar>
              <MessageActions>
                <MessageAction
                  onClick={() => navigator.clipboard.writeText(markdownContent)}
                  label="Copy"
                  tooltip="Copy markdown"
                >
                  <CopyIcon className="size-3" />
                </MessageAction>
              </MessageActions>
            </MessageToolbar>
          </Message>
        </ConversationContent>
        <ConversationScrollButton />
      </Conversation>
    </div>
  );
}

// ============================================================================
// USAGE IN ACTACODE DESKTOP
// ============================================================================

/**
 * To integrate these components in the Aktacode Desktop app:
 * 
 * 1. The basic structure is already in place in:
 *    - components/chat/ChatView.tsx
 *    - components/chat/ChatConversation.tsx
 *    - components/chat/ChatMessage.tsx
 *    - components/chat/ChatAssistantMessage.tsx
 *    - components/chat/ChatUserMessage.tsx
 * 
 * 2. Actions (copy, retry, feedback) have been added to ChatAssistantMessage
 * 
 * 3. To use branching, update the message data structure to support multiple branches
 * 
 * 4. For streaming, pass parseIncompleteMarkdown={isStreaming} to MessageResponse
 * 
 * 5. The MessageToolbar provides a clean layout for action buttons
 * 
 * Key improvements made:
 * ✅ Added @source for Streamdown in globals.css
 * ✅ Added MessageActions with Copy, Retry, Like, Dislike
 * ✅ Added MessageToolbar for better action layout
 * ✅ Added parseIncompleteMarkdown prop for streaming
 * ✅ Created examples for branching, streaming, and markdown
 */
