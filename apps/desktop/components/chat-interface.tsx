"use client";

import { useState, useRef, useEffect } from "react";
import { useAppState } from "@/lib/app-state";
import {
  Send,
  Paperclip,
  Sparkles,
  Zap,
  ChevronDown,
  Terminal,
  GitBranch,
} from "lucide-react";
import { cn } from "@/lib/utils";

export function ChatInterface() {
  const { currentThreadId, threads, sendMessage, projects, currentProjectId } = useAppState();

  const [input, setInput] = useState("");
  const [isTyping, setIsTyping] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Mock UI state
  const [selectedModel, setSelectedModel] = useState("GPT-5.4");
  const [selectedEffort, setSelectedEffort] = useState("Medium");
  const [selectedRuntime, setSelectedRuntime] = useState("Local");
  const [selectedAccess, setSelectedAccess] = useState("Full access");
  const [selectedBranch, setSelectedBranch] = useState("main");

  // Dropdown open states
  const [modelOpen, setModelOpen] = useState(false);
  const [effortOpen, setEffortOpen] = useState(false);
  const [runtimeOpen, setRuntimeOpen] = useState(false);
  const [accessOpen, setAccessOpen] = useState(false);
  const [branchOpen, setBranchOpen] = useState(false);
  const [projectOpen, setProjectOpen] = useState(false);

  const currentThread = threads.find((t) => t.id === currentThreadId);
  const currentProject = projects.find((p) => p.id === currentProjectId);

  // Close all dropdowns
  const closeAllDropdowns = () => {
    setModelOpen(false);
    setEffortOpen(false);
    setRuntimeOpen(false);
    setAccessOpen(false);
    setBranchOpen(false);
    setProjectOpen(false);
  };

  // Close dropdowns when clicking outside
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      if (!target.closest('[data-dropdown]')) {
        closeAllDropdowns();
      }
    };

    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  // Auto-scroll to bottom
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [currentThread?.messages]);

  // Auto-resize textarea
  useEffect(() => {
    if (textareaRef.current) {
      textareaRef.current.style.height = "auto";
      textareaRef.current.style.height = `${Math.min(textareaRef.current.scrollHeight, 200)}px`;
    }
  }, [input]);

  const handleSubmit = async (e?: React.FormEvent) => {
    e?.preventDefault();

    if (!input.trim() || !currentThreadId) return;

    const message = input.trim();
    setInput("");
    setIsTyping(true);

    try {
      await sendMessage(message);
    } finally {
      setIsTyping(false);
    }

    if (textareaRef.current) {
      textareaRef.current.style.height = "auto";
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  };

  return (
    <>
      {/* Messages */}
      <div className="flex-1 overflow-y-auto pb-64 custom-scrollbar" data-no-drag="true">
        {currentThread?.messages.length === 0 ? (
          <div className="w-full flex flex-col items-center text-center px-6 pt-32">
            <div className="w-full max-w-3xl">
              <div className="w-12 h-12 mb-6 mx-auto">
                <svg
                  viewBox="0 0 24 24"
                  fill="none"
                  xmlns="http://www.w3.org/2000/svg"
                  className="w-full h-full text-foreground"
                >
                  <path
                    d="M12 2C6.47715 2 2 6.47715 2 12C2 17.5228 6.47715 22 12 22C17.5228 22 22 17.5228 22 12C22 6.47715 17.5228 2 12 2Z"
                    stroke="currentColor"
                    strokeWidth="1.5"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeDasharray="2 4"
                  />
                  <path
                    d="M8 12C8 9.79086 9.79086 8 12 8C14.2091 8 16 9.79086 16 12C16 14.2091 14.2091 16 12 16C9.79086 16 8 14.2091 8 12Z"
                    fill="currentColor"
                  />
                </svg>
              </div>
              <h2 className="text-[28px] font-semibold tracking-tight mb-2 text-foreground">
                Let&apos;s build
              </h2>

              {/* Project Selector */}
              <div className="relative inline-block mb-12" data-dropdown>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    closeAllDropdowns();
                    setProjectOpen(!projectOpen);
                  }}
                  className="appearance-none bg-transparent hover:bg-accent/50 text-muted-foreground py-1.5 pl-3 pr-8 rounded-lg focus:outline-none focus:ring-0 text-sm font-medium cursor-pointer transition-all flex items-center gap-2"
                >
                  <span className="truncate max-w-[200px]">
                    {currentProject?.name || "Select Project"}
                  </span>
                  <ChevronDown
                    className={cn(
                      "w-3.5 h-3.5 transition-transform",
                      projectOpen && "rotate-180"
                    )}
                  />
                </button>

                {projectOpen && (
                  <div className="absolute top-full left-0 mt-1 bg-panel/95 backdrop-blur-xl border border-border/60 rounded-lg shadow-lg py-1 z-50 min-w-[200px]" data-dropdown>
                    {projects.map((project) => (
                      <button
                        key={project.id}
                        onClick={(e) => {
                          e.stopPropagation();
                          setProjectOpen(false);
                        }}
                        className={cn(
                          "w-full text-left px-3 py-1.5 text-sm hover:bg-accent transition-colors",
                          currentProjectId === project.id
                            ? "bg-accent/30 text-foreground"
                            : "text-muted-foreground"
                        )}
                      >
                        {project.name}
                      </button>
                    ))}
                  </div>
                )}
              </div>

              <div className="grid grid-cols-1 md:grid-cols-3 gap-3 w-full">
                <div className="bg-muted/40 hover:bg-accent/80 border border-transparent p-4 rounded-2xl text-left cursor-pointer transition-all group">
                  <div className="w-8 h-8 rounded-lg bg-muted/80 flex items-center justify-center mb-3 transition-colors group-hover:bg-accent">
                    <Sparkles className="w-4 h-4 text-foreground/70" />
                  </div>
                  <h3 className="text-[13px] font-medium text-foreground/90 mb-1">
                    Start a new project
                  </h3>
                  <p className="text-[12px] text-muted-foreground leading-relaxed group-hover:text-foreground/70 max-w-[90%]">
                    Begin building with AI assistance from scratch.
                  </p>
                </div>

                <div className="bg-muted/40 hover:bg-accent/80 border border-transparent p-4 rounded-2xl text-left cursor-pointer transition-all group">
                  <div className="w-8 h-8 rounded-lg bg-muted/80 flex items-center justify-center mb-3 transition-colors group-hover:bg-accent">
                    <Terminal className="w-4 h-4 text-[#ff8e8b]" />
                  </div>
                  <h3 className="text-[13px] font-medium text-foreground/90 mb-1">
                    Debug an issue
                  </h3>
                  <p className="text-[12px] text-muted-foreground leading-relaxed group-hover:text-foreground/70 max-w-[90%]">
                    Get help fixing bugs and errors in your code.
                  </p>
                </div>

                <div className="bg-muted/40 hover:bg-accent/80 border border-transparent p-4 rounded-2xl text-left cursor-pointer transition-all group">
                  <div className="w-8 h-8 rounded-lg bg-muted/80 flex items-center justify-center mb-3 transition-colors group-hover:bg-accent">
                    <GitBranch className="w-4 h-4 text-[#ffd166]" />
                  </div>
                  <h3 className="text-[13px] font-medium text-foreground/90 mb-1">
                    Review code changes
                  </h3>
                  <p className="text-[12px] text-muted-foreground leading-relaxed group-hover:text-foreground/70 max-w-[90%]">
                    Understand and improve your recent commits.
                  </p>
                </div>
              </div>
            </div>
          </div>
        ) : (
          <div className="w-full flex flex-col items-center text-center px-6 pt-32">
            <div className="w-full max-w-3xl space-y-6">
              {currentThread?.messages.map((message) => (
                <div
                  key={message.id}
                  className={cn(
                    "flex gap-4",
                    message.role === "user" ? "flex-row-reverse" : ""
                  )}
                >
                  {/* Avatar */}
                  <div
                    className={cn(
                      "w-8 h-8 rounded-full flex items-center justify-center shrink-0",
                      message.role === "user"
                        ? "bg-primary text-primary-foreground"
                        : "bg-accent text-foreground"
                    )}
                  >
                    {message.role === "user" ? (
                      <span className="text-[12px] font-medium">U</span>
                    ) : (
                      <Sparkles className="w-4 h-4" />
                    )}
                  </div>

                  {/* Message */}
                  <div
                    className={cn(
                      "flex-1 max-w-[80%]",
                      message.role === "user" ? "text-right" : ""
                    )}
                  >
                    <div
                      className={cn(
                        "inline-block px-4 py-2.5 rounded-2xl text-[14px] leading-relaxed text-left",
                        message.role === "user"
                          ? "bg-primary text-primary-foreground"
                          : "bg-muted text-foreground"
                      )}
                    >
                      {message.content}
                    </div>
                    <div
                      className={cn(
                        "text-[11px] text-muted-foreground mt-1.5",
                        message.role === "user" ? "text-right" : "text-left"
                      )}
                    >
                      {new Date(message.timestamp).toLocaleTimeString([], {
                        hour: "2-digit",
                        minute: "2-digit",
                      })}
                    </div>
                  </div>
                </div>
              ))}

              {isTyping && (
                <div className="flex gap-4">
                  <div className="w-8 h-8 rounded-full bg-accent text-foreground flex items-center justify-center shrink-0">
                    <Sparkles className="w-4 h-4" />
                  </div>
                  <div className="bg-muted px-4 py-3 rounded-2xl">
                    <div className="flex gap-1.5">
                      <div
                        className="w-2 h-2 bg-muted-foreground/50 rounded-full animate-bounce"
                        style={{ animationDelay: "0ms" }}
                      />
                      <div
                        className="w-2 h-2 bg-muted-foreground/50 rounded-full animate-bounce"
                        style={{ animationDelay: "150ms" }}
                      />
                      <div
                        className="w-2 h-2 bg-muted-foreground/50 rounded-full animate-bounce"
                        style={{ animationDelay: "300ms" }}
                      />
                    </div>
                  </div>
                </div>
              )}

              <div ref={messagesEndRef} />
            </div>
          </div>
        )}
      </div>

      {/* Input with Mock Dropdowns */}
      <div className="absolute bottom-0 left-0 right-0 flex flex-col items-center p-4 bg-gradient-to-t from-background via-background to-transparent pointer-events-none pb-6">
        <div className="w-full max-w-3xl px-6 pointer-events-auto">
          <div className="bg-panel/95 backdrop-blur-xl border border-border shadow-lg rounded-[20px] p-2.5 focus-within:border-border/80 transition-all">
            <textarea
              ref={textareaRef}
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={handleKeyDown}
              className="w-full bg-transparent border-none focus:outline-none focus:ring-0 text-foreground/90 placeholder-muted-foreground/70 text-[15px] resize-none min-h-[44px] p-2 custom-scrollbar"
              placeholder="Ask Axora anything, @ to add files, / for commands"
              rows={1}
            />

            <div className="flex items-center justify-between px-1 pt-1 mt-1">
              <div className="flex items-center gap-1.5">
                {/* Model Selector */}
                <div className="relative" data-dropdown>
                  <button
                    type="button"
                    onClick={(e) => {
                      e.stopPropagation();
                      closeAllDropdowns();
                      setModelOpen(!modelOpen);
                    }}
                    className="flex items-center gap-1.5 px-2 py-1.5 text-muted-foreground text-[11px] font-medium transition-all rounded-md hover:bg-accent/50 hover:text-foreground"
                  >
                    <Sparkles className="w-3.5 h-3.5 text-muted-foreground" />
                    {selectedModel}
                    <ChevronDown
                      className={cn(
                        "w-3 h-3 ml-0.5 opacity-50 transition-transform",
                        modelOpen && "rotate-180"
                      )}
                    />
                  </button>
                  {modelOpen && (
                    <div className="absolute bottom-full left-0 mb-1 bg-panel/95 backdrop-blur-xl border border-border/60 rounded-lg shadow-lg py-1 z-50 min-w-[180px]" data-dropdown>
                      {["GPT-5.4", "GPT-4.5", "Claude 3.7", "Gemini 2.0"].map(
                        (model) => (
                          <button
                            key={model}
                            onClick={(e) => {
                              e.stopPropagation();
                              setSelectedModel(model);
                              setModelOpen(false);
                            }}
                            className={cn(
                              "w-full text-left px-3 py-1.5 text-xs hover:bg-accent transition-colors",
                              selectedModel === model
                                ? "bg-accent/30 text-foreground"
                                : "text-muted-foreground"
                            )}
                          >
                            {model}
                          </button>
                        )
                      )}
                    </div>
                  )}
                </div>

                {/* Effort Selector */}
                <div className="relative" data-dropdown>
                  <button
                    type="button"
                    onClick={(e) => {
                      e.stopPropagation();
                      closeAllDropdowns();
                      setEffortOpen(!effortOpen);
                    }}
                    className="flex items-center gap-1.5 px-2 py-1.5 text-muted-foreground text-[11px] font-medium transition-all rounded-md hover:bg-accent/50 hover:text-foreground"
                  >
                    <Zap className="w-3 h-3" />
                    {selectedEffort}
                    <ChevronDown
                      className={cn(
                        "w-3 h-3 ml-0.5 opacity-50 transition-transform",
                        effortOpen && "rotate-180"
                      )}
                    />
                  </button>
                  {effortOpen && (
                    <div className="absolute bottom-full left-0 mb-1 bg-panel/95 backdrop-blur-xl border border-border/60 rounded-lg shadow-lg py-1 z-50 min-w-[140px]" data-dropdown>
                      {["Low", "Medium", "High", "Maximum"].map((effort) => (
                        <button
                          key={effort}
                          onClick={(e) => {
                            e.stopPropagation();
                            setSelectedEffort(effort);
                            setEffortOpen(false);
                          }}
                          className={cn(
                            "w-full text-left px-3 py-1.5 text-xs hover:bg-accent transition-colors",
                            selectedEffort === effort
                              ? "bg-accent/30 text-foreground"
                              : "text-muted-foreground"
                          )}
                        >
                          {effort}
                        </button>
                      ))}
                    </div>
                  )}
                </div>
              </div>

              <div className="flex items-center gap-2">
                <button
                  type="button"
                  className="p-1.5 text-muted-foreground hover:text-foreground transition-colors rounded-lg hover:bg-accent"
                >
                  <Paperclip className="w-[18px] h-[18px]" strokeWidth={2} />
                </button>
                <button
                  type="submit"
                  disabled={!input.trim() || isTyping}
                  className="w-[30px] h-[30px] rounded-full bg-primary text-primary-foreground flex items-center justify-center hover:bg-primary/90 transition-all shadow-sm ml-1 disabled:opacity-50 disabled:cursor-not-allowed"
                  onClick={(e) => {
                    e.preventDefault();
                    handleSubmit();
                  }}
                >
                  <Send className="w-[16px] h-[16px]" strokeWidth={2.5} />
                </button>
              </div>
            </div>
          </div>

          {/* Bottom Bar with Mock Dropdowns */}
          <div className="flex items-center justify-between px-2 mt-3 text-[11px] font-medium text-muted-foreground">
            <div className="flex items-center gap-4">
              {/* Runtime Selector */}
              <div className="relative" data-dropdown>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    closeAllDropdowns();
                    setRuntimeOpen(!runtimeOpen);
                  }}
                  className="flex items-center gap-1.5 cursor-pointer hover:text-foreground transition-colors"
                >
                  <Terminal className="w-3.5 h-3.5" />
                  {selectedRuntime}
                  <ChevronDown
                    className={cn(
                      "w-3 h-3 opacity-50 transition-transform",
                      runtimeOpen && "rotate-180"
                    )}
                  />
                </button>
                {runtimeOpen && (
                  <div className="absolute bottom-full left-0 mb-1 bg-panel/95 backdrop-blur-xl border border-border/60 rounded-lg shadow-lg py-1 z-50 min-w-[140px]" data-dropdown>
                    {["Local", "Cloud", "Hybrid"].map((runtime) => (
                      <button
                        key={runtime}
                        onClick={(e) => {
                          e.stopPropagation();
                          setSelectedRuntime(runtime);
                          setRuntimeOpen(false);
                        }}
                        className={cn(
                          "w-full text-left px-3 py-1.5 text-xs hover:bg-accent transition-colors",
                          selectedRuntime === runtime
                            ? "bg-accent/30 text-foreground"
                            : "text-muted-foreground"
                        )}
                      >
                        {runtime}
                      </button>
                    ))}
                  </div>
                )}
              </div>

              {/* Access Selector */}
              <div className="relative" data-dropdown>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    closeAllDropdowns();
                    setAccessOpen(!accessOpen);
                  }}
                  className={cn(
                    "flex items-center gap-1.5 cursor-pointer hover:text-foreground transition-colors",
                    selectedAccess === "Full access"
                      ? "text-[#ff8e8b] hover:text-[#ff9f9c]"
                      : ""
                  )}
                >
                  <div
                    className={cn(
                      "w-3.5 h-3.5 flex items-center justify-center rounded border text-[8px] leading-none pb-[1px]",
                      selectedAccess === "Full access"
                        ? "border-[#ff8e8b] text-[#ff8e8b]"
                        : "border-muted-foreground text-muted-foreground"
                    )}
                  >
                    !
                  </div>
                  {selectedAccess}
                  <ChevronDown
                    className={cn(
                      "w-3 h-3 opacity-50 transition-transform",
                      accessOpen && "rotate-180"
                    )}
                  />
                </button>
                {accessOpen && (
                  <div className="absolute bottom-full left-0 mb-1 bg-panel/95 backdrop-blur-xl border border-border/60 rounded-lg shadow-lg py-1 z-50 min-w-[160px]" data-dropdown>
                    {[
                      { value: "Full access", color: "#ff8e8b" },
                      { value: "Limited", color: "#ffd166" },
                      { value: "Read-only", color: "#06d6a0" },
                    ].map((access) => (
                      <button
                        key={access.value}
                        onClick={(e) => {
                          e.stopPropagation();
                          setSelectedAccess(access.value);
                          setAccessOpen(false);
                        }}
                        className={cn(
                          "w-full text-left px-3 py-1.5 text-xs hover:bg-accent transition-colors flex items-center gap-2",
                          selectedAccess === access.value
                            ? "bg-accent/30 text-foreground"
                            : "text-muted-foreground"
                        )}
                      >
                        <div
                          className={cn(
                            "w-2 h-2 rounded-full",
                            selectedAccess === access.value ? "" : "opacity-0"
                          )}
                          style={{ backgroundColor: access.color }}
                        />
                        <span
                          style={{
                            color:
                              selectedAccess === access.value
                                ? access.color
                                : undefined,
                          }}
                        >
                          {access.value}
                        </span>
                      </button>
                    ))}
                  </div>
                )}
              </div>
            </div>

            {/* Branch Selector */}
            <div className="relative" data-dropdown>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  closeAllDropdowns();
                  setBranchOpen(!branchOpen);
                }}
                className="flex items-center gap-1.5 cursor-pointer hover:text-foreground transition-colors"
              >
                <GitBranch className="w-3.5 h-3.5" />
                {selectedBranch}
                <ChevronDown
                  className={cn(
                    "w-3 h-3 opacity-50 transition-transform",
                    branchOpen && "rotate-180"
                  )}
                />
              </button>
              {branchOpen && (
                <div className="absolute bottom-full right-0 mb-1 bg-panel/95 backdrop-blur-xl border border-border/60 rounded-lg shadow-lg py-1 z-50 min-w-[140px]" data-dropdown>
                  {["main", "develop", "feature/new", "bugfix/fix"].map(
                    (branch) => (
                      <button
                        key={branch}
                        onClick={(e) => {
                          e.stopPropagation();
                          setSelectedBranch(branch);
                          setBranchOpen(false);
                        }}
                        className={cn(
                          "w-full text-left px-3 py-1.5 text-xs hover:bg-accent transition-colors",
                          selectedBranch === branch
                            ? "bg-accent/30 text-foreground"
                            : "text-muted-foreground"
                        )}
                      >
                        {branch}
                      </button>
                    )
                  )}
                </div>
              )}
            </div>
          </div>
        </div>
      </div>
    </>
  );
}
