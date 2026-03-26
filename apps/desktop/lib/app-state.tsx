"use client";

import {
  createContext,
  useContext,
  useState,
  useCallback,
  ReactNode,
} from "react";

// ============ TYPES ============

export interface Message {
  id: string;
  role: "user" | "assistant";
  content: string;
  timestamp: number;
}

export interface ExecutionTraceItem {
  id: string;
  messageId: string;
  toolCallId: string;
  toolName: string;
  toolKind: string;
  phase:
    | "requested"
    | "approved"
    | "started"
    | "progress"
    | "completed"
    | "failed"
    | "denied";
  status: "pending" | "running" | "input-available" | "complete" | "error";
  parameters: Record<string, unknown>;
  result?: string;
  error?: string;
  requiresApproval: boolean;
  timestamp: number;
}

export interface Thread {
  id: string;
  title: string;
  projectId: string;
  messages: Message[];
  executionTrace: ExecutionTraceItem[];
  createdAt: number;
  updatedAt: number;
}

export interface Project {
  id: string;
  name: string;
  path: string;
  icon?: "folder" | "github";
}

export interface AppState {
  threads: Thread[];
  currentThreadId: string | null;
  projects: Project[];
  currentProjectId: string | null;
  isChatOpen: boolean;
}

// ============ MOCK DATA ============

const MOCK_PROJECTS: Project[] = [
  {
    id: "proj-1",
    name: "openakta",
    path: "~/Projects/openakta",
    icon: "github",
  },
  {
    id: "proj-2",
    name: "nexus-social",
    path: "~/Projects/nexus-social",
    icon: "github",
  },
  {
    id: "proj-3",
    name: "fluri-v0",
    path: "~/Projects/fluri-v0",
    icon: "folder",
  },
];

const INITIAL_THREADS: Thread[] = [
  {
    id: "thread-1",
    title: "openakta",
    projectId: "proj-1",
    messages: [],
    executionTrace: [],
    createdAt: Date.now(),
    updatedAt: Date.now(),
  },
  {
    id: "thread-2",
    title: "nexus-social",
    projectId: "proj-2",
    messages: [],
    executionTrace: [],
    createdAt: Date.now(),
    updatedAt: Date.now(),
  },
  {
    id: "thread-3",
    title: "fluri-v0",
    projectId: "proj-3",
    messages: [],
    executionTrace: [],
    createdAt: Date.now(),
    updatedAt: Date.now(),
  },
];

// ============ MOCK LLM RESPONSES ============

const MOCK_RESPONSES = [
  "I'll help you with that. Let me analyze the codebase and provide a solution.",
  "Great question! Based on the project structure, I recommend the following approach:",
  "I've reviewed your code and found a few optimization opportunities. Here's what I suggest:",
  "Let me break this down into manageable steps for you.",
  "I can help implement this feature. Here's my plan:",
];

function generateMockResponse(userMessage: string): string {
  // Simple keyword-based mock responses
  const lowerMessage = userMessage.toLowerCase();

  if (lowerMessage.includes("error") || lowerMessage.includes("bug")) {
    return [
      "I've identified the issue. Let me walk you through the fix:",
      "",
      "1. First, we need to check the error logs",
      "2. Then we'll identify the root cause",
      "3. Finally, we'll implement the fix",
      "",
      "Would you like me to show you the specific code changes?",
    ].join("\n");
  }

  if (
    lowerMessage.includes("implement") ||
    lowerMessage.includes("create") ||
    lowerMessage.includes("add")
  ) {
    return [
      "I'll help you implement this feature. Here's my approach:",
      "",
      "```typescript",
      "// Example implementation",
      "const feature = await implementFeature({",
      '  name: "your-feature",',
      '  priority: "high"',
      "});",
      "```",
      "",
      "Shall I proceed with the full implementation?",
    ].join("\n");
  }

  if (
    lowerMessage.includes("explain") ||
    lowerMessage.includes("what") ||
    lowerMessage.includes("how")
  ) {
    return [
      "Let me explain this in detail:",
      "",
      "**Overview:**",
      "This is a common pattern in modern web development.",
      "",
      "**Key Points:**",
      "- First, understand the core concept",
      "- Then, apply it to your specific use case",
      "- Finally, test thoroughly",
      "",
      "Would you like more details on any specific aspect?",
    ].join("\n");
  }

  // Default response
  return MOCK_RESPONSES[Math.floor(Math.random() * MOCK_RESPONSES.length)];
}

// ============ CONTEXT ============

interface AppContextType extends AppState {
  // Thread actions
  createThread: (projectId?: string) => void;
  selectThread: (threadId: string) => void;
  closeChat: () => void;

  // Message actions
  sendMessage: (content: string) => Promise<void>;

  // Project actions
  addProject: (project: Omit<Project, "id">) => string;
  selectProject: (projectId: string) => void;

  // Helper actions
  getProjectName: (projectId: string) => string;
}

const AppContext = createContext<AppContextType | null>(null);

// ============ PROVIDER ============

export function AppProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<AppState>({
    threads: INITIAL_THREADS,
    currentThreadId: null,
    projects: MOCK_PROJECTS,
    currentProjectId: null,
    isChatOpen: false,
  });

  // Create new thread
  const createThread = useCallback(
    (projectId?: string) => {
      const newProjectId =
        projectId || state.currentProjectId || MOCK_PROJECTS[0].id;
      const project = state.projects.find((p) => p.id === newProjectId);

      const newThread: Thread = {
        id: `thread-${Date.now()}`,
        title: project?.name || "New thread",
        projectId: newProjectId,
        messages: [],
        executionTrace: [],
        createdAt: Date.now(),
        updatedAt: Date.now(),
      };

      setState((prev) => ({
        ...prev,
        threads: [newThread, ...prev.threads],
        currentThreadId: newThread.id,
        currentProjectId: newProjectId,
        isChatOpen: true,
      }));
    },
    [state.currentProjectId, state.projects],
  );

  // Select thread
  const selectThread = useCallback(
    (threadId: string) => {
      const thread = state.threads.find((t) => t.id === threadId);
      setState((prev) => ({
        ...prev,
        currentThreadId: threadId,
        currentProjectId: thread?.projectId ?? prev.currentProjectId,
        isChatOpen: true,
      }));
    },
    [state.threads],
  );

  // Add project
  const addProject = useCallback((project: Omit<Project, "id">) => {
    const nextId = `proj-${Date.now()}`;
    let resolvedId = nextId;

    setState((prev) => {
      const existing = prev.projects.find(
        (p) => p.path === project.path || p.name === project.name,
      );
      if (existing) {
        resolvedId = existing.id;
        return {
          ...prev,
          currentProjectId: existing.id,
        };
      }

      const newProject: Project = {
        id: nextId,
        ...project,
      };
      resolvedId = newProject.id;

      return {
        ...prev,
        projects: [newProject, ...prev.projects],
        currentProjectId: newProject.id,
      };
    });

    return resolvedId;
  }, []);

  // Close chat
  const closeChat = useCallback(() => {
    setState((prev) => ({
      ...prev,
      isChatOpen: false,
    }));
  }, []);

  // Send message
  const sendMessage = useCallback(
    async (content: string) => {
      if (!state.currentThreadId) return;

      // Add user message
      const userMessage: Message = {
        id: `msg-${Date.now()}`,
        role: "user",
        content,
        timestamp: Date.now(),
      };

      setState((prev) => ({
        ...prev,
        threads: prev.threads.map((t) =>
          t.id === prev.currentThreadId
            ? {
                ...t,
                messages: [...t.messages, userMessage],
                updatedAt: Date.now(),
              }
            : t,
        ),
      }));

      // Simulate LLM response delay
      await new Promise((resolve) =>
        setTimeout(resolve, 1000 + Math.random() * 1000),
      );

      // Generate and add assistant response
      const assistantMessage: Message = {
        id: `msg-${Date.now() + 1}`,
        role: "assistant",
        content: generateMockResponse(content),
        timestamp: Date.now(),
      };
      const traceItems = generateMockTrace(content, assistantMessage.id);

      setState((prev) => ({
        ...prev,
        threads: prev.threads.map((t) =>
          t.id === prev.currentThreadId
            ? {
                ...t,
                messages: [...t.messages, assistantMessage],
                executionTrace: [...t.executionTrace, ...traceItems],
                updatedAt: Date.now(),
              }
            : t,
        ),
      }));
    },
    [state.currentThreadId],
  );

  // Select project
  const selectProject = useCallback((projectId: string) => {
    setState((prev) => {
      // If chat is not open, just change project
      if (!prev.isChatOpen) {
        return {
          ...prev,
          currentProjectId: projectId,
        };
      }

      // If chat is open, find or create thread for this project
      const existingThread = prev.threads.find(
        (t) => t.projectId === projectId,
      );

      if (existingThread) {
        return {
          ...prev,
          currentThreadId: existingThread.id,
          currentProjectId: projectId,
          isChatOpen: true,
        };
      } else {
        // Create new thread for this project
        const project = prev.projects.find((p) => p.id === projectId);
        const newThread: Thread = {
          id: `thread-${Date.now()}`,
          title: project?.name || "New thread",
          projectId,
          messages: [],
          executionTrace: [],
          createdAt: Date.now(),
          updatedAt: Date.now(),
        };

        return {
          ...prev,
          threads: [newThread, ...prev.threads],
          currentThreadId: newThread.id,
          currentProjectId: projectId,
          isChatOpen: true,
        };
      }
    });
  }, []);

  // Get project name helper
  const getProjectName = useCallback(
    (projectId: string) => {
      return state.projects.find((p) => p.id === projectId)?.name || "Unknown";
    },
    [state.projects],
  );

  const contextValue: AppContextType = {
    ...state,
    createThread,
    selectThread,
    closeChat,
    sendMessage,
    addProject,
    selectProject,
    getProjectName,
  };

  return (
    <AppContext.Provider value={contextValue}>{children}</AppContext.Provider>
  );
}

// ============ HOOK ============

export function useAppState() {
  const context = useContext(AppContext);
  if (!context) {
    throw new Error("useAppState must be used within AppProvider");
  }
  return context;
}

function generateMockTrace(
  userMessage: string,
  messageId: string,
): ExecutionTraceItem[] {
  const lower = userMessage.toLowerCase();
  const timestamp = Date.now();

  if (lower.includes("read") || lower.includes("inspect") || lower.includes("search")) {
    const toolCallId = `tool-${timestamp}`;
    return [
      {
        id: `${toolCallId}-requested`,
        messageId,
        toolCallId,
        toolName: lower.includes("search") ? "graph_retrieve_code" : "read_file",
        toolKind: lower.includes("search") ? "retrieval" : "filesystem",
        phase: "requested",
        status: "pending",
        parameters: lower.includes("search")
          ? { query: userMessage }
          : { path: "src/main.rs" },
        requiresApproval: false,
        timestamp,
      },
      {
        id: `${toolCallId}-completed`,
        messageId,
        toolCallId,
        toolName: lower.includes("search") ? "graph_retrieve_code" : "read_file",
        toolKind: lower.includes("search") ? "retrieval" : "filesystem",
        phase: "completed",
        status: "complete",
        parameters: lower.includes("search")
          ? { query: userMessage }
          : { path: "src/main.rs" },
        result: lower.includes("search")
          ? "Retrieved matching code context."
          : "Read src/main.rs successfully.",
        requiresApproval: false,
        timestamp: timestamp + 1,
      },
    ];
  }

  return [];
}
