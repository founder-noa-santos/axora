# AI SDK Elements Message Components - Complete Implementation Prompt

**Prompt for LLM: Complete Frontend Chat UI Implementation with AI SDK Elements**

---

## Context & Objective

You are implementing a production-ready chat interface for the Aktacode Desktop application using **AI SDK Elements** message components. This is a critical UI component that handles all user-assistant communication.

**Primary Goal:** Replace the current custom message components with the **pure, unmodified components from the AI SDK Elements library**, consuming them exactly as documented at https://elements.ai-sdk.dev/components/message

---

## Critical Requirements

### 1. USE PURE AI SDK ELEMENTS COMPONENTS

**DO NOT** create custom implementations of these components. **DO NOT** modify the library components. **CONSUME THEM AS-IS** from the package.

The components MUST be imported and used exactly as shown in the official documentation:

```typescript
// ✅ CORRECT - Import from the library
import {
  Message,
  MessageContent,
  MessageResponse,
  MessageActions,
  MessageAction,
  MessageToolbar,
} from "@/components/ai-elements/message";

// ❌ WRONG - Do NOT create your own Message component
// Do NOT modify the library components
// Do NOT reinvent the wheel
```

### 2. MANDATORY: Add Streamdown CSS Source

**BEFORE doing anything else**, you MUST add this to `globals.css`:

```css
/* REQUIRED for MessageResponse component - DO NOT SKIP */
@source "../node_modules/streamdown/dist/*.js";
```

**Why:** The `MessageResponse` component uses Streamdown for markdown rendering. Without this import, styles will NOT be applied and the component will break.

**Verification:** After adding this, verify the file exists at `node_modules/streamdown/dist/`

---

## Documentation References

### Primary Documentation
- **AI SDK Elements - Message Components:** https://elements.ai-sdk.dev/components/message
- **Streamdown (Markdown Renderer):** https://streamdown.ai/

### Key Documentation Points to Follow

1. **Message Component Props:**
   - `from: "user" | "assistant" | "system"` - REQUIRED
   - `className?: string` - Optional custom classes
   - All other props spread to root div

2. **MessageResponse Props:**
   - `parseIncompleteMarkdown?: boolean` - Default true, fix incomplete markdown during streaming
   - `className?: string`
   - `components?: object` - Custom markdown components
   - `allowedImagePrefixes?: string[]` - Default ["*"]
   - `allowedLinkPrefixes?: string[]` - Default ["*"]
   - `rehypePlugins?: array` - Default [rehypeKatex]
   - `remarkPlugins?: array` - Default [remarkGfm, remarkMath]

3. **MessageAction Props:**
   - `label: string` - Accessible label (REQUIRED for a11y)
   - `tooltip?: string` - Hover tooltip text
   - `onClick: () => void` - Click handler
   - All Button props spread (variant, size, etc.)

4. **MessageToolbar:**
   - Container for actions and branch selectors
   - Flex layout with space-between alignment
   - Use for placing actions below message content

---

## Implementation Tasks

### Task 1: Verify/Install Dependencies

**Check package.json for these dependencies:**

```json
{
  "dependencies": {
    "streamdown": "latest",
    "@streamdown/cjk": "latest",
    "@streamdown/code": "latest",
    "@streamdown/math": "latest",
    "@streamdown/mermaid": "latest"
  }
}
```

**If missing, install:**
```bash
npm install streamdown @streamdown/cjk @streamdown/code @streamdown/math @streamdown/mermaid
```

### Task 2: Update globals.css

**File:** `aktacode/apps/desktop/app/globals.css`

**Add at the top, after other imports:**

```css
/* ==========================================================================
   AI SDK ELEMENTS - STREAMDOWN CSS SOURCE
   REQUIRED for MessageResponse component to work properly
   https://elements.ai-sdk.dev/components/message
   ========================================================================== */
@source "../node_modules/streamdown/dist/*.js";
```

### Task 3: Replace ChatAssistantMessage

**File:** `aktacode/apps/desktop/components/chat/ChatAssistantMessage.tsx`

**Complete replacement using AI SDK Elements:**

```typescript
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
import { CopyIcon, RotateCcwIcon, ThumbsUpIcon, ThumbsDownIcon } from "lucide-react";

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
            {/* CRITICAL: parseIncompleteMarkdown must be set based on streaming state */}
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

      {/* MessageToolbar with actions - only show when not streaming and has content */}
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
```

**Key Points:**
- ✅ Use `Message` component from AI SDK Elements (NOT custom)
- ✅ Use `MessageContent` component from AI SDK Elements
- ✅ Use `MessageResponse` with `parseIncompleteMarkdown` prop
- ✅ Use `MessageToolbar` for action layout
- ✅ Use `MessageAction` with `label` and `tooltip` props
- ✅ Actions only show when `!isStreaming && message.content`

### Task 4: Replace ChatUserMessage

**File:** `aktacode/apps/desktop/components/chat/ChatUserMessage.tsx`

```typescript
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
        <MessageResponse parseIncompleteMarkdown={false}>
          {message.content}
        </MessageResponse>
      </MessageContent>
    </Message>
  );
}
```

**Note:** Even user messages should use `MessageResponse` for consistent markdown rendering.

### Task 5: Update ChatMessage

**File:** `aktacode/apps/desktop/components/chat/ChatMessage.tsx`

```typescript
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
      return (
        <ChatUserMessage
          message={message}
          className={className}
        />
      );
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
```

### Task 6: Update ChatConversation

**File:** `aktacode/apps/desktop/components/chat/ChatConversation.tsx`

```typescript
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
```

**Note:** Added `ConversationScrollButton` for better UX (auto-scroll to bottom button).

### Task 7: Update ChatView to Pass Callbacks

**File:** `aktacode/apps/desktop/components/chat/ChatView.tsx`

Add the callback handlers and pass them to ChatConversation:

```typescript
// Add these handlers inside ChatView component

const handleCopy = useCallback((content: string) => {
  // TODO: Add toast notification
  console.log("Copied to clipboard:", content);
}, []);

const handleRetry = useCallback(() => {
  // TODO: Implement retry logic with AI backend
  console.log("Retry last message");
}, []);

const handleFeedback = useCallback((feedback: "like" | "dislike") => {
  // TODO: Send feedback to backend for analytics
  console.log("Feedback:", feedback);
}, []);

// Pass to ChatConversation
<ChatConversation
  messages={uiMessages}
  emptyState={emptyState}
  onCopy={handleCopy}
  onRetry={handleRetry}
  onFeedback={handleFeedback}
/>
```

---

## Verification Checklist

After implementation, verify:

### Visual Verification
- [ ] User messages aligned right with secondary background
- [ ] Assistant messages full-width with proper spacing
- [ ] Action buttons appear below assistant messages (Copy, Retry, Like, Dislike)
- [ ] Action buttons have tooltips on hover
- [ ] Scroll-to-bottom button appears when not at bottom
- [ ] Markdown renders correctly (headers, code blocks, lists, tables)
- [ ] Code blocks have syntax highlighting
- [ ] Math equations render (if using $ or $$)

### Functional Verification
- [ ] Copy button copies message content to clipboard
- [ ] Retry button triggers callback (check console)
- [ ] Like/Dislike buttons trigger callback (check console)
- [ ] Messages scroll properly
- [ ] Streaming messages render correctly (no broken markdown)
- [ ] No console errors related to Streamdown or MessageResponse

### Code Verification
- [ ] `@source "../node_modules/streamdown/dist/*.js";` added to globals.css
- [ ] All Message components imported from `@/components/ai-elements/message`
- [ ] No custom Message component implementations
- [ ] `parseIncompleteMarkdown` prop used correctly
- [ ] MessageToolbar used for action layout
- [ ] All MessageAction components have `label` prop (accessibility)

---

## Common Mistakes to AVOID

❌ **DO NOT** create your own Message component
```typescript
// ❌ WRONG
export function Message({ children, from }) {
  return <div className={from === "user" ? "ml-auto" : ""}>{children}</div>
}

// ✅ CORRECT
import { Message } from "@/components/ai-elements/message";
```

❌ **DO NOT** forget the Streamdown CSS source
```css
/* ❌ WRONG - Missing this will break MessageResponse */
// No @source import

/* ✅ CORRECT */
@source "../node_modules/streamdown/dist/*.js";
```

❌ **DO NOT** use MessageResponse without parseIncompleteMarkdown during streaming
```typescript
// ❌ WRONG - Will show broken markdown during streaming
<MessageResponse>{content}</MessageResponse>

// ✅ CORRECT - Fixes incomplete markdown automatically
<MessageResponse parseIncompleteMarkdown={!isComplete}>{content}</MessageResponse>
```

❌ **DO NOT** forget label prop on MessageAction
```typescript
// ❌ WRONG - Breaks accessibility
<MessageAction onClick={handleCopy}>
  <CopyIcon />
</MessageAction>

// ✅ CORRECT - Screen readers can announce it
<MessageAction onClick={handleCopy} label="Copy">
  <CopyIcon />
</MessageAction>
```

❌ **DO NOT** show actions during streaming
```typescript
// ❌ WRONG - Actions flicker during streaming
<MessageToolbar>
  <MessageActions>...</MessageActions>
</MessageToolbar>

// ✅ CORRECT - Only show when complete
{!isStreaming && (
  <MessageToolbar>
    <MessageActions>...</MessageActions>
  </MessageToolbar>
)}
```

---

## Advanced: Message Branching (Optional)

If you want to support multiple response versions (like ChatGPT's "Regenerate" showing alternatives):

```typescript
import {
  MessageBranch,
  MessageBranchContent,
  MessageBranchSelector,
  MessageBranchPrevious,
  MessageBranchNext,
  MessageBranchPage,
} from "@/components/ai-elements/message";

function BranchingMessage({ branches }: { branches: string[] }) {
  return (
    <MessageBranch defaultBranch={0}>
      <MessageBranchContent>
        {branches.map((branch, i) => (
          <MessageResponse key={i}>{branch}</MessageResponse>
        ))}
      </MessageBranchContent>
      
      <MessageToolbar>
        <MessageBranchSelector>
          <MessageBranchPrevious />
          <MessageBranchPage />
          <MessageBranchNext />
        </MessageBranchSelector>
        
        <MessageActions>
          <MessageAction label="Retry">
            <RefreshCcwIcon />
          </MessageAction>
          <MessageAction label="Copy">
            <CopyIcon />
          </MessageAction>
        </MessageActions>
      </MessageToolbar>
    </MessageBranch>
  );
}
```

---

## Testing Instructions

### 1. Test Markdown Rendering
```typescript
const testMarkdown = `
# Header 1
## Header 2

**Bold** and *italic*

- List item 1
- List item 2

\`\`\`typescript
function test() {
  return "Hello";
}
\`\`\`

| Col 1 | Col 2 |
|-------|-------|
| A     | B     |
`;
```

### 2. Test Streaming
```typescript
// Simulate streaming
const [content, setContent] = useState("");
const [isStreaming, setIsStreaming] = useState(true);

useEffect(() => {
  const fullText = "Hello **world** this is a `test`";
  let index = 0;
  
  const interval = setInterval(() => {
    if (index < fullText.length) {
      setContent(prev => prev + fullText[index]);
      index++;
    } else {
      setIsStreaming(false);
      clearInterval(interval);
    }
  }, 50);
  
  return () => clearInterval(interval);
}, []);

// Use in MessageResponse
<MessageResponse parseIncompleteMarkdown={isStreaming}>
  {content}
</MessageResponse>
```

### 3. Test Actions
```typescript
// Verify clipboard
const handleCopy = async (content: string) => {
  await navigator.clipboard.writeText(content);
  const clipboardContent = await navigator.clipboard.readText();
  console.assert(clipboardContent === content, "Copy failed!");
};

// Verify callbacks fire
const handleFeedback = (feedback: "like" | "dislike") => {
  console.log("Feedback received:", feedback);
  // Add a toast notification here
};
```

---

## File Structure

After implementation, your structure should be:

```
aktacode/apps/desktop/
├── app/
│   └── globals.css                    # ✅ Has @source for Streamdown
├── components/
│   ├── ai-elements/
│   │   ├── message.tsx                # ✅ Pure AI SDK Elements (DO NOT MODIFY)
│   │   ├── conversation.tsx           # ✅ Pure AI SDK Elements (DO NOT MODIFY)
│   │   └── reasoning.tsx              # ✅ Pure AI SDK Elements (DO NOT MODIFY)
│   └── chat/
│       ├── ChatView.tsx               # ✅ Updated with callbacks
│       ├── ChatConversation.tsx       # ✅ Updated with props
│       ├── ChatMessage.tsx            # ✅ Updated with props
│       ├── ChatAssistantMessage.tsx   # ✅ Complete rewrite with AI SDK
│       ├── ChatUserMessage.tsx        # ✅ Complete rewrite with AI SDK
│       ├── ChatCheckpoint.tsx         # (unchanged)
│       ├── ChatToolCall.tsx           # (unchanged)
│       └── ChatToolApproval.tsx       # (unchanged)
```

---

## Success Criteria

✅ **All of these must be true:**

1. MessageResponse renders markdown correctly (headers, lists, code, tables)
2. Code blocks have syntax highlighting
3. Actions (Copy, Retry, Like, Dislike) appear below assistant messages
4. Copy button actually copies to clipboard
5. No console errors about Streamdown or missing styles
6. Streaming messages don't show broken markdown
7. User messages aligned right, assistant messages full-width
8. Scroll-to-bottom button works
9. Tooltips appear on action button hover
10. No custom Message component implementations

---

## Troubleshooting

### MessageResponse styles not working
**Solution:** Verify `@source "../node_modules/streamdown/dist/*.js";` is in globals.css

### Actions not appearing
**Solution:** Check `!isStreaming && message.content` condition

### Copy not working
**Solution:** Ensure running in HTTPS or localhost (clipboard API requirement)

### Markdown not rendering
**Solution:** Verify MessageResponse is used, not plain text

### Build errors about Streamdown
**Solution:** Run `npm install streamdown @streamdown/*`

---

## Final Notes

This implementation follows the **official AI SDK Elements documentation** exactly. Do not deviate from the documented patterns unless you have a very specific reason.

**Remember:**
- Use the library components AS-IS
- Do NOT modify the library components
- Do NOT create custom implementations
- Follow the documentation examples closely
- Test thoroughly before considering complete

**Documentation:** https://elements.ai-sdk.dev/components/message

Good luck! 🚀
