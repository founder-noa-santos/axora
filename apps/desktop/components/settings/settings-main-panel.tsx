"use client";

import { useMemo, useState } from "react";
import { ThemeModeToggle } from "@/components/theme-mode-toggle";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Separator } from "@/components/ui/separator";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import type { DesktopPreferences } from "@/shared/contracts/desktop";
import type { UsageState } from "@/shared/contracts/preferences";
import { ExternalLink, FolderOpen, Plus, Settings } from "lucide-react";
import { cn } from "@/lib/utils";

function SettingRow({
  label,
  description,
  control,
}: {
  label: string;
  description: string;
  control: React.ReactNode;
}) {
  return (
    <div className="flex items-center justify-between px-5 py-4">
      <div className="flex flex-col gap-0.5 pr-4">
        <span className="text-[14px] text-foreground">{label}</span>
        <span className="text-[13px] text-muted-foreground leading-relaxed">
          {description}
        </span>
      </div>
      <div className="flex shrink-0 items-center">{control}</div>
    </div>
  );
}

/** Mock: fields are “percent remaining” for progress UI. */
const MOCK_USAGE: UsageState = {
  fiveHourLimitPercent: 88,
  fiveHourResetAt: new Date(Date.now() + 3.6e6).toISOString(),
  weeklyLimitPercent: 74,
  weeklyResetAt: new Date(Date.now() + 86400000 * 4).toISOString(),
  creditBalance: 120,
  autoReloadEnabled: false,
  plan: "free",
};

type SettingsNavItem = { id: string; label: string };

export function SettingsMainPanel({
  activeTab,
  preferences,
  onToggle,
  settingsNavItems,
}: {
  activeTab: string;
  preferences: DesktopPreferences | null;
  onToggle: (
    key: keyof Omit<DesktopPreferences, "themeMode">,
    value: boolean,
  ) => void;
  settingsNavItems: SettingsNavItem[];
}) {
  const current = preferences ?? {
    themeMode: "dark" as const,
    compactSidebar: true,
    reduceMotion: false,
    commandCenterPinned: true,
    launchAtLogin: false,
  };

  /* LOCAL-ONLY: was incorrectly bound to `launchAtLogin` (IPC). */
  const [preventSleep, setPreventSleep] = useState(false);
  /* LOCAL-ONLY: was incorrectly bound to `reduceMotion` (IPC). */
  const [requireCmdEnterForLongPrompts, setRequireCmdEnterForLongPrompts] =
    useState(false);

  const [permissionNotifications, setPermissionNotifications] = useState(true);
  const [questionNotifications, setQuestionNotifications] = useState(true);

  const [accentColor, setAccentColor] = useState("#6b8cce");
  const [backgroundColor, setBackgroundColor] = useState("#0a0a0b");
  const [foregroundColor, setForegroundColor] = useState("#e8e8ea");
  const [translucentSidebar, setTranslucentSidebar] = useState(false);
  const [contrastLight, setContrastLight] = useState(50);
  const [contrastDark, setContrastDark] = useState(50);
  const [usePointerCursors, setUsePointerCursors] = useState(true);
  const [uiFontSizePx, setUiFontSizePx] = useState(14);

  const [approvalPolicy, setApprovalPolicy] = useState<string>("on_request");
  const [sandboxMode, setSandboxMode] = useState<string>("read_write");
  const [importCursor, setImportCursor] = useState(false);
  const [importCodex, setImportCodex] = useState(false);

  const [personality, setPersonality] = useState("pragmatic");
  const [customInstructions, setCustomInstructions] = useState("");

  const [gitBranchPrefix, setGitBranchPrefix] = useState("codex/");
  const [prMerge, setPrMerge] = useState("squash");
  const [gitToggles, setGitToggles] = useState({
    showPrIcons: true,
    forcePush: false,
    draftPrs: true,
  });
  const [commitInstr, setCommitInstr] = useState("");
  const [prInstr, setPrInstr] = useState("");

  const [worktreesAutoDelete, setWorktreesAutoDelete] = useState(true);
  const [worktreesLimit, setWorktreesLimit] = useState(15);

  const [mcpDialogOpen, setMcpDialogOpen] = useState(false);
  const [mcpDraft, setMcpDraft] = useState({
    name: "",
    url: "",
    envLines: [{ key: "", value: "" }] as { key: string; value: string }[],
  });

  const archivedMock = useMemo(
    () => [
      {
        id: "a1",
        title: "Spike: embeddings",
        projectId: "proj-1",
        archivedAt: new Date().toISOString(),
      },
    ],
    [],
  );

  const envProjectsMock = useMemo(
    () => [
      {
        id: "p1",
        name: "openakta",
        org: "fluri",
        environments: [
          {
            name: "default",
            configPath: "/Users/me/Projects/openakta/environment.toml",
          },
        ],
      },
    ],
    [],
  );

  const tabTitle =
    settingsNavItems.find((i) => i.id === activeTab)?.label ?? "Settings";

  if (activeTab === "general") {
    return (
      <div className="custom-scrollbar flex-1 overflow-y-auto bg-background">
        <div className="mx-auto max-w-[720px] px-8 py-10">
          <h1 className="text-foreground mb-8 text-[22px] font-semibold">
            {tabTitle}
          </h1>
          <div className="space-y-8">
            <div className="border-border bg-muted/50 overflow-hidden rounded-xl border">
              <SettingRow
                label="Default open destination"
                description="Where files and folders open by default (local only)"
                control={
                  <Select defaultValue="cursor">
                    <SelectTrigger className="bg-accent text-foreground h-9 w-[180px] rounded-lg border-none text-[13px] focus:ring-0 focus:ring-offset-0">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent className="bg-accent border-border text-foreground rounded-lg">
                      <SelectItem value="cursor" className="text-[13px]">
                        Cursor
                      </SelectItem>
                      <SelectItem value="vscode" className="text-[13px]">
                        VS Code
                      </SelectItem>
                      <SelectItem value="finder" className="text-[13px]">
                        Finder
                      </SelectItem>
                    </SelectContent>
                  </Select>
                }
              />
              <div className="bg-border h-px" />
              <SettingRow
                label="Language"
                description="Language for the app UI (local only)"
                control={
                  <Select defaultValue="auto">
                    <SelectTrigger className="bg-accent text-foreground h-9 w-[180px] rounded-lg border-none text-[13px] focus:ring-0 focus:ring-offset-0">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent className="bg-accent border-border text-foreground rounded-lg">
                      <SelectItem value="auto" className="text-[13px]">
                        Auto Detect
                      </SelectItem>
                      <SelectItem value="en" className="text-[13px]">
                        English
                      </SelectItem>
                    </SelectContent>
                  </Select>
                }
              />
              <div className="bg-border h-px" />
              <SettingRow
                label="Thread detail"
                description="Choose how much command output to show in threads (local only)"
                control={
                  <Select defaultValue="steps">
                    <SelectTrigger className="bg-accent text-foreground h-9 w-[220px] rounded-lg border-none text-[13px] focus:ring-0 focus:ring-offset-0">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent className="bg-accent border-border text-foreground rounded-lg">
                      <SelectItem value="minimal" className="text-[13px]">
                        Minimal
                      </SelectItem>
                      <SelectItem value="steps" className="text-[13px]">
                        Steps
                      </SelectItem>
                      <SelectItem
                        value="steps_with_code"
                        className="text-[13px]"
                      >
                        Steps with code commands
                      </SelectItem>
                    </SelectContent>
                  </Select>
                }
              />
              <div className="bg-border h-px" />
              <SettingRow
                label="Popout Window hotkey"
                description="Keyboard shortcut for quick popout (local only)"
                control={
                  <div className="flex items-center gap-2">
                    <Input
                      className="h-9 w-[120px] text-[13px]"
                      defaultValue="⌃⌥Space"
                      readOnly
                    />
                    <Button type="button" size="sm" variant="outline">
                      Set
                    </Button>
                  </div>
                }
              />
              <div className="bg-border h-px" />
              <SettingRow
                label="Prevent sleep while running"
                description="Keep the system awake while OPENAKTA runs a thread (local only — not launch-at-login)"
                control={
                  <Switch
                    checked={preventSleep}
                    onCheckedChange={setPreventSleep}
                    className="data-[state=checked]:bg-primary data-[state=unchecked]:bg-muted-foreground/30"
                  />
                }
              />
              <div className="bg-border h-px" />
              <SettingRow
                label="Launch OPENAKTA at login"
                description="Open OPENAKTA when you start your computer"
                control={
                  <Switch
                    checked={current.launchAtLogin}
                    onCheckedChange={(v) => onToggle("launchAtLogin", v)}
                    className="data-[state=checked]:bg-primary data-[state=unchecked]:bg-muted-foreground/30"
                  />
                }
              />
              <div className="bg-border h-px" />
              <SettingRow
                label="Pin command center"
                description="Keep the command center visible in the shell"
                control={
                  <Switch
                    checked={current.commandCenterPinned}
                    onCheckedChange={(v) => onToggle("commandCenterPinned", v)}
                    className="data-[state=checked]:bg-primary data-[state=unchecked]:bg-muted-foreground/30"
                  />
                }
              />
              <div className="bg-border h-px" />
              <SettingRow
                label="Require ⌘ + enter to send long prompts"
                description="Multiline prompts require ⌘ + Enter to send (local only — not reduce-motion)"
                control={
                  <Switch
                    checked={requireCmdEnterForLongPrompts}
                    onCheckedChange={setRequireCmdEnterForLongPrompts}
                    className="data-[state=checked]:bg-primary data-[state=unchecked]:bg-muted-foreground/30"
                  />
                }
              />
              <div className="bg-border h-px" />
              <SettingRow
                label="Speed"
                description="Standard vs fast inference (local only for now)"
                control={
                  <Select defaultValue="standard">
                    <SelectTrigger className="bg-accent text-foreground h-9 w-[180px] rounded-lg border-none text-[13px] focus:ring-0 focus:ring-offset-0">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent className="bg-accent border-border text-foreground rounded-lg">
                      <SelectItem value="standard" className="text-[13px]">
                        Standard
                      </SelectItem>
                      <SelectItem value="fast" className="text-[13px]">
                        Fast (2× usage)
                      </SelectItem>
                    </SelectContent>
                  </Select>
                }
              />
              <div className="bg-border h-px" />
              <SettingRow
                label="Follow-up behavior"
                description="Queue vs steer (local only)"
                control={
                  <div className="bg-accent flex items-center gap-2 rounded-lg p-1">
                    <button
                      type="button"
                      className="bg-muted text-foreground rounded-md px-3 py-1.5 text-[13px]"
                    >
                      Queue
                    </button>
                    <button
                      type="button"
                      className="text-muted-foreground hover:text-foreground rounded-md px-3 py-1.5 text-[13px]"
                    >
                      Steer
                    </button>
                  </div>
                }
              />
            </div>

            <div>
              <h2 className="text-foreground mb-4 text-[16px] font-medium">
                Notifications
              </h2>
              <div className="border-border bg-muted/50 overflow-hidden rounded-xl border">
                <SettingRow
                  label="Completion notifications"
                  description="When OPENAKTA alerts you that a run finished (local only)"
                  control={
                    <Select defaultValue="unfocused">
                      <SelectTrigger className="bg-accent text-foreground h-9 w-[200px] rounded-lg border-none text-[13px] focus:ring-0 focus:ring-offset-0">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent className="bg-accent border-border text-foreground rounded-lg">
                        <SelectItem value="always" className="text-[13px]">
                          Always
                        </SelectItem>
                        <SelectItem value="unfocused" className="text-[13px]">
                          Only when unfocused
                        </SelectItem>
                        <SelectItem value="never" className="text-[13px]">
                          Never
                        </SelectItem>
                      </SelectContent>
                    </Select>
                  }
                />
                <div className="bg-border h-px" />
                <SettingRow
                  label="Permission notifications"
                  description="Alert when notification permissions are needed (local only)"
                  control={
                    <Checkbox
                      checked={permissionNotifications}
                      onCheckedChange={(v) =>
                        setPermissionNotifications(v === true)
                      }
                    />
                  }
                />
                <div className="bg-border h-px" />
                <SettingRow
                  label="Question notifications"
                  description="Alert when the model asks a question (local only)"
                  control={
                    <Checkbox
                      checked={questionNotifications}
                      onCheckedChange={(v) =>
                        setQuestionNotifications(v === true)
                      }
                    />
                  }
                />
              </div>
            </div>
          </div>
        </div>
      </div>
    );
  }

  if (activeTab === "appearance") {
    return (
      <div className="custom-scrollbar flex-1 overflow-y-auto bg-background">
        <div className="mx-auto max-w-[720px] px-8 py-10">
          <h1 className="text-foreground mb-8 text-[22px] font-semibold">
            {tabTitle}
          </h1>
          <div className="space-y-8">
            <div>
              <h2 className="text-foreground mb-4 text-[16px] font-medium">
                Theme
              </h2>
              <div className="border-border bg-muted/50 rounded-xl border p-6">
                <p className="text-muted-foreground mb-4 text-[13px]">
                  Choose how OPENAKTA looks. Theme mode syncs via
                  DesktopPreferences (IPC).
                </p>
                <ThemeModeToggle />
              </div>
            </div>
            <div className="border-border bg-muted/50 overflow-hidden rounded-xl border">
              <SettingRow
                label="Light / dark presets"
                description="Named presets (local only)"
                control={
                  <div className="flex gap-2">
                    <Select defaultValue="default-light">
                      <SelectTrigger className="h-9 w-[130px] text-[12px]">
                        <SelectValue placeholder="Light" />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="default-light">
                          Light · Default
                        </SelectItem>
                      </SelectContent>
                    </Select>
                    <Select defaultValue="default-dark">
                      <SelectTrigger className="h-9 w-[130px] text-[12px]">
                        <SelectValue placeholder="Dark" />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="default-dark">
                          Dark · Default
                        </SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                }
              />
              <div className="bg-border h-px" />
              <SettingRow
                label="Accent color"
                description="Local preview only"
                control={
                  <input
                    type="color"
                    aria-label="Accent"
                    className="h-9 w-12 cursor-pointer rounded border-0 bg-transparent"
                    value={accentColor}
                    onChange={(e) => setAccentColor(e.target.value)}
                  />
                }
              />
              <div className="bg-border h-px" />
              <SettingRow
                label="Background color"
                description="Canvas background (local preview)"
                control={
                  <input
                    type="color"
                    aria-label="Background"
                    className="h-9 w-12 cursor-pointer rounded border-0 bg-transparent"
                    value={backgroundColor}
                    onChange={(e) => setBackgroundColor(e.target.value)}
                  />
                }
              />
              <div className="bg-border h-px" />
              <SettingRow
                label="Foreground color"
                description="Primary text color (local preview)"
                control={
                  <input
                    type="color"
                    aria-label="Foreground"
                    className="h-9 w-12 cursor-pointer rounded border-0 bg-transparent"
                    value={foregroundColor}
                    onChange={(e) => setForegroundColor(e.target.value)}
                  />
                }
              />
              <div className="bg-border h-px" />
              <SettingRow
                label="Translucent sidebar"
                description="Glass effect (local only)"
                control={
                  <Switch
                    checked={translucentSidebar}
                    onCheckedChange={setTranslucentSidebar}
                  />
                }
              />
              <div className="bg-border h-px" />
              <SettingRow
                label="Contrast (light)"
                description="0–100 (local only)"
                control={
                  <input
                    type="range"
                    min={0}
                    max={100}
                    value={contrastLight}
                    onChange={(e) => setContrastLight(Number(e.target.value))}
                    className="w-[140px]"
                  />
                }
              />
              <div className="bg-border h-px" />
              <SettingRow
                label="Contrast (dark)"
                description="0–100 (local only)"
                control={
                  <input
                    type="range"
                    min={0}
                    max={100}
                    value={contrastDark}
                    onChange={(e) => setContrastDark(Number(e.target.value))}
                    className="w-[140px]"
                  />
                }
              />
              <div className="bg-border h-px" />
              <SettingRow
                label="UI / Code fonts"
                description="Typography preview (local only)"
                control={
                  <span className="text-muted-foreground text-[12px]">
                    Instrument Sans / JetBrains Mono
                  </span>
                }
              />
              <div className="bg-border h-px" />
              <SettingRow
                label="Use pointer cursors"
                description="Local only"
                control={
                  <Switch
                    checked={usePointerCursors}
                    onCheckedChange={setUsePointerCursors}
                  />
                }
              />
              <div className="bg-border h-px" />
              <SettingRow
                label="UI font size"
                description="Pixels (local only)"
                control={
                  <input
                    type="range"
                    min={10}
                    max={20}
                    value={uiFontSizePx}
                    onChange={(e) => setUiFontSizePx(Number(e.target.value))}
                    className="w-[140px]"
                  />
                }
              />
              <div className="bg-border h-px" />
              <SettingRow
                label="Compact sidebar"
                description="Reduces sidebar padding (syncs via DesktopPreferences)"
                control={
                  <Switch
                    checked={current.compactSidebar}
                    onCheckedChange={(v) => onToggle("compactSidebar", v)}
                    className="data-[state=checked]:bg-primary data-[state=unchecked]:bg-muted-foreground/30"
                  />
                }
              />
              <div className="bg-border h-px" />
              <SettingRow
                label="Reduce motion"
                description="Minimize animations (syncs via DesktopPreferences)"
                control={
                  <Switch
                    checked={current.reduceMotion}
                    onCheckedChange={(v) => onToggle("reduceMotion", v)}
                    className="data-[state=checked]:bg-primary data-[state=unchecked]:bg-muted-foreground/30"
                  />
                }
              />
            </div>
          </div>
        </div>
      </div>
    );
  }

  if (activeTab === "configuration") {
    return (
      <div className="custom-scrollbar flex-1 overflow-y-auto bg-background">
        <div className="mx-auto max-w-[720px] px-8 py-10">
          <h1 className="text-foreground mb-8 text-[22px] font-semibold">
            {tabTitle}
          </h1>
          <div className="border-border bg-muted/50 overflow-hidden rounded-xl border">
            <SettingRow
              label="Approval policy"
              description="Tool execution policy (local UI)"
              control={
                <Select
                  value={approvalPolicy}
                  onValueChange={setApprovalPolicy}
                >
                  <SelectTrigger className="h-9 w-[180px] text-[13px]">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="on_request">On request</SelectItem>
                    <SelectItem value="always">Always</SelectItem>
                    <SelectItem value="never">Never</SelectItem>
                  </SelectContent>
                </Select>
              }
            />
            <div className="bg-border h-px" />
            <SettingRow
              label="Sandbox"
              description="Filesystem access level (local UI)"
              control={
                <Select value={sandboxMode} onValueChange={setSandboxMode}>
                  <SelectTrigger className="h-9 w-[200px] text-[13px]">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="read_only">Read only</SelectItem>
                    <SelectItem value="read_write">Read + write</SelectItem>
                    <SelectItem value="full_access">Full access</SelectItem>
                  </SelectContent>
                </Select>
              }
            />
            <div className="bg-border h-px" />
            <div className="px-5 py-4">
              <p className="text-foreground mb-3 text-[14px]">
                Import external agent config
              </p>
              <div className="space-y-2">
                <label className="flex items-center gap-2 text-[13px]">
                  <Checkbox
                    checked={importCursor}
                    onCheckedChange={(v) => setImportCursor(v === true)}
                  />
                  Cursor rules &amp; prompts
                </label>
                <label className="flex items-center gap-2 text-[13px]">
                  <Checkbox
                    checked={importCodex}
                    onCheckedChange={(v) => setImportCodex(v === true)}
                  />
                  Codex CLI config
                </label>
              </div>
              <Button type="button" className="mt-4" size="sm">
                Apply selected
              </Button>
            </div>
          </div>
        </div>
      </div>
    );
  }

  if (activeTab === "personalization") {
    return (
      <div className="custom-scrollbar flex-1 overflow-y-auto bg-background">
        <div className="mx-auto max-w-[720px] px-8 py-10">
          <h1 className="text-foreground mb-8 text-[22px] font-semibold">
            {tabTitle}
          </h1>
          <div className="space-y-6">
            <div>
              <Label className="mb-2 block text-[13px]">Personality</Label>
              <Select value={personality} onValueChange={setPersonality}>
                <SelectTrigger className="max-w-md">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="pragmatic">Pragmatic</SelectItem>
                  <SelectItem value="collaborative">Collaborative</SelectItem>
                  <SelectItem value="verbose">Verbose</SelectItem>
                  <SelectItem value="concise">Concise</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div>
              <Label className="mb-2 block text-[13px]">
                Custom instructions
              </Label>
              <Textarea
                className="min-h-[180px] text-[13px]"
                value={customInstructions}
                onChange={(e) => setCustomInstructions(e.target.value)}
                placeholder="System instructions merged into new threads…"
              />
            </div>
          </div>
        </div>
      </div>
    );
  }

  if (activeTab === "usage") {
    const u = MOCK_USAGE;
    return (
      <div className="custom-scrollbar flex-1 overflow-y-auto bg-background">
        <div className="mx-auto max-w-[720px] px-8 py-10">
          <h1 className="text-foreground mb-8 text-[22px] font-semibold">
            {tabTitle}
          </h1>
          <div className="space-y-6">
            <div>
              <div className="mb-1 flex justify-between text-[13px]">
                <span>5h usage limit</span>
                <span className="text-muted-foreground">
                  {u.fiveHourLimitPercent}% left · resets{" "}
                  {new Date(u.fiveHourResetAt).toLocaleTimeString([], {
                    hour: "2-digit",
                    minute: "2-digit",
                  })}
                </span>
              </div>
              <div className="bg-muted h-2 w-full overflow-hidden rounded-full">
                <div
                  className="bg-primary h-full rounded-full transition-all"
                  style={{ width: `${u.fiveHourLimitPercent}%` }}
                />
              </div>
            </div>
            <div>
              <div className="mb-1 flex justify-between text-[13px]">
                <span>Weekly usage limit</span>
                <span className="text-muted-foreground">
                  {u.weeklyLimitPercent}% left ·{" "}
                  {new Date(u.weeklyResetAt).toLocaleDateString()}
                </span>
              </div>
              <div className="bg-muted h-2 w-full overflow-hidden rounded-full">
                <div
                  className="bg-primary h-full rounded-full transition-all"
                  style={{ width: `${u.weeklyLimitPercent}%` }}
                />
              </div>
            </div>
            <Separator />
            <div className="flex items-center justify-between gap-4">
              <div>
                <p className="text-[14px]">Credit balance</p>
                <p className="text-muted-foreground text-[13px]">
                  {u.creditBalance} credits remaining · plan: {u.plan}
                </p>
              </div>
              <Button size="sm" variant="outline">
                Purchase
              </Button>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-[13px]">Auto-reload credit</span>
              <Button size="sm" variant="ghost">
                Settings
              </Button>
            </div>
            <Button variant="ghost" className="h-auto px-0 text-[13px]" asChild>
              <a
                href="https://vercel.com/docs/ai-gateway"
                target="_blank"
                rel="noreferrer"
                className="inline-flex items-center gap-1"
              >
                Upgrade to Pro
                <ExternalLink className="size-3" />
              </a>
            </Button>
          </div>
        </div>
      </div>
    );
  }

  if (activeTab === "mcp") {
    const mockCustom = [
      { id: "c1", name: "Filesystem", enabled: true },
      { id: "c2", name: "GitHub", enabled: false },
    ];
    const mockRec = [
      { id: "r1", name: "Supabase", installed: false },
      { id: "r2", name: "Sentry", installed: true },
    ];
    return (
      <div className="custom-scrollbar flex-1 overflow-y-auto bg-background">
        <div className="mx-auto max-w-[720px] px-8 py-10">
          <h1 className="text-foreground mb-8 text-[22px] font-semibold">
            {tabTitle}
          </h1>
          <div className="space-y-8">
            <div>
              <div className="mb-3 flex items-center justify-between">
                <h2 className="text-[15px] font-medium">Custom servers</h2>
                <Button
                  type="button"
                  size="sm"
                  onClick={() => setMcpDialogOpen(true)}
                >
                  <Plus className="mr-1 size-3.5" />
                  Add server
                </Button>
              </div>
              <div className="border-border rounded-xl border">
                {mockCustom.map((row) => (
                  <div
                    key={row.id}
                    className="flex items-center justify-between px-4 py-3"
                  >
                    <span className="text-[13px]">{row.name}</span>
                    <Switch checked={row.enabled} disabled />
                  </div>
                ))}
              </div>
            </div>
            <div>
              <div className="mb-3 flex items-center justify-between">
                <h2 className="text-[15px] font-medium">Recommended</h2>
                <Button type="button" size="sm" variant="outline">
                  Refresh
                </Button>
              </div>
              <div className="border-border rounded-xl border">
                {mockRec.map((row) => (
                  <div
                    key={row.id}
                    className="flex items-center justify-between px-4 py-3"
                  >
                    <span className="text-[13px]">{row.name}</span>
                    {row.installed ? (
                      <Switch checked disabled />
                    ) : (
                      <Button size="sm" variant="outline">
                        Install
                      </Button>
                    )}
                  </div>
                ))}
              </div>
            </div>
          </div>
          <Dialog open={mcpDialogOpen} onOpenChange={setMcpDialogOpen}>
            <DialogContent className="sm:max-w-md">
              <DialogHeader>
                <DialogTitle>Add MCP server</DialogTitle>
              </DialogHeader>
              <div className="space-y-3 py-2">
                <div>
                  <Label className="text-[12px]">Server name</Label>
                  <Input
                    value={mcpDraft.name}
                    onChange={(e) =>
                      setMcpDraft((d) => ({ ...d, name: e.target.value }))
                    }
                  />
                </div>
                <div>
                  <Label className="text-[12px]">URL or command</Label>
                  <Input
                    value={mcpDraft.url}
                    onChange={(e) =>
                      setMcpDraft((d) => ({ ...d, url: e.target.value }))
                    }
                  />
                </div>
                <div>
                  <Label className="text-[12px]">Environment variables</Label>
                  <div className="mt-1 space-y-2">
                    {mcpDraft.envLines.map((line, i) => (
                      <div key={i} className="flex gap-2">
                        <Input
                          placeholder="KEY"
                          value={line.key}
                          onChange={(e) =>
                            setMcpDraft((d) => {
                              const next = [...d.envLines];
                              next[i] = { ...next[i], key: e.target.value };
                              return { ...d, envLines: next };
                            })
                          }
                        />
                        <Input
                          placeholder="value"
                          value={line.value}
                          onChange={(e) =>
                            setMcpDraft((d) => {
                              const next = [...d.envLines];
                              next[i] = { ...next[i], value: e.target.value };
                              return { ...d, envLines: next };
                            })
                          }
                        />
                      </div>
                    ))}
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      onClick={() =>
                        setMcpDraft((d) => ({
                          ...d,
                          envLines: [...d.envLines, { key: "", value: "" }],
                        }))
                      }
                    >
                      Add row
                    </Button>
                  </div>
                </div>
              </div>
              <div className="flex justify-end pt-2">
                <Button type="button" onClick={() => setMcpDialogOpen(false)}>
                  Save
                </Button>
              </div>
            </DialogContent>
          </Dialog>
        </div>
      </div>
    );
  }

  if (activeTab === "git") {
    return (
      <div className="custom-scrollbar flex-1 overflow-y-auto bg-background">
        <div className="mx-auto max-w-[720px] px-8 py-10">
          <h1 className="text-foreground mb-8 text-[22px] font-semibold">
            {tabTitle}
          </h1>
          <div className="border-border bg-muted/50 space-y-4 rounded-xl border p-5">
            <div>
              <Label className="mb-1 block text-[12px]">Branch prefix</Label>
              <Input
                value={gitBranchPrefix}
                onChange={(e) => setGitBranchPrefix(e.target.value)}
              />
            </div>
            <div>
              <Label className="mb-2 block text-[12px]">PR merge method</Label>
              <div className="bg-accent flex w-fit gap-1 rounded-lg p-1">
                <button
                  type="button"
                  onClick={() => setPrMerge("merge")}
                  className={cn(
                    "rounded-md px-3 py-1.5 text-[13px]",
                    prMerge === "merge"
                      ? "bg-muted text-foreground"
                      : "text-muted-foreground",
                  )}
                >
                  Merge
                </button>
                <button
                  type="button"
                  onClick={() => setPrMerge("squash")}
                  className={cn(
                    "rounded-md px-3 py-1.5 text-[13px]",
                    prMerge === "squash"
                      ? "bg-muted text-foreground"
                      : "text-muted-foreground",
                  )}
                >
                  Squash
                </button>
              </div>
            </div>
            <SettingRow
              label="Show PR icons in sidebar"
              description="Decorates thread rows with PR state (local)"
              control={
                <Switch
                  checked={gitToggles.showPrIcons}
                  onCheckedChange={(v) =>
                    setGitToggles((t) => ({ ...t, showPrIcons: v }))
                  }
                />
              }
            />
            <SettingRow
              label="Always force push"
              description="Dangerous · local only"
              control={
                <Switch
                  checked={gitToggles.forcePush}
                  onCheckedChange={(v) =>
                    setGitToggles((t) => ({ ...t, forcePush: v }))
                  }
                />
              }
            />
            <SettingRow
              label="Create draft pull requests"
              description="Default PRs as draft (local)"
              control={
                <Switch
                  checked={gitToggles.draftPrs}
                  onCheckedChange={(v) =>
                    setGitToggles((t) => ({ ...t, draftPrs: v }))
                  }
                />
              }
            />
            <div>
              <Label className="mb-1 block text-[12px]">
                Commit instructions
              </Label>
              <Textarea
                className="min-h-[100px]"
                value={commitInstr}
                onChange={(e) => setCommitInstr(e.target.value)}
              />
            </div>
            <div>
              <Label className="mb-1 block text-[12px]">
                Pull request instructions
              </Label>
              <Textarea
                className="min-h-[100px]"
                value={prInstr}
                onChange={(e) => setPrInstr(e.target.value)}
              />
            </div>
          </div>
        </div>
      </div>
    );
  }

  if (activeTab === "environments") {
    return (
      <div className="custom-scrollbar flex-1 overflow-y-auto bg-background">
        <div className="mx-auto max-w-[720px] px-8 py-10">
          <h1 className="text-foreground mb-8 text-[22px] font-semibold">
            {tabTitle}
          </h1>
          <Button type="button" size="sm" className="mb-4">
            Add project
          </Button>
          <div className="border-border rounded-xl border">
            {envProjectsMock.map((p) => (
              <div
                key={p.id}
                className="border-border border-b px-4 py-3 last:border-b-0"
              >
                <div className="flex items-center gap-2">
                  <FolderOpen className="text-muted-foreground size-4" />
                  <span className="text-[13px] font-medium">{p.name}</span>
                  {p.org ? (
                    <span className="text-muted-foreground text-[11px]">
                      · {p.org}
                    </span>
                  ) : null}
                </div>
                {p.environments.map((env) => (
                  <div
                    key={env.name}
                    className="mt-2 flex items-center justify-between pl-6"
                  >
                    <span className="text-muted-foreground text-[12px]">
                      {env.name}
                    </span>
                    <Button size="sm" variant="ghost">
                      View
                    </Button>
                  </div>
                ))}
              </div>
            ))}
          </div>
        </div>
      </div>
    );
  }

  if (activeTab === "worktrees") {
    return (
      <div className="custom-scrollbar flex-1 overflow-y-auto bg-background">
        <div className="mx-auto max-w-[720px] px-8 py-10">
          <h1 className="text-foreground mb-8 text-[22px] font-semibold">
            {tabTitle}
          </h1>
          <div className="border-border bg-muted/50 mb-6 overflow-hidden rounded-xl border">
            <SettingRow
              label="Automatically delete old worktrees"
              description="When off, OPENAKTA keeps all worktrees (local)"
              control={
                <Switch
                  checked={worktreesAutoDelete}
                  onCheckedChange={setWorktreesAutoDelete}
                />
              }
            />
            <div className="bg-border h-px" />
            <SettingRow
              label="Auto-delete limit"
              description="Max retained worktrees (local)"
              control={
                <Input
                  type="number"
                  min={1}
                  max={100}
                  className="h-9 w-20 text-[13px]"
                  value={worktreesLimit}
                  onChange={(e) =>
                    setWorktreesLimit(Number(e.target.value) || 15)
                  }
                />
              }
            />
          </div>
          <p className="text-muted-foreground text-center text-[13px]">
            Worktrees created by OPENAKTA will appear here.
          </p>
        </div>
      </div>
    );
  }

  if (activeTab === "archived") {
    return (
      <div className="custom-scrollbar flex-1 overflow-y-auto bg-background">
        <div className="mx-auto max-w-[720px] px-8 py-10">
          <h1 className="text-foreground mb-8 text-[22px] font-semibold">
            {tabTitle}
          </h1>
          {archivedMock.length === 0 ? (
            <p className="text-muted-foreground text-center text-[14px]">
              No archived threads
            </p>
          ) : (
            <div className="border-border rounded-xl border">
              {archivedMock.map((t) => (
                <div
                  key={t.id}
                  className="flex items-center justify-between gap-4 border-b px-4 py-3 last:border-b-0"
                >
                  <div className="min-w-0">
                    <p className="truncate text-[13px] font-medium">
                      {t.title}
                    </p>
                    <p className="text-muted-foreground text-[11px]">
                      {new Date(t.archivedAt).toLocaleString()} · {t.projectId}
                    </p>
                  </div>
                  <Button size="sm" variant="outline">
                    Unarchive
                  </Button>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-1 flex-col items-center justify-center bg-background text-muted-foreground">
      <Settings className="mb-3 size-12 opacity-30" />
      <p className="text-[14px]">{tabTitle}</p>
    </div>
  );
}
