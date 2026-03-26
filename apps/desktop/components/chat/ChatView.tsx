"use client";

import { useCallback, useMemo, useState } from "react";
import { useAppState } from "@/lib/app-state";
import { threadMessagesToContracts } from "@/lib/chat-message-mapper";
import { cn } from "@/lib/utils";
import { ChevronDown, GitBranch, Sparkles, Terminal } from "lucide-react";
import { ChatConversation } from "./ChatConversation";
import { ChatPlanPanel } from "./ChatPlanPanel";
import { ChatPromptBar } from "./ChatPromptBar";
import { ChatQueuePanel } from "./ChatQueuePanel";

export function ChatView({ className }: { className?: string }) {
  const { currentThreadId, threads, sendMessage, projects, currentProjectId } =
    useAppState();

  const [projectOpen, setProjectOpen] = useState(false);

  const currentThread = threads.find((t) => t.id === currentThreadId);
  const currentProject = projects.find((p) => p.id === currentProjectId);

  const uiMessages = useMemo(
    () => (currentThread ? threadMessagesToContracts(currentThread) : []),
    [currentThread],
  );

  const handleCopy = useCallback((content: string) => {
    console.log("Copied to clipboard:", content);
  }, []);

  const handleRetry = useCallback(() => {
    console.log("Retry last message");
  }, []);

  const handleFeedback = useCallback((feedback: "like" | "dislike") => {
    console.log("Feedback:", feedback);
  }, []);

  const emptyState =
    uiMessages.length === 0 ? (
      <div className="flex w-full flex-col items-center px-6 pt-24 text-center">
        <div className="w-full max-w-3xl">
          <div className="text-foreground mx-auto mb-6 flex size-12 items-center justify-center">
            <Sparkles className="size-8" />
          </div>
          <h2 className="text-foreground mb-2 text-[28px] font-semibold tracking-tight">
            Let&apos;s build
          </h2>
          <div className="relative mb-10 inline-block">
            <button
              type="button"
              className="text-muted-foreground hover:bg-accent/50 flex items-center gap-2 rounded-lg py-1.5 pr-8 pl-3 text-sm font-medium transition-all"
              onClick={() => setProjectOpen((o) => !o)}
            >
              <span className="max-w-[200px] truncate">
                {currentProject?.name ?? "Select project"}
              </span>
              <ChevronDown
                className={cn(
                  "size-3.5 transition-transform",
                  projectOpen && "rotate-180",
                )}
              />
            </button>
            {projectOpen && (
              <div className="bg-panel/95 border-border/60 absolute top-full left-0 z-50 mt-1 min-w-[200px] rounded-lg border py-1 shadow-lg backdrop-blur-xl">
                {projects.map((project) => (
                  <button
                    type="button"
                    key={project.id}
                    className="text-muted-foreground hover:bg-accent w-full px-3 py-1.5 text-left text-sm transition-colors"
                    onClick={() => setProjectOpen(false)}
                  >
                    {project.name}
                  </button>
                ))}
              </div>
            )}
          </div>
          <div className="grid w-full grid-cols-1 gap-3 md:grid-cols-3">
            <div className="bg-muted/40 hover:bg-accent/80 group cursor-pointer rounded-2xl border border-transparent p-4 text-left transition-all">
              <div className="bg-muted/80 group-hover:bg-accent mb-3 flex size-8 items-center justify-center rounded-lg transition-colors">
                <Sparkles className="text-foreground/70 size-4" />
              </div>
              <h3 className="text-foreground/90 mb-1 text-[13px] font-medium">
                Start a new project
              </h3>
              <p className="text-muted-foreground group-hover:text-foreground/70 max-w-[90%] text-[12px] leading-relaxed">
                Begin building with AI assistance from scratch.
              </p>
            </div>
            <div className="bg-muted/40 hover:bg-accent/80 group cursor-pointer rounded-2xl border border-transparent p-4 text-left transition-all">
              <div className="bg-muted/80 group-hover:bg-accent mb-3 flex size-8 items-center justify-center rounded-lg transition-colors">
                <Terminal className="size-4 text-[#ff8e8b]" />
              </div>
              <h3 className="text-foreground/90 mb-1 text-[13px] font-medium">
                Debug an issue
              </h3>
              <p className="text-muted-foreground group-hover:text-foreground/70 max-w-[90%] text-[12px] leading-relaxed">
                Get help fixing bugs and errors in your code.
              </p>
            </div>
            <div className="bg-muted/40 hover:bg-accent/80 group cursor-pointer rounded-2xl border border-transparent p-4 text-left transition-all">
              <div className="bg-muted/80 group-hover:bg-accent mb-3 flex size-8 items-center justify-center rounded-lg transition-colors">
                <GitBranch className="size-4 text-[#ffd166]" />
              </div>
              <h3 className="text-foreground/90 mb-1 text-[13px] font-medium">
                Review code changes
              </h3>
              <p className="text-muted-foreground group-hover:text-foreground/70 max-w-[90%] text-[12px] leading-relaxed">
                Understand and improve your recent commits.
              </p>
            </div>
          </div>
        </div>
      </div>
    ) : undefined;

  return (
    <div
      className={cn("flex min-h-0 flex-1 flex-col bg-background", className)}
      data-no-drag="true"
    >
      <div className="flex min-h-0 flex-1">
        <div className="flex min-w-0 flex-1 flex-col">
          <ChatConversation
            messages={uiMessages}
            emptyState={emptyState}
            onCopy={handleCopy}
            onRetry={handleRetry}
            onFeedback={handleFeedback}
          />
          <ChatPromptBar
            disabled={!currentThreadId}
            onSubmit={(text) => sendMessage(text)}
          />
        </div>

        <aside className="bg-muted/10 border-border/60 flex w-[min(340px,32vw)] shrink-0 flex-col gap-3 overflow-y-auto border-l p-3 custom-scrollbar">
          <ChatPlanPanel />
          <ChatQueuePanel />
        </aside>
      </div>
    </div>
  );
}
