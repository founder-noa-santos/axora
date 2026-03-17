# Phase 4 Sprint C5: Chat Interface

**Agent:** C (Implementation Specialist — Tauri + Backend Integration)  
**Sprint:** C5  
**Priority:** HIGH  
**Estimated:** 8 hours  
**Subagents:** ENABLED (GPT-5.4)

---

## 🎯 Mission

Implement Chat interface for mission input, message history, and file attachment.

**Context:** Users need to submit missions and see conversation history with Coordinator.

**Difficulty:** ⚠️ **MEDIUM-HIGH** — Chat UI, state management, file upload, API integration

---

## 📋 Subagents Assignment

**You are Lead Agent. Delegate to 3 subagents:**

### Subagent 1: Chat UI + Message Display
**Task:** Implement chat panel UI with message history
**File:** `apps/desktop/src/panels/ChatPanel.tsx`
**Deliverables:**
- Chat panel layout (messages container, input area)
- Message component (user vs assistant messages)
- Message history scrolling
- Typing indicator
- 5+ tests

### Subagent 2: Mission Input + Submission
**Task:** Implement mission input and submission logic
**File:** `apps/desktop/src/components/chat/MissionInput.tsx`
**Deliverables:**
- Text input (multi-line, auto-resize)
- Submit button (with loading state)
- Keyboard shortcuts (Enter to send, Cmd+Enter for newline)
- Mission submission to API
- 5+ tests

### Subagent 3: File Attachment + Context
**Task:** Implement file attachment for mission context
**File:** `apps/desktop/src/components/chat/FileAttachment.tsx`
**Deliverables:**
- File upload (drag & drop, click to browse)
- File preview (name, size, type)
- Remove attachment
- File upload to backend
- 5+ tests

---

## 🏗️ Lead Agent Responsibilities

**You must:**

1. **Coordinate 3 Subagents:**
   - Assign tasks to all 3 subagents
   - Review chat UI + input + file attachment
   - Ensure consistency

2. **Integrate Components:**
   - Connect Chat UI to mission submission
   - Integrate file attachment with mission input
   - Add message store (state management)

3. **Implement Message Store:**
   - Message history (user + assistant)
   - Loading states (pending, submitting)
   - Error handling (failed submissions)
   - Clear conversation

4. **Write Integration Tests:**
   - Test message display
   - Test mission submission
   - Test file attachment
   - Test conversation history

5. **Update Documentation:**
   - Add chat interface documentation
   - Add usage examples

---

## 📐 Technical Spec

### Message Types

```typescript
// apps/desktop/src/types/message.ts
export type MessageRole = 'user' | 'assistant' | 'system';

export interface Message {
  id: string;
  role: MessageRole;
  content: string;
  timestamp: number;
  attachments?: FileAttachment[];
  metadata?: {
    missionId?: string;
    status?: 'pending' | 'submitted' | 'completed' | 'failed';
  };
}

export interface FileAttachment {
  id: string;
  name: string;
  size: number;
  type: string;
  path?: string; // Local file path
  url?: string;  // Uploaded URL
}
```

### Message Store

```typescript
// apps/desktop/src/store/message-store.ts
import { create } from 'zustand';
import { Message } from '../types/message';

interface MessageStore {
  messages: Message[];
  isLoading: boolean;
  error: string | null;
  
  // Actions
  addMessage: (message: Message) => void;
  updateMessage: (id: string, updates: Partial<Message>) => void;
  clearMessages: () => void;
  submitMission: (content: string, attachments?: FileAttachment[]) => Promise<void>;
}

export const useMessageStore = create<MessageStore>((set, get) => ({
  messages: [],
  isLoading: false,
  error: null,
  
  addMessage: (message) => {
    set((state) => ({
      messages: [...state.messages, message],
    }));
  },
  
  updateMessage: (id, updates) => {
    set((state) => ({
      messages: state.messages.map((msg) =>
        msg.id === id ? { ...msg, ...updates } : msg
      ),
    }));
  },
  
  clearMessages: () => {
    set({ messages: [] });
  },
  
  submitMission: async (content, attachments) => {
    set({ isLoading: true, error: null });
    
    try {
      // Add user message
      const userMessage: Message = {
        id: Date.now().toString(),
        role: 'user',
        content,
        timestamp: Date.now(),
        attachments,
        metadata: { status: 'submitted' },
      };
      get().addMessage(userMessage);
      
      // Submit to API
      const response = await fetch('/api/missions', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ content, attachments }),
      });
      
      const result = await response.json();
      
      // Add assistant response
      const assistantMessage: Message = {
        id: (Date.now() + 1).toString(),
        role: 'assistant',
        content: result.message || 'Mission submitted successfully',
        timestamp: Date.now(),
        metadata: { missionId: result.missionId, status: 'completed' },
      };
      get().addMessage(assistantMessage);
      
      set({ isLoading: false });
    } catch (error) {
      set({ 
        isLoading: false, 
        error: error.message 
      });
      
      // Add error message
      const errorMessage: Message = {
        id: Date.now().toString(),
        role: 'system',
        content: `Error: ${error.message}`,
        timestamp: Date.now(),
        metadata: { status: 'failed' },
      };
      get().addMessage(errorMessage);
    }
  },
}));
```

### Chat Panel UI

```tsx
// apps/desktop/src/panels/ChatPanel.tsx
import { useEffect, useRef } from 'react';
import { useMessageStore } from '../store/message-store';
import { MissionInput } from '../components/chat/MissionInput';
import { MessageList } from '../components/chat/MessageList';

export function ChatPanel() {
  const { messages, isLoading } = useMessageStore();
  const messagesEndRef = useRef<HTMLDivElement>(null);
  
  // Auto-scroll to bottom
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);
  
  return (
    <div className="chat-panel">
      <header className="panel-header">
        <h2>Chat</h2>
      </header>
      
      <div className="chat-content">
        <MessageList messages={messages} />
        <div ref={messagesEndRef} />
      </div>
      
      <div className="chat-input">
        <MissionInput disabled={isLoading} />
      </div>
    </div>
  );
}
```

---

## ✅ Success Criteria

**Sprint is done when:**

- [ ] 3 subagents complete their tasks
- [ ] Lead agent integrates all components
- [ ] Chat panel compiles and works
- [ ] Mission submission works
- [ ] File attachment works
- [ ] Message history displays correctly
- [ ] 15+ tests passing (5 per subagent + 5 integration)
- [ ] Keyboard shortcuts work (Enter, Cmd+Enter)
- [ ] Auto-scroll works (new messages)

---

## 🔗 Dependencies

**Requires:**
- Sprint C4 complete (Tauri setup)
- Sprint A4 complete (UI Components)

**Blocks:**
- Sprint C6 (Integration needs chat interface)

---

## 📝 Notes for GPT-5.4

**Subagent Pattern:**
```
Lead Agent:
  ├─ Subagent 1: Chat UI (parallel)
  ├─ Subagent 2: Mission Input (parallel)
  └─ Subagent 3: File Attachment (parallel)
  ↓
Lead Agent: Integration + Message Store + Tests
```

**Chat Design:**
- Familiar chat interface (like messaging apps)
- Clear distinction (user vs assistant messages)
- File attachment support
- Loading states (typing indicator)

**Difficulty: MEDIUM-HIGH**
- 3 subagents to coordinate
- State management complexity
- File upload integration
- Real-time updates (when backend ready)

**Review Checklist:**
- [ ] Chat UI renders correctly
- [ ] Mission submission works
- [ ] File attachment works (upload + preview)
- [ ] Message history scrolls correctly
- [ ] Keyboard shortcuts work
- [ ] Auto-scroll to new messages

---

**Start AFTER Sprint C4 and A4 complete.**
