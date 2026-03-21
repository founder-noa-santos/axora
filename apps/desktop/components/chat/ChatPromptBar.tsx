"use client";

import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ChangeEvent,
  type DragEvent,
  type KeyboardEvent,
} from "react";
import {
  PromptInput,
  PromptInputBody,
  PromptInputButton,
  PromptInputFooter,
  PromptInputHeader,
  PromptInputSelect,
  PromptInputSelectContent,
  PromptInputSelectItem,
  PromptInputSelectTrigger,
  PromptInputSelectValue,
  PromptInputSubmit,
  PromptInputTextarea,
  PromptInputTools,
  usePromptInputAttachments,
} from "@/components/ai-elements/prompt-input";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
import { cn } from "@/lib/utils";
import {
  filterMockFiles,
  filterSlashCommands,
  parseActiveTrigger,
  type FileHit,
  SLASH_COMMANDS,
} from "@/components/chat/useComposerMentions";
import { ArrowUp, Shield, ShieldCheck, Plus, X } from "lucide-react";
import { SelectGroup, SelectLabel } from "@/components/ui/select";
import {
  HoverCard,
  HoverCardContent,
  HoverCardTrigger,
} from "@/components/ui/hover-card";
import { Badge } from "@/components/ui/badge";

const MODELS = [
  { id: "gpt-5.4", label: "GPT-5.4" },
  { id: "gpt-4.5", label: "GPT-4.5" },
  { id: "claude-3.7", label: "Claude 3.7" },
  { id: "gemini-2.0", label: "Gemini 2.0" },
] as const;

const EFFORTS = ["Low", "Medium", "High", "Maximum"] as const;
const RUNTIMES = ["Local", "Cloud", "Hybrid"] as const;
const BRANCHES = ["main", "develop", "feature/new", "bugfix/fix"] as const;

const ACCESS_OPTIONS = [
  { value: "Default permissions", icon: Shield },
  { value: "Full access", icon: ShieldCheck },
] as const;

/** Matches `dataTransferHasFiles` in prompt-input (DOMStringList has no `.includes`). */
function dataTransferHasFiles(dt: DataTransfer | null): boolean {
  if (!dt?.types) {
    return false;
  }
  return Array.from(dt.types as Iterable<string>).includes("Files");
}

function slashItemValue(id: string) {
  return `slash:${id}`;
}

function fileItemValue(path: string) {
  return `at:${encodeURIComponent(path)}`;
}

function parsePaletteItemValue(
  v: string,
): { kind: "slash"; id: string } | { kind: "at"; path: string } | null {
  if (v.startsWith("slash:")) {
    return { id: v.slice("slash:".length), kind: "slash" };
  }
  if (v.startsWith("at:")) {
    try {
      return {
        kind: "at",
        path: decodeURIComponent(v.slice("at:".length)),
      };
    } catch {
      return null;
    }
  }
  return null;
}

const pillSelectTriggerClass =
  "h-7 w-fit !w-fit shrink-0 rounded-full border-transparent bg-transparent px-2.5 text-[11px] font-medium text-muted-foreground shadow-none transition-colors hover:bg-muted/50 hover:text-foreground data-[state=open]:bg-muted/50 data-[state=open]:text-foreground aria-expanded:bg-muted/50 aria-expanded:text-foreground focus:outline-none focus:ring-0 focus:ring-offset-0 justify-start";

function ContextWindowIndicator() {
  const contextPercent = 62;
  const size = 16;
  const strokeWidth = 1.75;
  const radius = (size - strokeWidth) / 2;
  const circumference = 2 * Math.PI * radius;
  const dashOffset = circumference * (1 - contextPercent / 100);

  return (
    <HoverCard openDelay={120} closeDelay={80}>
      <HoverCardTrigger asChild>
        <button
          type="button"
          className="text-muted-foreground hover:text-foreground hover:bg-muted/50 flex size-7 items-center justify-center rounded-full bg-transparent transition-colors outline-none"
          aria-label="Context window"
        >
          <svg
            aria-hidden="true"
            viewBox={`0 0 ${size} ${size}`}
            className="size-4 -rotate-90"
          >
            <circle
              cx={size / 2}
              cy={size / 2}
              r={radius}
              fill="none"
              className="stroke-muted-foreground/20"
              strokeWidth={strokeWidth}
            />
            <circle
              cx={size / 2}
              cy={size / 2}
              r={radius}
              fill="none"
              className="stroke-foreground/80"
              strokeWidth={strokeWidth}
              strokeLinecap="round"
              strokeDasharray={circumference}
              strokeDashoffset={dashOffset}
            />
          </svg>
        </button>
      </HoverCardTrigger>
      <HoverCardContent
        align="center"
        sideOffset={8}
        className="w-[188px] px-2.5 py-2 text-center"
      >
        <div className="text-muted-foreground text-[10px] leading-none font-medium">
          Context window:
        </div>
        <div className="mt-1 text-[11px] font-medium leading-tight">
          62% full
        </div>
        <div className="mt-1.5 text-[13px] font-semibold leading-tight">
          160k / 258k tokens used
        </div>
        <div className="text-muted-foreground mt-1.5 text-[10px] leading-tight">
          Codex compacts context automatically
        </div>
      </HoverCardContent>
    </HoverCard>
  );
}

/** Opens the PromptInput hidden file input — must render inside `<PromptInput>`. */
function PromptAttachButton({ className }: { className?: string }) {
  const attachments = usePromptInputAttachments();
  return (
    <PromptInputButton
      type="button"
      className={cn(
        "text-muted-foreground hover:text-foreground hover:bg-muted/60 flex h-6 w-6 items-center justify-center rounded-full bg-transparent p-0 transition-colors",
        className,
      )}
      onClick={() => attachments.openFileDialog()}
    >
      <Plus className="size-3.5" strokeWidth={2.5} />
    </PromptInputButton>
  );
}

/** Attachment chips above the textarea; collapses when empty. Must render inside `<PromptInput>`. */
function PromptAttachmentPreviewStrip() {
  const attachments = usePromptInputAttachments();
  if (attachments.files.length === 0) {
    return null;
  }

  return (
    <PromptInputHeader className="w-full min-w-0 gap-1 overflow-x-auto overflow-y-hidden px-0 pt-0 pb-1 [-ms-overflow-style:none] [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
      <div className="text-muted-foreground flex min-w-0 flex-nowrap items-center gap-1 text-[11px]">
        {attachments.files.map((file) => {
          const isImage =
            typeof file.mediaType === "string" &&
            file.mediaType.startsWith("image/");
          return (
            <Badge
              key={file.id}
              variant="secondary"
              className="group border-border/30 relative flex min-h-7 max-w-[min(220px,70vw)] shrink-0 items-center gap-0.5 border bg-transparent py-1.5 px-1.5 font-medium normal-case tracking-normal text-muted-foreground shadow-none hover:bg-transparent"
            >
              {isImage && file.url ? (
                // eslint-disable-next-line @next/next/no-img-element -- blob: preview URLs from PromptInput attachments
                <img
                  alt=""
                  className="size-4 shrink-0 rounded-sm object-cover"
                  draggable={false}
                  src={file.url}
                />
              ) : null}
              <span className="min-w-0 flex-1 truncate">{file.filename}</span>
              <button
                type="button"
                className="text-muted-foreground hover:text-foreground absolute top-1/2 right-1.5 z-10 flex size-4 -translate-y-1/2 items-center justify-center rounded-full bg-panel/90 opacity-0 shadow-sm backdrop-blur-sm transition-opacity group-hover:opacity-100 hover:bg-muted/90 dark:bg-panel/85"
                aria-label={`Remove ${file.filename}`}
                onClick={(e) => {
                  e.stopPropagation();
                  attachments.remove(file.id);
                }}
              >
                <X className="size-2.5" strokeWidth={2.5} />
              </button>
            </Badge>
          );
        })}
      </div>
    </PromptInputHeader>
  );
}

function PromptSubmitButton({
  disabled,
  hasText,
}: {
  disabled?: boolean;
  hasText: boolean;
}) {
  return (
    <PromptInputSubmit
      disabled={disabled || !hasText}
      variant={hasText ? "primary" : "ghost"}
      size="icon-sm"
      className={cn(
        "ml-1 size-7 shrink-0 rounded-full shadow-sm transition-colors",
        !hasText &&
          "border-0 border-transparent bg-muted/75 text-foreground/65 shadow-none opacity-100 hover:bg-muted/75 disabled:opacity-100 disabled:hover:bg-muted/75",
      )}
    >
      <ArrowUp
        className={cn("size-3.5", !hasText && "opacity-90")}
        strokeWidth={2.5}
      />
    </PromptInputSubmit>
  );
}

export function ChatPromptBar({
  onSubmit,
  disabled,
  className,
}: {
  onSubmit: (text: string) => void | Promise<void>;
  disabled?: boolean;
  className?: string;
}) {
  const [model, setModel] = useState<string>(MODELS[0].id);
  const [effort, setEffort] = useState<string>("Medium");
  const [runtime, setRuntime] = useState<string>("Local");
  const [access, setAccess] =
    useState<(typeof ACCESS_OPTIONS)[number]["value"]>("Full access");
  const [branch, setBranch] = useState<string>("main");
  const [message, setMessage] = useState<string>("");
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const messageRef = useRef(message);
  const caretRef = useRef(0);
  const paletteSuppressedRef = useRef(false);
  const paletteSelectionRef = useRef("");

  const [caretOffset, setCaretOffset] = useState(0);
  const [paletteSuppressed, setPaletteSuppressed] = useState(false);
  const [paletteSelection, setPaletteSelection] = useState("");
  const [composerFileDragOver, setComposerFileDragOver] = useState(false);

  const [activeMenu, setActiveMenu] = useState<
    "model" | "effort" | "runtime" | "access" | "branch" | null
  >(null);

  useEffect(() => {
    messageRef.current = message;
  }, [message]);

  useEffect(() => {
    paletteSuppressedRef.current = paletteSuppressed;
  }, [paletteSuppressed]);

  useEffect(() => {
    caretRef.current = caretOffset;
  }, [caretOffset]);

  useEffect(() => {
    paletteSelectionRef.current = paletteSelection;
  }, [paletteSelection]);

  useEffect(() => {
    const clearComposerFileDrag = () => setComposerFileDragOver(false);
    window.addEventListener("dragend", clearComposerFileDrag);
    // `drop` bubbles to `document`, not `window`; Electron often skips `dragend` after OS → web drop.
    document.addEventListener("drop", clearComposerFileDrag);
    return () => {
      window.removeEventListener("dragend", clearComposerFileDrag);
      document.removeEventListener("drop", clearComposerFileDrag);
    };
  }, []);

  const handleComposerDragOver = (e: DragEvent<HTMLDivElement>) => {
    if (!dataTransferHasFiles(e.dataTransfer)) {
      return;
    }
    e.preventDefault();
    e.dataTransfer.dropEffect = "copy";
    setComposerFileDragOver(true);
  };

  const handleComposerDragLeave = (e: DragEvent<HTMLDivElement>) => {
    const related = e.relatedTarget as Node | null;
    if (related && e.currentTarget.contains(related)) {
      return;
    }
    setComposerFileDragOver(false);
  };

  const handleComposerDrop = () => {
    setComposerFileDragOver(false);
  };

  const parsedTrigger = useMemo(
    () => parseActiveTrigger(message, caretOffset),
    [message, caretOffset],
  );

  const paletteVisible = parsedTrigger !== null && !paletteSuppressed;

  const slashFiltered = useMemo(
    () =>
      parsedTrigger?.kind === "slash"
        ? filterSlashCommands(parsedTrigger.query)
        : [],
    [parsedTrigger],
  );

  const fileHits = useMemo((): FileHit[] => {
    if (!parsedTrigger || parsedTrigger.kind !== "at") {
      return [];
    }
    return filterMockFiles(parsedTrigger.query);
  }, [parsedTrigger]);

  const reconcilePaletteSelection = useCallback((text: string, pos: number) => {
    const t = parseActiveTrigger(text, pos);
    if (t) {
      const vals =
        t.kind === "slash"
          ? filterSlashCommands(t.query).map((c) => slashItemValue(c.id))
          : filterMockFiles(t.query).map((f) => fileItemValue(f.path));
      setPaletteSelection((prev) =>
        vals.includes(prev) ? prev : (vals[0] ?? ""),
      );
    } else {
      setPaletteSelection("");
    }
  }, []);

  const applyPaletteSelection = useCallback(
    (value: string) => {
      const text = messageRef.current;
      const caret = caretRef.current;
      const t = parseActiveTrigger(text, caret);
      if (!t) {
        return;
      }
      const parsed = parsePaletteItemValue(value);
      if (!parsed) {
        return;
      }
      let insert = "";
      if (parsed.kind === "slash") {
        const cmd = SLASH_COMMANDS.find((c) => c.id === parsed.id);
        if (!cmd) {
          return;
        }
        insert = `${cmd.insert} `;
      } else {
        insert = `@${parsed.path} `;
      }
      const before = text.slice(0, t.start);
      const after = text.slice(t.caret);
      const next = before + insert + after;
      const pos = t.start + insert.length;
      setMessage(next);
      setCaretOffset(pos);
      caretRef.current = pos;
      setPaletteSuppressed(false);
      reconcilePaletteSelection(next, pos);
      requestAnimationFrame(() => {
        const el = textareaRef.current;
        if (el) {
          el.focus();
          el.setSelectionRange(pos, pos);
        }
      });
    },
    [reconcilePaletteSelection],
  );

  const handleMessageChange = (e: ChangeEvent<HTMLTextAreaElement>) => {
    setPaletteSuppressed(false);
    const newText = e.currentTarget.value;
    const pos = e.currentTarget.selectionStart ?? 0;
    setMessage(newText);
    setCaretOffset(pos);
    caretRef.current = pos;
    reconcilePaletteSelection(newText, pos);
  };

  const syncCaretFromTextarea = (el: HTMLTextAreaElement) => {
    const pos = el.selectionStart ?? 0;
    setCaretOffset(pos);
    caretRef.current = pos;
    reconcilePaletteSelection(el.value, pos);
  };

  const handleTextareaKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.nativeEvent.isComposing || e.keyCode === 229) {
      return;
    }

    const pos = e.currentTarget.selectionStart ?? 0;
    const t = parseActiveTrigger(messageRef.current, pos);
    caretRef.current = pos;

    if (e.key === "Escape" && t) {
      e.preventDefault();
      setPaletteSuppressed(true);
      return;
    }

    const visible = t && !paletteSuppressedRef.current;
    if (!visible) {
      return;
    }

    const values =
      t.kind === "slash"
        ? filterSlashCommands(t.query).map((c) => slashItemValue(c.id))
        : filterMockFiles(t.query).map((f) => fileItemValue(f.path));

    if (e.key === "ArrowDown") {
      e.preventDefault();
      if (values.length === 0) {
        return;
      }
      setPaletteSelection((prev) => {
        const idx = values.indexOf(prev);
        const base = idx < 0 ? 0 : idx;
        const nextIdx = Math.min(values.length - 1, base + 1);
        return values[nextIdx]!;
      });
      return;
    }

    if (e.key === "ArrowUp") {
      e.preventDefault();
      if (values.length === 0) {
        return;
      }
      setPaletteSelection((prev) => {
        const idx = values.indexOf(prev);
        const base = idx < 0 ? 0 : idx;
        const nextIdx = Math.max(0, base - 1);
        return values[nextIdx]!;
      });
      return;
    }

    if (e.key === "Enter" && !e.shiftKey) {
      if (values.length === 0) {
        return;
      }
      e.preventDefault();
      const current =
        paletteSelectionRef.current &&
        values.includes(paletteSelectionRef.current)
          ? paletteSelectionRef.current
          : values[0];
      if (current) {
        applyPaletteSelection(current);
      }
    }
  };

  const handleMenuOpenChange =
    (menu: "model" | "effort" | "runtime" | "access" | "branch") =>
    (open: boolean) => {
      setActiveMenu((current) => {
        if (open) {
          return menu;
        }
        return current === menu ? null : current;
      });
    };

  // `InputGroup` defaults to `items-center`; with `flex-col` that shrinks children to content
  // width and clips the textarea. This composer is always vertical, so force the shell
  // into the official PromptInput stacked layout and stretch the children.
  const inputGroupShell =
    "[&_[data-slot=input-group]]:!h-auto [&_[data-slot=input-group]]:!flex-col [&_[data-slot=input-group]]:!items-stretch [&_[data-slot=input-group]]:rounded-[24px] [&_[data-slot=input-group]]:border [&_[data-slot=input-group]]:border-border/30 [&_[data-slot=input-group]]:bg-panel/95 [&_[data-slot=input-group]]:shadow-[0_8px_24px_-20px_rgba(15,23,42,0.16)] [&_[data-slot=input-group]]:backdrop-blur-xl [&_[data-slot=input-group]]:px-2.5 [&_[data-slot=input-group]]:pt-2 [&_[data-slot=input-group]]:pb-[6px] [&_[data-slot=input-group]]:transition-colors";

  return (
    <div
      className={cn(
        "from-background via-background pointer-events-auto shrink-0 bg-linear-to-t to-transparent pb-6 pt-2",
        className,
      )}
      data-no-drag="true"
    >
      <div className="mx-auto w-full max-w-3xl px-6">
        <div className="relative isolate w-full">
          {/* Focus stays on the textarea; cmdk selection is driven by controlled `value` plus ArrowUp/ArrowDown/Enter handled in `handleTextareaKeyDown`. */}
          {paletteVisible ? (
            <div
              className="pointer-events-auto absolute bottom-full left-0 right-0 z-50 mb-1 max-h-[min(320px,40vh)] overflow-hidden rounded-[24px] border border-border/30 bg-panel/95 text-popover-foreground shadow-none backdrop-blur-xl supports-[backdrop-filter]:bg-panel/90"
              role="presentation"
            >
              <Command
                className="h-auto max-h-[min(320px,40vh)] rounded-none border-0 bg-transparent shadow-none"
                label={
                  parsedTrigger?.kind === "at"
                    ? "Insert file reference"
                    : "Run command"
                }
                loop
                onValueChange={setPaletteSelection}
                shouldFilter={false}
                value={paletteSelection}
              >
                <CommandList className="max-h-[min(280px,36vh)]">
                  <CommandGroup
                    heading={
                      parsedTrigger?.kind === "at" ? "Files" : "Commands"
                    }
                  >
                    {parsedTrigger?.kind === "slash"
                      ? slashFiltered.map((c) => (
                          <CommandItem
                            key={c.id}
                            value={slashItemValue(c.id)}
                            onSelect={() =>
                              applyPaletteSelection(slashItemValue(c.id))
                            }
                          >
                            <span className="text-muted-foreground font-mono text-xs">
                              {c.insert}
                            </span>
                            <span>{c.label}</span>
                          </CommandItem>
                        ))
                      : fileHits.map((f) => (
                          <CommandItem
                            key={f.path}
                            value={fileItemValue(f.path)}
                            onSelect={() =>
                              applyPaletteSelection(fileItemValue(f.path))
                            }
                          >
                            {f.label ?? f.path}
                          </CommandItem>
                        ))}
                  </CommandGroup>
                  <CommandEmpty>No results.</CommandEmpty>
                </CommandList>
              </Command>
            </div>
          ) : null}
          <div
            className="relative w-full"
            onDragLeave={handleComposerDragLeave}
            onDragOver={handleComposerDragOver}
            onDrop={handleComposerDrop}
          >
            {composerFileDragOver ? (
              <div
                aria-hidden
                className="pointer-events-none absolute inset-0 z-[5] flex items-center justify-center rounded-[24px] bg-muted/35 text-sm font-medium text-muted-foreground backdrop-blur-[1px]"
              >
                Upload file
              </div>
            ) : null}
            <PromptInput
              globalDrop
              multiple
              className={cn("w-full", inputGroupShell)}
              onSubmit={async (msg) => {
                const text = msg.text?.trim() ?? "";
                if (!text) return;
                // msg.files are shown as chips only; parent onSubmit is text-only until IPC supports uploads.
                await onSubmit(text);
                setMessage("");
              }}
            >
              <PromptAttachmentPreviewStrip />
              <PromptInputBody>
                <PromptInputTextarea
                  ref={textareaRef}
                  disabled={disabled}
                  name="message"
                  placeholder="Ask Openakta anything, @ to add files, / for commands"
                  value={message}
                  onChange={handleMessageChange}
                  onClick={(e) => syncCaretFromTextarea(e.currentTarget)}
                  onKeyDown={handleTextareaKeyDown}
                  onKeyUp={(e) => syncCaretFromTextarea(e.currentTarget)}
                  onSelect={(e) => syncCaretFromTextarea(e.currentTarget)}
                  className="text-foreground/90 placeholder:text-muted-foreground/70 min-h-[64px] w-full min-w-0 self-stretch px-2.5 py-1.5 text-[15px] leading-7"
                />
              </PromptInputBody>
              <PromptInputFooter className="mt-[4px] flex w-full shrink-0 items-center justify-between gap-3 px-0.5 !pt-0 !pb-[2px]">
                <PromptInputTools className="flex min-w-0 items-center gap-1.5">
                  <PromptAttachButton />

                  <PromptInputSelect
                    value={model}
                    onValueChange={(value) => {
                      setModel(value);
                      setActiveMenu(null);
                    }}
                    disabled={disabled}
                    open={activeMenu === "model"}
                    onOpenChange={handleMenuOpenChange("model")}
                  >
                    <PromptInputSelectTrigger
                      size="sm"
                      className={pillSelectTriggerClass}
                    >
                      <PromptInputSelectValue placeholder={MODELS[0].label} />
                    </PromptInputSelectTrigger>
                    <PromptInputSelectContent
                      className="z-50"
                      position="popper"
                    >
                      <SelectGroup>
                        <SelectLabel className="px-2 pt-1 pb-0 text-sm font-medium leading-none text-muted-foreground">
                          Select model
                        </SelectLabel>
                        {MODELS.map((m) => (
                          <PromptInputSelectItem
                            key={m.id}
                            value={m.id}
                            className="text-xs"
                          >
                            {m.label}
                          </PromptInputSelectItem>
                        ))}
                      </SelectGroup>
                    </PromptInputSelectContent>
                  </PromptInputSelect>

                  <PromptInputSelect
                    value={effort}
                    onValueChange={(value) => {
                      setEffort(value);
                      setActiveMenu(null);
                    }}
                    disabled={disabled}
                    open={activeMenu === "effort"}
                    onOpenChange={handleMenuOpenChange("effort")}
                  >
                    <PromptInputSelectTrigger
                      size="sm"
                      className={pillSelectTriggerClass}
                    >
                      <PromptInputSelectValue placeholder="Medium" />
                    </PromptInputSelectTrigger>
                    <PromptInputSelectContent
                      className="z-50"
                      position="popper"
                    >
                      <SelectGroup>
                        <SelectLabel className="px-2 pt-1 pb-0 text-sm font-medium leading-none text-muted-foreground">
                          Select effort
                        </SelectLabel>
                        {EFFORTS.map((e) => (
                          <PromptInputSelectItem
                            key={e}
                            value={e}
                            className="text-xs"
                          >
                            {e}
                          </PromptInputSelectItem>
                        ))}
                      </SelectGroup>
                    </PromptInputSelectContent>
                  </PromptInputSelect>
                </PromptInputTools>

                <PromptSubmitButton
                  disabled={disabled}
                  hasText={message.trim().length > 0}
                />
              </PromptInputFooter>
            </PromptInput>
          </div>
        </div>

        <div className="text-muted-foreground mt-3 flex items-center justify-between px-2 text-[11px] font-medium">
          <div className="flex items-center gap-4">
            <PromptInputSelect
              value={runtime}
              onValueChange={(value) => {
                setRuntime(value as (typeof RUNTIMES)[number]);
                setActiveMenu(null);
              }}
              disabled={disabled}
              open={activeMenu === "runtime"}
              onOpenChange={handleMenuOpenChange("runtime")}
            >
              <PromptInputSelectTrigger
                size="sm"
                className={pillSelectTriggerClass}
              >
                <PromptInputSelectValue placeholder="Local" />
              </PromptInputSelectTrigger>
              <PromptInputSelectContent
                align="start"
                side="top"
                className="min-w-[140px]"
              >
                {RUNTIMES.map((r) => (
                  <PromptInputSelectItem key={r} className="text-xs" value={r}>
                    {r}
                  </PromptInputSelectItem>
                ))}
              </PromptInputSelectContent>
            </PromptInputSelect>

            <PromptInputSelect
              value={access}
              onValueChange={(value) => {
                setAccess(value as (typeof ACCESS_OPTIONS)[number]["value"]);
                setActiveMenu(null);
              }}
              disabled={disabled}
              open={activeMenu === "access"}
              onOpenChange={handleMenuOpenChange("access")}
            >
              <PromptInputSelectTrigger
                size="sm"
                className={pillSelectTriggerClass}
              >
                <PromptInputSelectValue placeholder="Full access" />
              </PromptInputSelectTrigger>
              <PromptInputSelectContent
                align="start"
                side="top"
                className="min-w-[160px]"
              >
                {ACCESS_OPTIONS.map((opt) => (
                  <PromptInputSelectItem
                    key={opt.value}
                    className="text-xs"
                    value={opt.value}
                  >
                    <span
                      className={cn(
                        "flex items-center gap-2",
                        access === opt.value && "text-foreground",
                      )}
                    >
                      <opt.icon className="size-3.5 shrink-0 text-muted-foreground/80" />
                      {opt.value}
                    </span>
                  </PromptInputSelectItem>
                ))}
              </PromptInputSelectContent>
            </PromptInputSelect>
          </div>

          <div className="flex items-center gap-1.5">
            <PromptInputSelect
              value={branch}
              onValueChange={(value) => {
                setBranch(value as (typeof BRANCHES)[number]);
                setActiveMenu(null);
              }}
              disabled={disabled}
              open={activeMenu === "branch"}
              onOpenChange={handleMenuOpenChange("branch")}
            >
              <PromptInputSelectTrigger
                size="sm"
                className={pillSelectTriggerClass}
              >
                <PromptInputSelectValue placeholder="main" />
              </PromptInputSelectTrigger>
              <PromptInputSelectContent
                align="end"
                side="top"
                className="min-w-[140px]"
              >
                {BRANCHES.map((b) => (
                  <PromptInputSelectItem key={b} className="text-xs" value={b}>
                    {b}
                  </PromptInputSelectItem>
                ))}
              </PromptInputSelectContent>
            </PromptInputSelect>

            <ContextWindowIndicator />
          </div>
        </div>
      </div>
    </div>
  );
}
