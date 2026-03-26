# AI SDK Elements - Message Components Implementation

## Overview

This document describes the implementation of AI SDK Elements message components in the Aktacode Desktop application, based on the official documentation: https://elements.ai-sdk.dev/components/message

## What Was Changed

### 1. globals.css - Streamdown CSS Source

**File:** `aktacode/apps/desktop/app/globals.css`

Added the required Streamdown CSS source import:

```css
/* Required for MessageResponse component (Streamdown styles) */
@source "../node_modules/streamdown/dist/*.js";
```

⚠️ **Important:** This is required for the `MessageResponse` component to work properly. Without this import, the Streamdown styles will not be applied.

### 2. ChatAssistantMessage.tsx - Added Actions Support

**File:** `aktacode/apps/desktop/components/chat/ChatAssistantMessage.tsx`

**New Features:**
- ✅ Copy to clipboard functionality
- ✅ Retry/regenerate response
- ✅ Like/Dislike feedback
- ✅ MessageToolbar for action buttons
- ✅ `parseIncompleteMarkdown` prop for streaming support

**New Props:**
```typescript
interface ChatAssistantMessageProps {
  message: UiMessage;
  className?: string;
  onRetry?: () => void;
  onCopy?: (content: string) => void;
  onFeedback?: (feedback: "like" | "dislike") => void;
}
```

**Key Changes:**
```tsx
// Added MessageToolbar with actions
{!isStreaming && message.content && (
  <MessageToolbar>
    <MessageActions>
      <MessageAction onClick={onRetry} label="Retry" />
      <MessageAction onClick={handleCopy} label="Copy" />
      <MessageAction onClick={() => onFeedback("like")} label="Like" />
      <MessageAction onClick={() => onFeedback("dislike")} label="Dislike" />
    </MessageActions>
  </MessageToolbar>
)}
```

### 3. ChatMessage.tsx - Props Propagation

**File:** `aktacode/apps/desktop/components/chat/ChatMessage.tsx`

Updated to pass action callbacks to child components:

```typescript
interface ChatMessageProps {
  message: UiMessage;
  className?: string;
  onRetry?: () => void;
  onCopy?: (content: string) => void;
  onFeedback?: (feedback: "like" | "dislike") => void;
}
```

### 4. ChatConversation.tsx - Props Propagation

**File:** `aktacode/apps/desktop/components/chat/ChatConversation.tsx`

Updated to accept and pass action callbacks:

```typescript
interface ChatConversationProps {
  messages: UiMessage[];
  emptyState?: ReactNode;
  onRetry?: () => void;
  onCopy?: (content: string) => void;
  onFeedback?: (feedback: "like" | "dislike") => void;
}
```

## Available Components

### Core Message Components

All components are exported from `@/components/ai-elements/message`:

#### Message
Container for a single message (user or assistant).

```tsx
<Message from="assistant" className="...">
  <MessageContent>...</MessageContent>
</Message>
```

#### MessageContent
Content wrapper with appropriate styling based on sender.

#### MessageResponse
Markdown renderer with Streamdown support.

```tsx
<MessageResponse parseIncompleteMarkdown={isStreaming}>
  {message.content}
</MessageResponse>
```

**Props:**
- `parseIncompleteMarkdown?: boolean` - Fix incomplete markdown during streaming (default: true)
- `className?: string`
- `components?: object` - Custom markdown components
- `allowedImagePrefixes?: string[]` - Security for images
- `allowedLinkPrefixes?: string[]` - Security for links

#### MessageActions
Container for action buttons.

```tsx
<MessageActions>
  <MessageAction label="Copy" onClick={...}>
    <CopyIcon className="size-3" />
  </MessageAction>
</MessageActions>
```

#### MessageAction
Individual action button with tooltip support.

```tsx
<MessageAction
  onClick={handleCopy}
  label="Copy"
  tooltip="Copy to clipboard"
>
  <CopyIcon className="size-3" />
</MessageAction>
```

#### MessageToolbar
Toolbar container for actions and branch selectors.

```tsx
<MessageToolbar>
  <MessageActions>...</MessageActions>
  <MessageBranchSelector>...</MessageBranchSelector>
</MessageToolbar>
```

### Advanced: Message Branching

For supporting multiple response versions:

```tsx
<MessageBranch defaultBranch={0}>
  <MessageBranchContent>
    <MessageResponse>Response version 1</MessageResponse>
    <MessageResponse>Response version 2</MessageResponse>
    <MessageResponse>Response version 3</MessageResponse>
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
    </MessageActions>
  </MessageToolbar>
</MessageBranch>
```

**Branching Components:**
- `MessageBranch` - Context provider for branching
- `MessageBranchContent` - Container for branch content
- `MessageBranchSelector` - Button group for navigation
- `MessageBranchPrevious` - Previous branch button
- `MessageBranchNext` - Next branch button
- `MessageBranchPage` - Page indicator (e.g., "2 of 3")

## Usage Examples

### Basic Chat with Actions

```tsx
import { ChatConversation } from "@/components/chat/ChatConversation";

function MyChat() {
  const handleCopy = (content: string) => {
    console.log("Copied:", content);
  };

  const handleRetry = () => {
    console.log("Retrying last message...");
  };

  const handleFeedback = (feedback: "like" | "dislike") => {
    console.log("Feedback:", feedback);
  };

  return (
    <ChatConversation
      messages={messages}
      onCopy={handleCopy}
      onRetry={handleRetry}
      onFeedback={handleFeedback}
    />
  );
}
```

### Streaming Response

```tsx
function StreamingMessage({ isStreaming, content }) {
  return (
    <Message from="assistant">
      <MessageContent>
        <MessageResponse parseIncompleteMarkdown={isStreaming}>
          {content}
        </MessageResponse>
      </MessageContent>
      
      {!isStreaming && (
        <MessageToolbar>
          <MessageActions>
            <MessageAction label="Copy" onClick={handleCopy}>
              <CopyIcon />
            </MessageAction>
            <MessageAction label="Retry" onClick={handleRetry}>
              <RefreshCcwIcon />
            </MessageAction>
          </MessageActions>
        </MessageToolbar>
      )}
    </Message>
  );
}
```

## Integration with Aktacode Desktop

### Current Implementation Status

✅ **Completed:**
- Streamdown CSS source added to globals.css
- Message actions (Copy, Retry, Like, Dislike)
- MessageToolbar layout
- parseIncompleteMarkdown for streaming
- Props propagation through component tree

🔄 **To Implement (Optional):**
- Message branching support (requires data structure changes)
- Actual retry logic integration with AI backend
- Feedback persistence (save like/dislike to database)
- Share functionality

### Next Steps

1. **Connect Actions to Backend:**
   - Implement actual retry logic that calls the AI API
   - Save feedback (like/dislike) to improve model
   - Add analytics for action usage

2. **Add Branching Support (Optional):**
   - Update message data structure to support multiple branches
   - Implement branch navigation in ChatView
   - Add backend support for generating multiple responses

3. **Enhance Streaming:**
   - Ensure `parseIncompleteMarkdown` is properly set during streaming
   - Add loading indicators
   - Improve streaming performance

## References

- **Official Documentation:** https://elements.ai-sdk.dev/components/message
- **Streamdown (Markdown Renderer):** https://streamdown.ai/
- **Example File:** `aktacode/apps/desktop/components/chat/AI_SDK_EXAMPLES.tsx`

## Component Files

```
aktacode/apps/desktop/components/
├── ai-elements/
│   └── message.tsx              # Core message components
│   └── conversation.tsx         # Conversation container
│   └── reasoning.tsx            # Reasoning block components
├── chat/
│   ├── ChatView.tsx             # Main chat view
│   ├── ChatConversation.tsx     # Conversation wrapper
│   ├── ChatMessage.tsx          # Message router by role
│   ├── ChatAssistantMessage.tsx # Assistant message with actions
│   ├── ChatUserMessage.tsx      # User message styling
│   └── AI_SDK_EXAMPLES.tsx      # Usage examples
```

## Key Takeaways

1. **Always add the Streamdown CSS source** to globals.css
2. **Use MessageToolbar** for consistent action button layout
3. **Set parseIncompleteMarkdown** based on streaming state
4. **Pass callbacks** (onRetry, onCopy, onFeedback) through the component tree
5. **Branching is optional** - implement only if you need multiple response versions
