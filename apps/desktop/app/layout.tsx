"use client";

import {
  type ChangeEvent,
  type InputHTMLAttributes,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  Settings,
  FolderOpen,
  ChevronDown,
  ChevronRight,
  Gamepad2,
  FileText,
  PencilRuler,
  Sparkles,
  Zap,
  Terminal as TerminalIcon,
  Paperclip,
  ArrowUp,
  GitBranch,
  Clock,
  LayoutGrid,
  SquarePen,
  ArrowLeft,
  Palette,
  Wrench,
  User,
  BarChart,
  Database,
  GitBranch as GitIcon,
  Terminal,
  FolderGit,
  Archive,
  Check,
  Globe,
  Gauge,
  LogOut,
  ExternalLink,
  AtSign,
  Briefcase,
  Search,
  FolderPlus,
  MessageSquareText,
} from "lucide-react";

import type { DesktopPreferences } from "@/shared/contracts/desktop";
import { defaultPreferences } from "@/shared/contracts/desktop";
import { desktopService } from "@/lib/services/desktop-service";
import { cn } from "@/lib/utils";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarProvider,
  SidebarGroup,
  SidebarGroupAction,
  SidebarGroupLabel,
  SidebarGroupContent,
  SidebarMenu,
  SidebarMenuItem,
  SidebarMenuButton,
  SidebarMenuAction,
  SidebarMenuSub,
  SidebarMenuSubButton,
  SidebarMenuSubItem,
  SidebarInset,
  useSidebar,
} from "@/components/ui/sidebar";
import { ThemeProvider } from "@/components/theme-provider";
import { AppProvider, useAppState } from "@/lib/app-state";
import { ChatView } from "@/components/chat";
import { SettingsMainPanel } from "@/components/settings/settings-main-panel";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";

import "./globals.css";

// ============ TYPES ============
type ViewState = { type: "home" } | { type: "settings"; tab: string };

const settingsNavItems = [
  { id: "general", label: "General", icon: Settings },
  { id: "appearance", label: "Appearance", icon: Palette },
  { id: "configuration", label: "Configuration", icon: Wrench },
  { id: "personalization", label: "Personalization", icon: User },
  { id: "usage", label: "Usage", icon: BarChart },
  { id: "mcp", label: "MCP servers", icon: Database },
  { id: "git", label: "Git", icon: GitIcon },
  { id: "environments", label: "Environments", icon: Terminal },
  { id: "worktrees", label: "Worktrees", icon: FolderGit },
  { id: "archived", label: "Archived threads", icon: Archive },
];

// ============ MOCK DATA ============
const MOCK_USER = {
  email: "cajilinoar@icloud.com",
  accountType: "Personal account",
};

const LANGUAGES = [
  { code: "auto", label: "Auto Detect", region: null },
  { code: "en-US", label: "English", region: "US" },
  { code: "en-GB", label: "English", region: "UK" },
  { code: "pt-BR", label: "Portuguese", region: "Brazil" },
  { code: "pt-PT", label: "Portuguese", region: "Portugal" },
  { code: "es", label: "Spanish", region: null },
  { code: "fr", label: "French", region: null },
  { code: "de", label: "German", region: null },
  { code: "it", label: "Italian", region: null },
  { code: "nl", label: "Dutch", region: null },
  { code: "da", label: "Danish", region: null },
  { code: "cs", label: "Czech", region: null },
  { code: "sq", label: "Albanian", region: null },
  { code: "hy", label: "Armenian", region: null },
  { code: "my", label: "Burmese", region: "Myanmar [Burma]" },
  { code: "ca", label: "Catalan", region: "España" },
  { code: "bs", label: "bosanski", region: "Bosnia & Herzegovina" },
];

const RATE_LIMITS = {
  shortTerm: { label: "5h", percentage: 2, resetTime: "3:27 PM" },
  weekly: { label: "Weekly", percentage: 26, resetDate: "Mar 24" },
};

// ============ SIDEBAR TOGGLE ============
function SidebarToggle() {
  const { open, toggleSidebar } = useSidebar();

  return (
    <button
      onClick={toggleSidebar}
      className={cn(
        "flex items-center justify-center rounded-[6px]",
        "text-muted-foreground hover:text-foreground hover:bg-accent",
        "w-[28px] h-[28px]",
        "transition-all duration-300 ease-[cubic-bezier(0.4,0,0.2,1)]",
      )}
      aria-label={open ? "Close sidebar" : "Open sidebar"}
      title={open ? "Close sidebar" : "Open sidebar"}
    >
      <svg
        width="16"
        height="16"
        viewBox="0 0 16 16"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
      >
        <rect
          x="2"
          y="3"
          width="12"
          height="10"
          rx="2"
          stroke="currentColor"
          strokeWidth="1.25"
        />
        <line
          x1="6.5"
          y1="3"
          x2="6.5"
          y2="13"
          stroke="currentColor"
          strokeWidth="1.25"
        />
      </svg>
    </button>
  );
}

// ============ SETTINGS DROPDOWN MENU ============
function SettingsDropdownMenu({
  onOpenSettings,
}: {
  onOpenSettings?: () => void;
}) {
  const [currentLanguage, setCurrentLanguage] = useState("auto");
  const [languageSearch, setLanguageSearch] = useState("");
  const [open, setOpen] = useState(false);
  const [activeSubmenu, setActiveSubmenu] = useState<
    "language" | "rateLimits" | null
  >(null);

  const filteredLanguages = LANGUAGES.filter((lang) => {
    const searchTerm = languageSearch.toLowerCase();
    const fullLabel = lang.region
      ? `${lang.label} (${lang.region})`
      : lang.label;
    return (
      fullLabel.toLowerCase().includes(searchTerm) ||
      lang.code.toLowerCase().includes(searchTerm)
    );
  });

  const currentLangLabel = LANGUAGES.find(
    (l) => l.code === currentLanguage,
  )?.label;

  const handleLanguageSelect = (code: string) => {
    setCurrentLanguage(code);
    setActiveSubmenu(null);
  };

  const handleBackToMain = () => {
    setActiveSubmenu(null);
    setLanguageSearch("");
  };

  return (
    <DropdownMenu open={open} onOpenChange={setOpen}>
      <DropdownMenuTrigger asChild>
        <button
          suppressHydrationWarning
          className={cn(
            "flex items-center gap-2 w-full h-[28px] px-2.5 rounded-[8px]",
            "text-foreground/80 hover:text-foreground",
            "hover:bg-accent",
            "transition-all",
            "data-[state=open]:bg-accent/50",
          )}
        >
          <Settings
            className="w-3.5 h-3.5 text-muted-foreground"
            strokeWidth={1.5}
          />
          <span className="text-[12px] font-medium">Settings</span>
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent
        side="top"
        align="start"
        sideOffset={8}
        className={cn(
          "w-[calc(var(--sidebar-width)-16px)] p-1.5 rounded-[12px]",
          "bg-popover border border-border/50",
          "shadow-lg",
          "animate-in fade-in-0 zoom-in-95 data-[side=top]:slide-in-from-bottom-2",
        )}
      >
        {activeSubmenu === null ? (
          /* MAIN MENU */
          <>
            {/* Account Row */}
            <div className="flex items-center gap-2.5 px-2 py-2 mb-1">
              <div className="w-7 h-7 rounded-full bg-accent flex items-center justify-center">
                <AtSign
                  className="w-3.5 h-3.5 text-muted-foreground"
                  strokeWidth={1.5}
                />
              </div>
              <div className="flex flex-col min-w-0">
                <span className="text-[12px] font-medium text-foreground truncate">
                  {MOCK_USER.email}
                </span>
                <span className="text-[11px] text-muted-foreground">
                  {MOCK_USER.accountType}
                </span>
              </div>
            </div>

            {/* Personal Account */}
            <button
              onClick={() => console.log("Personal account clicked")}
              className={cn(
                "flex items-center gap-2 w-full h-[28px] px-2 rounded-[8px]",
                "text-foreground/80 hover:text-foreground",
                "hover:bg-accent",
                "transition-colors",
                "text-left",
              )}
            >
              <Briefcase
                className="w-3.5 h-3.5 text-muted-foreground"
                strokeWidth={1.5}
              />
              <span className="text-[12px] font-medium">Personal account</span>
            </button>

            <DropdownMenuSeparator className="my-1.5 h-px bg-border/50" />

            {/* Settings */}
            <button
              onClick={() => {
                console.log("Settings clicked");
                onOpenSettings?.();
              }}
              className={cn(
                "flex items-center gap-2 w-full h-[28px] px-2 rounded-[8px]",
                "text-foreground/80 hover:text-foreground",
                "hover:bg-accent",
                "transition-colors",
                "text-left",
              )}
            >
              <Settings
                className="w-3.5 h-3.5 text-muted-foreground"
                strokeWidth={1.5}
              />
              <span className="text-[12px] font-medium">Settings</span>
            </button>

            {/* Language */}
            <button
              onClick={() => setActiveSubmenu("language")}
              className={cn(
                "flex items-center gap-2 w-full h-[28px] px-2 rounded-[8px]",
                "text-foreground/80 hover:text-foreground",
                "hover:bg-accent",
                "transition-colors",
                "text-left",
              )}
            >
              <Globe
                className="w-3.5 h-3.5 text-muted-foreground"
                strokeWidth={1.5}
              />
              <span className="text-[12px] font-medium flex-1">Language</span>
              <span className="text-[11px] text-muted-foreground mr-1">
                {currentLangLabel}
              </span>
              <ChevronRight
                className="w-3.5 h-3.5 text-muted-foreground"
                strokeWidth={1.5}
              />
            </button>

            {/* Rate Limits */}
            <button
              onClick={() => setActiveSubmenu("rateLimits")}
              className={cn(
                "flex items-center gap-2 w-full h-[28px] px-2 rounded-[8px]",
                "text-foreground/80 hover:text-foreground",
                "hover:bg-accent",
                "transition-colors",
                "text-left",
              )}
            >
              <Gauge
                className="w-3.5 h-3.5 text-muted-foreground"
                strokeWidth={1.5}
              />
              <span className="text-[12px] font-medium flex-1">
                Rate limits remaining
              </span>
              <span className="text-[11px] text-muted-foreground mr-1">
                {RATE_LIMITS.shortTerm.percentage}%
              </span>
              <ChevronRight
                className="w-3.5 h-3.5 text-muted-foreground"
                strokeWidth={1.5}
              />
            </button>

            <DropdownMenuSeparator className="my-1.5 h-px bg-border/50" />

            {/* Log Out */}
            <button
              onClick={() => console.log("Log out clicked")}
              className={cn(
                "flex items-center gap-2 w-full h-[28px] px-2 rounded-[8px]",
                "text-foreground/80 hover:text-destructive",
                "hover:bg-accent",
                "transition-colors",
                "text-left",
              )}
            >
              <LogOut
                className="w-3.5 h-3.5 text-muted-foreground"
                strokeWidth={1.5}
              />
              <span className="text-[12px] font-medium">Log out</span>
            </button>
          </>
        ) : activeSubmenu === "language" ? (
          /* LANGUAGE SUBMENU */
          <>
            {/* Back Header */}
            <button
              onClick={handleBackToMain}
              className={cn(
                "flex items-center gap-2 w-full h-[28px] px-2 rounded-[8px] mb-2",
                "text-foreground/80 hover:text-foreground",
                "hover:bg-accent",
                "transition-colors",
                "text-left",
              )}
            >
              <ChevronDown
                className="w-3.5 h-3.5 text-muted-foreground rotate-90"
                strokeWidth={1.5}
              />
              <span className="text-[12px] font-medium">Language</span>
            </button>

            {/* Search Input */}
            <div className="px-1.5 pb-1.5">
              <div className="relative">
                <Search
                  className="absolute left-2 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-muted-foreground"
                  strokeWidth={1.5}
                />
                <input
                  type="text"
                  placeholder="Search languages"
                  value={languageSearch}
                  onChange={(e) => setLanguageSearch(e.target.value)}
                  className={cn(
                    "w-full h-[26px] pl-7 pr-2 text-[12px]",
                    "bg-accent/50 border-0 rounded-[6px]",
                    "placeholder:text-muted-foreground/60",
                    "focus:outline-none focus:ring-0",
                    "text-foreground",
                  )}
                />
              </div>
            </div>

            {/* Language List */}
            <div className="max-h-[240px] overflow-y-auto">
              {filteredLanguages.map((lang) => {
                const isSelected = currentLanguage === lang.code;
                const displayLabel = lang.region
                  ? `${lang.label} (${lang.region})`
                  : lang.label;

                return (
                  <button
                    key={lang.code}
                    onClick={() => handleLanguageSelect(lang.code)}
                    className={cn(
                      "flex items-center justify-between w-full h-[28px] px-2 rounded-[8px]",
                      "text-foreground/80 hover:text-foreground",
                      "hover:bg-accent",
                      "transition-colors",
                      "text-left",
                      isSelected && "bg-accent/30",
                    )}
                  >
                    <span className="text-[12px]">{displayLabel}</span>
                    {isSelected && (
                      <Check
                        className="w-3.5 h-3.5 text-muted-foreground"
                        strokeWidth={1.5}
                      />
                    )}
                  </button>
                );
              })}
            </div>
          </>
        ) : (
          /* RATE LIMITS SUBMENU */
          <>
            {/* Back Header */}
            <button
              onClick={handleBackToMain}
              className={cn(
                "flex items-center gap-2 w-full h-[28px] px-2 rounded-[8px] mb-2",
                "text-foreground/80 hover:text-foreground",
                "hover:bg-accent",
                "transition-colors",
                "text-left",
              )}
            >
              <ChevronDown
                className="w-3.5 h-3.5 text-muted-foreground rotate-90"
                strokeWidth={1.5}
              />
              <span className="text-[12px] font-medium">
                Rate limits remaining
              </span>
            </button>

            {/* Short Term */}
            <div className="flex items-center justify-between px-2 py-1.5">
              <span className="text-[12px] text-foreground">
                {RATE_LIMITS.shortTerm.label}
              </span>
              <div className="flex items-center gap-2">
                <span className="text-[11px] text-muted-foreground">
                  {RATE_LIMITS.shortTerm.percentage}%
                </span>
                <span className="text-[11px] text-muted-foreground">
                  {RATE_LIMITS.shortTerm.resetTime}
                </span>
              </div>
            </div>

            {/* Weekly */}
            <div className="flex items-center justify-between px-2 py-1.5">
              <span className="text-[12px] text-foreground">
                {RATE_LIMITS.weekly.label}
              </span>
              <div className="flex items-center gap-2">
                <span className="text-[11px] text-muted-foreground">
                  {RATE_LIMITS.weekly.percentage}%
                </span>
                <span className="text-[11px] text-muted-foreground">
                  {RATE_LIMITS.weekly.resetDate}
                </span>
              </div>
            </div>

            <DropdownMenuSeparator className="my-1.5 h-px bg-border/50" />

            {/* Upgrade to Pro */}
            <button
              onClick={() => console.log("Upgrade to Pro clicked")}
              className={cn(
                "flex items-center justify-between w-full h-[28px] px-2 rounded-[8px]",
                "text-foreground/80 hover:text-foreground",
                "hover:bg-accent",
                "transition-colors",
                "text-left",
              )}
            >
              <span className="text-[12px]">Upgrade to Pro</span>
              <ExternalLink
                className="w-3 h-3 text-muted-foreground"
                strokeWidth={1.5}
              />
            </button>

            {/* Learn more */}
            <button
              onClick={() => console.log("Learn more clicked")}
              className={cn(
                "flex items-center justify-between w-full h-[28px] px-2 rounded-[8px]",
                "text-foreground/80 hover:text-foreground",
                "hover:bg-accent",
                "transition-colors",
                "text-left",
              )}
            >
              <span className="text-[12px]">Learn more</span>
              <ExternalLink
                className="w-3 h-3 text-muted-foreground"
                strokeWidth={1.5}
              />
            </button>
          </>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

// ============ HOME SIDEBAR ============
function HomeSidebar({ onOpenSettings }: { onOpenSettings: () => void }) {
  const {
    threads,
    projects,
    currentThreadId,
    currentProjectId,
    createThread,
    selectThread,
    addProject,
  } = useAppState();
  const [userOpenProjectIds, setUserOpenProjectIds] = useState<string[]>([]);
  const folderInputRef = useRef<HTMLInputElement | null>(null);

  const activeProjectId = useMemo(() => {
    if (currentThreadId) {
      return (
        threads.find((thread) => thread.id === currentThreadId)?.projectId ??
        null
      );
    }
    return currentProjectId;
  }, [currentProjectId, currentThreadId, threads]);

  const projectsWithThreads = useMemo(
    () =>
      projects.map((project) => ({
        project,
        threads: threads
          .filter((thread) => thread.projectId === project.id)
          .sort((a, b) => b.updatedAt - a.updatedAt),
      })),
    [projects, threads],
  );

  /** Ensures the active project row is expanded without syncing via an effect (see react-hooks/set-state-in-effect). */
  const openProjectIds = useMemo(() => {
    if (!activeProjectId) return userOpenProjectIds;
    if (userOpenProjectIds.includes(activeProjectId)) return userOpenProjectIds;
    return [activeProjectId, ...userOpenProjectIds].slice(0, 4);
  }, [activeProjectId, userOpenProjectIds]);

  const openProject = (projectId: string) => {
    setUserOpenProjectIds((prev) =>
      prev.includes(projectId) ? prev : [projectId, ...prev].slice(0, 4),
    );
  };

  const toggleProject = (projectId: string) => {
    setUserOpenProjectIds((prev) =>
      prev.includes(projectId)
        ? prev.filter((id) => id !== projectId)
        : [projectId, ...prev].slice(0, 4),
    );
  };

  const handleCreateThread = (projectId: string) => {
    createThread(projectId);
    openProject(projectId);
  };

  const handleSelectThread = (projectId: string, threadId: string) => {
    selectThread(threadId);
    openProject(projectId);
  };

  const handleAddProject = async () => {
    const picker = (
      window as Window & {
        showDirectoryPicker?: () => Promise<{ name: string }>;
      }
    ).showDirectoryPicker;
    if (picker) {
      try {
        const handle = await picker();
        const projectName = handle.name?.trim() || "New project";
        const projectId = addProject({
          name: projectName,
          path: `~/Projects/${projectName}`,
          icon: "folder",
        });
        openProject(projectId);
        return;
      } catch {
        // User cancelled the picker.
      }
    }

    folderInputRef.current?.click();
  };

  const handleFolderInputChange = (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;

    const relativePath = file.webkitRelativePath || file.name;
    const projectName =
      relativePath.split("/")[0] || file.name || "New project";
    const projectPath =
      (file as File & { path?: string }).path ?? `~/Projects/${projectName}`;
    const projectId = addProject({
      name: projectName,
      path: projectPath,
      icon: "folder",
    });

    openProject(projectId);
    event.currentTarget.value = "";
  };

  return (
    <>
      <input
        ref={folderInputRef}
        type="file"
        className="fixed left-0 top-0 size-px opacity-0 pointer-events-none"
        onChange={handleFolderInputChange}
        {...({
          webkitdirectory: "",
          directory: "",
        } as InputHTMLAttributes<HTMLInputElement>)}
      />

      <SidebarContent className="px-3 pt-14">
        <SidebarGroup className="p-0 mb-1">
          <SidebarGroupLabel className="px-2.5 text-xs font-semibold text-muted-foreground/60 tracking-wide mb-1">
            Quick actions
          </SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenu className="gap-0">
              <SidebarMenuItem>
                <SidebarMenuButton
                  onClick={() => createThread()}
                  className="h-[28px] px-2.5 rounded-[8px] hover:bg-accent text-foreground transition-all"
                >
                  <SquarePen
                    className="w-3.5 h-3.5 text-muted-foreground"
                    strokeWidth={1.5}
                  />
                  <span className="text-[12px] font-medium">New thread</span>
                  <span className="ml-auto text-[10px] bg-muted-foreground/20 px-1.5 py-0.5 rounded text-muted-foreground border border-border/50">
                    ⌘N
                  </span>
                </SidebarMenuButton>
              </SidebarMenuItem>
              <SidebarMenuItem>
                <SidebarMenuButton className="h-[28px] px-2.5 rounded-[8px] hover:bg-accent text-foreground/80 hover:text-foreground transition-all">
                  <Clock
                    className="w-3.5 h-3.5 text-muted-foreground"
                    strokeWidth={1.5}
                  />
                  <span className="text-[12px] font-medium">Automations</span>
                </SidebarMenuButton>
              </SidebarMenuItem>
              <SidebarMenuItem>
                <SidebarMenuButton className="h-[28px] px-2.5 rounded-[8px] hover:bg-accent text-foreground/80 hover:text-foreground transition-all">
                  <LayoutGrid
                    className="w-3.5 h-3.5 text-muted-foreground"
                    strokeWidth={1.5}
                  />
                  <span className="text-[12px] font-medium">Skills</span>
                </SidebarMenuButton>
              </SidebarMenuItem>
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>

        <SidebarGroup className="p-0 mt-3">
          <div className="mb-1 flex items-center justify-between px-2.5">
            <SidebarGroupLabel className="h-auto px-0 text-xs font-semibold tracking-wide text-muted-foreground/60">
              Threads
            </SidebarGroupLabel>
            <SidebarGroupAction
              type="button"
              aria-label="Add project"
              title="Add project"
              className="static right-auto top-auto size-6 rounded-[6px] text-muted-foreground/60"
              onClick={handleAddProject}
            >
              <FolderPlus className="size-4" />
            </SidebarGroupAction>
          </div>

          <SidebarGroupContent>
            <SidebarMenu className="gap-0">
              {projectsWithThreads.map(
                ({ project, threads: projectThreads }) => {
                  const isExpanded = openProjectIds.includes(project.id);
                  const isActive =
                    activeProjectId === project.id && !isExpanded;

                  return (
                    <SidebarMenuItem key={project.id} suppressHoverGroup>
                      <div className="group/project-row relative">
                        <SidebarMenuButton
                          isActive={isActive}
                          onClick={() => toggleProject(project.id)}
                          className={cn(
                            "h-[28px] px-2.5 rounded-[8px] hover:bg-accent group-hover/project-row:bg-accent transition-all group-has-[[data-sidebar=menu-action]]/project-row:pr-8",
                            isActive
                              ? "text-foreground bg-accent/30"
                              : "text-foreground/80 hover:text-foreground",
                          )}
                        >
                          <span className="flex size-3.5 items-center justify-center">
                            <FolderOpen className="size-3.5 text-muted-foreground group-hover/project-row:hidden" />
                            {isExpanded ? (
                              <ChevronDown className="hidden size-3.5 text-muted-foreground group-hover/project-row:block" />
                            ) : (
                              <ChevronRight className="hidden size-3.5 text-muted-foreground group-hover/project-row:block" />
                            )}
                          </span>
                          <span className="min-w-0 flex-1 truncate text-[12px] font-medium">
                            {project.name}
                          </span>
                        </SidebarMenuButton>

                        <SidebarMenuAction
                          showOnHover
                          hoverScope="project-row"
                          type="button"
                          aria-label={`New thread in ${project.name}`}
                          title={`New thread in ${project.name}`}
                          className="!top-1/2 !-translate-y-1/2"
                          onClick={(event) => {
                            event.stopPropagation();
                            handleCreateThread(project.id);
                          }}
                        >
                          <SquarePen className="size-3.5" strokeWidth={1.75} />
                        </SidebarMenuAction>
                      </div>

                      {isExpanded ? (
                        <SidebarMenuSub className="mx-0 mt-1 border-l-0 px-0 py-0">
                          {projectThreads.length === 0 ? (
                            <li className="px-2.5 py-1.5 text-[11px] text-muted-foreground/60">
                              No threads yet
                            </li>
                          ) : (
                            projectThreads.map((thread) => {
                              const isThreadActive =
                                currentThreadId === thread.id;
                              const threadLabel =
                                thread.messages.length > 0
                                  ? `${thread.messages.length} messages`
                                  : "Empty thread";

                              return (
                                <SidebarMenuSubItem key={thread.id}>
                                  <SidebarMenuSubButton
                                    asChild
                                    isActive={isThreadActive}
                                    size="sm"
                                    className={cn(
                                      "h-[28px] px-2.5 rounded-[8px] hover:bg-accent transition-all",
                                      isThreadActive
                                        ? "text-foreground bg-accent/30"
                                        : "text-foreground/80 hover:text-foreground",
                                    )}
                                  >
                                    <button
                                      type="button"
                                      onClick={() =>
                                        handleSelectThread(
                                          project.id,
                                          thread.id,
                                        )
                                      }
                                      className="flex w-full items-center gap-2 text-left"
                                    >
                                      <MessageSquareText className="size-3.5 shrink-0 text-muted-foreground" />
                                      <span className="min-w-0 flex-1 truncate text-[12px] font-medium">
                                        {thread.title}
                                      </span>
                                      <span className="shrink-0 text-[10px] text-muted-foreground/60">
                                        {threadLabel}
                                      </span>
                                    </button>
                                  </SidebarMenuSubButton>
                                </SidebarMenuSubItem>
                              );
                            })
                          )}
                        </SidebarMenuSub>
                      ) : null}
                    </SidebarMenuItem>
                  );
                },
              )}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>

      <SidebarFooter className="p-2 mt-auto shrink-0">
        <SidebarMenu>
          <SidebarMenuItem>
            <SettingsDropdownMenu onOpenSettings={onOpenSettings} />
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarFooter>
    </>
  );
}

// ============ SETTINGS SIDEBAR ============
function SettingsSidebar({
  activeTab,
  onChangeTab,
  onBack,
}: {
  activeTab: string;
  onChangeTab: (tab: string) => void;
  onBack: () => void;
}) {
  return (
    <>
      <SidebarContent className="px-3 pt-14">
        <SidebarGroup className="p-0 mb-1">
          <SidebarGroupLabel className="px-2.5 text-xs font-semibold text-muted-foreground/60 uppercase tracking-wider mb-1">
            Settings
          </SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenu className="gap-0">
              {settingsNavItems.map((item) => {
                const Icon = item.icon;
                const isActive = activeTab === item.id;

                return (
                  <SidebarMenuItem key={item.id}>
                    <SidebarMenuButton
                      onClick={() => onChangeTab(item.id)}
                      className={cn(
                        "h-[28px] px-2.5 rounded-[8px] hover:bg-accent transition-all",
                        isActive
                          ? "text-foreground bg-accent/30"
                          : "text-foreground/80 hover:text-foreground",
                      )}
                    >
                      <Icon
                        className="w-3.5 h-3.5 text-muted-foreground"
                        strokeWidth={1.5}
                      />
                      <span className="text-[12px] font-medium">
                        {item.label}
                      </span>
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                );
              })}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>

        <SidebarGroup className="p-0 mt-3">
          <SidebarGroupLabel className="px-2.5 text-xs font-semibold text-muted-foreground/60 uppercase tracking-wider mb-1">
            Navigation
          </SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenu className="gap-0">
              <SidebarMenuItem>
                <SidebarMenuButton
                  onClick={onBack}
                  className="h-[28px] px-2.5 rounded-[8px] hover:bg-accent text-foreground/80 hover:text-foreground transition-all"
                >
                  <ArrowLeft
                    className="w-3.5 h-3.5 text-muted-foreground"
                    strokeWidth={1.5}
                  />
                  <span className="text-[12px] font-medium">Back to app</span>
                </SidebarMenuButton>
              </SidebarMenuItem>
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>
    </>
  );
}

// ============ APP SIDEBAR ============
function AppSidebar({
  view,
  onOpenSettings,
  onChangeSettingsTab,
  onBackToHome,
}: {
  view: ViewState;
  onOpenSettings: () => void;
  onChangeSettingsTab: (tab: string) => void;
  onBackToHome: () => void;
}) {
  return (
    <Sidebar collapsible="offcanvas" className="bg-sidebar">
      {view.type === "home" ? (
        <HomeSidebar onOpenSettings={onOpenSettings} />
      ) : (
        <SettingsSidebar
          activeTab={view.tab}
          onChangeTab={onChangeSettingsTab}
          onBack={onBackToHome}
        />
      )}
    </Sidebar>
  );
}

// ============ TOP CONTROLS ============
function TopControls({
  isFullscreen,
  sidebarOpen,
}: {
  isFullscreen: boolean;
  sidebarOpen: boolean;
}) {
  // Windowed + collapsed: position to avoid traffic lights and align with them
  // Windowed + open: keep current correct placement inside content area
  // Fullscreen: no adjustment needed (traffic lights hidden)
  const isWindowedCollapsed = !isFullscreen && !sidebarOpen;

  if (isWindowedCollapsed) {
    // Align horizontally with traffic lights (y: 12px) and add more right spacing
    return (
      <div
        className="absolute z-50 pointer-events-none transition-[left,top] duration-300 ease-[cubic-bezier(0.4,0,0.2,1)]"
        style={{ left: "95px", top: "12px" }}
        data-no-drag="true"
      >
        <div className="pointer-events-auto flex items-center gap-1">
          <SidebarToggle />
          <button
            className="flex items-center justify-center rounded-[6px] text-muted-foreground hover:text-foreground hover:bg-accent w-[28px] h-[28px] transition-all duration-300 ease-[cubic-bezier(0.4,0,0.2,1)]"
            aria-label="New thread"
            title="New thread"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              className="lucide lucide-square-pen"
            >
              <path d="M12 3H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"></path>
              <path d="M18.375 2.625a1 1 0 0 1 3 3l-9.013 9.014a2 2 0 0 1-.853.505l-2.873.84a.5.5 0 0 1-.62-.62l.84-2.873a2 2 0 0 1 .506-.852z"></path>
            </svg>
          </button>
        </div>
      </div>
    );
  }

  // Default placement for all other states (windowed open, fullscreen)
  return (
    <div
      className="absolute top-3 left-3 z-50 pointer-events-none transition-[left,top] duration-300 ease-[cubic-bezier(0.4,0,0.2,1)]"
      data-no-drag="true"
    >
      <div className="pointer-events-auto">
        <SidebarToggle />
      </div>
    </div>
  );
}

// ============ HOME CONTENT ============
function HomeContent() {
  return (
    <>
      <div
        className="flex-1 overflow-y-auto pb-64 custom-scrollbar"
        data-no-drag="true"
      >
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

            <div className="relative inline-block mb-12">
              <select className="appearance-none bg-transparent hover:bg-accent/50 text-muted-foreground py-1.5 pl-3 pr-8 rounded-lg focus:outline-none focus:ring-0 text-sm font-medium cursor-pointer transition-all">
                <option>Project Nexus</option>
                <option>Openakta App</option>
                <option>Snake Game</option>
              </select>
              <div className="pointer-events-none absolute inset-y-0 right-0 flex items-center px-2 text-muted-foreground">
                <ChevronDown className="w-3.5 h-3.5" />
              </div>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-3 gap-3 w-full">
              <div className="bg-muted/40 hover:bg-accent/80 border border-transparent p-4 rounded-2xl text-left cursor-pointer transition-all group">
                <div className="w-8 h-8 rounded-lg bg-muted/80 flex items-center justify-center mb-3 transition-colors group-hover:bg-accent">
                  <Gamepad2 className="w-4 h-4 text-foreground/70" />
                </div>
                <h3 className="text-[13px] font-medium text-foreground/90 mb-1">
                  Build a classic Snake game
                </h3>
                <p className="text-[12px] text-muted-foreground leading-relaxed group-hover:text-foreground/70 max-w-[90%]">
                  Start a new Python project using Pygame for a retro
                  experience.
                </p>
              </div>

              <div className="bg-muted/40 hover:bg-accent/80 border border-transparent p-4 rounded-2xl text-left cursor-pointer transition-all group">
                <div className="w-8 h-8 rounded-lg bg-muted/80 flex items-center justify-center mb-3 transition-colors group-hover:bg-accent">
                  <FileText className="w-4 h-4 text-[#ff8e8b]" />
                </div>
                <h3 className="text-[13px] font-medium text-foreground/90 mb-1">
                  Create a one-page PDF
                </h3>
                <p className="text-[12px] text-muted-foreground leading-relaxed group-hover:text-foreground/70 max-w-[90%]">
                  Generate a clean document layout using React and Tailwind CSS.
                </p>
              </div>

              <div className="bg-muted/40 hover:bg-accent/80 border border-transparent p-4 rounded-2xl text-left cursor-pointer transition-all group">
                <div className="w-8 h-8 rounded-lg bg-muted/80 flex items-center justify-center mb-3 transition-colors group-hover:bg-accent">
                  <PencilRuler className="w-4 h-4 text-[#ffd166]" />
                </div>
                <h3 className="text-[13px] font-medium text-foreground/90 mb-1">
                  Create a plan to...
                </h3>
                <p className="text-[12px] text-muted-foreground leading-relaxed group-hover:text-foreground/70 max-w-[90%]">
                  Map out a microservices architecture for your next big idea.
                </p>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div className="absolute bottom-0 left-0 right-0 flex flex-col items-center p-4 bg-gradient-to-t from-background via-background to-transparent pointer-events-none pb-6">
        <div className="w-full max-w-3xl px-6 pointer-events-auto">
          <div className="bg-panel/95 backdrop-blur-xl border border-border shadow-lg rounded-[20px] p-2.5 focus-within:border-border/80 transition-all">
            <textarea
              className="w-full bg-transparent border-none focus:outline-none focus:ring-0 text-foreground/90 placeholder-muted-foreground/70 text-[15px] resize-none min-h-[44px] p-2 custom-scrollbar"
              placeholder="Ask Openakta anything, @ to add files, / for commands"
              rows={1}
            ></textarea>

            <div className="flex items-center justify-between px-1 pt-1 mt-1">
              <div className="flex items-center gap-1.5">
                <button className="flex items-center gap-1.5 px-2 py-1.5 text-muted-foreground text-[11px] font-medium transition-all rounded-md hover:bg-accent/50 hover:text-foreground">
                  <Sparkles className="w-3.5 h-3.5 text-muted-foreground" />
                  GPT-5.4
                  <ChevronDown className="w-3 h-3 ml-0.5 opacity-50" />
                </button>
                <button className="flex items-center gap-1.5 px-2 py-1.5 text-muted-foreground text-[11px] font-medium transition-all rounded-md hover:bg-accent/50 hover:text-foreground">
                  <Zap className="w-3 h-3" />
                  Medium
                  <ChevronDown className="w-3 h-3 ml-0.5 opacity-50" />
                </button>
              </div>

              <div className="flex items-center gap-2">
                <button className="p-1.5 text-muted-foreground hover:text-foreground transition-colors rounded-lg hover:bg-accent">
                  <Paperclip className="w-[18px] h-[18px]" strokeWidth={2} />
                </button>
                <button className="w-[30px] h-[30px] rounded-full bg-primary text-primary-foreground flex items-center justify-center hover:bg-primary/90 transition-all shadow-sm ml-1">
                  <ArrowUp className="w-[16px] h-[16px]" strokeWidth={2.5} />
                </button>
              </div>
            </div>
          </div>

          <div className="flex items-center justify-between px-2 mt-3 text-[11px] font-medium text-muted-foreground">
            <div className="flex items-center gap-4">
              <span className="flex items-center gap-1.5 cursor-pointer hover:text-foreground transition-colors">
                <TerminalIcon className="w-3.5 h-3.5" />
                Local
                <ChevronDown className="w-3 h-3 opacity-50" />
              </span>
              <span className="flex items-center gap-1.5 text-[#ff8e8b] cursor-pointer hover:text-[#ff9f9c] transition-colors">
                <div className="w-3.5 h-3.5 flex items-center justify-center rounded border border-[#ff8e8b] text-[8px] leading-none pb-[1px]">
                  !
                </div>
                Full access
                <ChevronDown className="w-3 h-3 opacity-50" />
              </span>
            </div>
            <div className="flex items-center gap-1.5 cursor-pointer hover:text-foreground transition-colors">
              <GitBranch className="w-3.5 h-3.5" />
              main
              <ChevronDown className="w-3 h-3 opacity-50" />
            </div>
          </div>
        </div>
      </div>
    </>
  );
}

// ============ MAIN CONTENT ============
function MainContent({
  view,
  preferences,
  onToggle,
  isFullscreen,
}: {
  view: ViewState;
  preferences: DesktopPreferences | null;
  onToggle: (
    key: keyof Omit<DesktopPreferences, "themeMode">,
    value: boolean,
  ) => void;
  isFullscreen: boolean;
}) {
  const { open } = useSidebar();
  const { currentThreadId, isChatOpen } = useAppState();

  return (
    <SidebarInset className="relative h-full">
      <TopControls isFullscreen={isFullscreen} sidebarOpen={open} />
      <div className="flex-1 flex flex-col h-full overflow-hidden bg-background rounded-l-[16px]">
        {currentThreadId && isChatOpen ? <ChatView /> : <HomeContent />}
        {view.type === "settings" && (
          <SettingsMainPanel
            activeTab={view.tab}
            preferences={preferences}
            onToggle={onToggle}
            settingsNavItems={settingsNavItems}
          />
        )}
      </div>
    </SidebarInset>
  );
}

// ============ APP CONTENT ============
export function AppContent() {
  const [preferences, setPreferences] = useState<DesktopPreferences | null>(
    null,
  );
  const [view, setView] = useState<ViewState>({ type: "home" });
  const [isFullscreen, setIsFullscreen] = useState(false);

  useEffect(() => {
    desktopService
      .getPreferences()
      .then((p) => setPreferences(p ?? defaultPreferences))
      .catch(() => setPreferences(defaultPreferences));

    desktopService.getFullscreenState().then((state) => {
      if (state !== null) {
        setIsFullscreen(state);
      }
    });

    const unsubscribe = desktopService.onFullscreenChange((fullscreen) => {
      setIsFullscreen(fullscreen);
    });

    return unsubscribe;
  }, []);

  async function handleToggle(
    key: keyof Omit<DesktopPreferences, "themeMode">,
    value: boolean,
  ) {
    const next = await desktopService.updatePreferences({ [key]: value });
    if (next) {
      setPreferences(next);
    }
  }

  const handleOpenSettings = () => {
    setView({ type: "settings", tab: "general" });
  };

  const handleBackToHome = () => {
    setView({ type: "home" });
  };

  const handleChangeSettingsTab = (tab: string) => {
    setView({ type: "settings", tab });
  };

  return (
    <>
      <AppSidebar
        view={view}
        onOpenSettings={handleOpenSettings}
        onChangeSettingsTab={handleChangeSettingsTab}
        onBackToHome={handleBackToHome}
      />
      <MainContent
        view={view}
        preferences={preferences}
        onToggle={handleToggle}
        isFullscreen={isFullscreen}
      />
    </>
  );
}

// ============ ROOT LAYOUT ============
export default function RootLayout() {
  return (
    <html lang="en" suppressHydrationWarning>
      <body className="antialiased">
        <ThemeProvider defaultTheme="system" enableSystem>
          <AppProvider>
            <SidebarProvider
              defaultOpen={true}
              style={{ "--sidebar-width": "260px" } as React.CSSProperties}
              className="h-screen w-full overflow-hidden bg-background text-foreground font-sans selection:bg-primary/20 app-drag-region"
            >
              <AppContent />
            </SidebarProvider>
          </AppProvider>
        </ThemeProvider>
      </body>
    </html>
  );
}
