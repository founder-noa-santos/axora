/**
 * Composer @ / trigger parsing and pluggable file search (mocked until IPC/indexer).
 * No UI — consumed by ChatPromptBar.
 */

export type ComposerTriggerKind = "at" | "slash";

/** Active same-line token: trigger char immediately followed by optional non-whitespace query; caret ends the segment. */
export type ActiveComposerTrigger = {
  kind: ComposerTriggerKind;
  /** index of @ or / in message */
  start: number;
  /** characters after @ or / up to caret (no whitespace) */
  query: string;
  caret: number;
};

export type SlashCommand = {
  id: string;
  label: string;
  /** Full replacement for the /… segment (includes leading `/`) */
  insert: string;
};

export const SLASH_COMMANDS: SlashCommand[] = [
  { id: "search", label: "Search codebase", insert: "/search" },
  { id: "fix", label: "Fix issues", insert: "/fix" },
  { id: "explain", label: "Explain selection", insert: "/explain" },
  { id: "test", label: "Generate tests", insert: "/test" },
  { id: "docs", label: "Update documentation", insert: "/docs" },
  { id: "refactor", label: "Refactor", insert: "/refactor" },
];

export function filterSlashCommands(query: string): SlashCommand[] {
  const q = query.toLowerCase();
  if (!q) {
    return SLASH_COMMANDS;
  }
  return SLASH_COMMANDS.filter(
    (c) => c.id.toLowerCase().includes(q) || c.label.toLowerCase().includes(q),
  );
}

export type FileHit = { path: string; label?: string };

const MOCK_FILES: FileHit[] = [
  { path: "apps/desktop/components/chat/ChatPromptBar.tsx" },
  { path: "apps/desktop/components/chat/ChatView.tsx" },
  { path: "apps/desktop/components/ui/command.tsx" },
  { path: "crates/openakta-core/src/config.rs" },
  { path: "crates/openakta-agents/src/lib.rs" },
  { path: "README.md" },
  { path: "package.json" },
  { path: "apps/desktop/app/layout.tsx" },
];

/** Synchronous filter for the mock file list (use in UI); keep `searchFiles` for async/IPC backends. */
export function filterMockFiles(query: string): FileHit[] {
  const q = query.toLowerCase();
  if (!q) {
    return MOCK_FILES;
  }
  return MOCK_FILES.filter((f) => f.path.toLowerCase().includes(q));
}

/**
 * Swap for a real indexer / IPC-backed search later.
 */
export async function searchFiles(query: string): Promise<FileHit[]> {
  await Promise.resolve();
  return filterMockFiles(query);
}

function isWhitespace(ch: string | undefined): boolean {
  return ch === " " || ch === "\t" || ch === "\n" || ch === "\r";
}

/**
 * Returns the active @ or / token when the caret sits at the end of a valid segment on the current line.
 */
export function parseActiveTrigger(
  text: string,
  caret: number,
): ActiveComposerTrigger | null {
  if (caret < 1) {
    return null;
  }

  let lineStart = 0;
  for (let i = caret - 1; i >= 0; i--) {
    const c = text[i];
    if (c === "\n" || c === "\r") {
      lineStart = i + 1;
      break;
    }
  }

  let triggerOffset = -1;
  let triggerChar: "@" | "/" | null = null;
  const lineToCaret = text.slice(lineStart, caret);
  for (let i = lineToCaret.length - 1; i >= 0; i--) {
    const ch = lineToCaret[i];
    if (ch === "@" || ch === "/") {
      triggerOffset = lineStart + i;
      triggerChar = ch;
      break;
    }
  }

  if (triggerOffset < 0 || !triggerChar) {
    return null;
  }

  if (triggerOffset > 0) {
    const prev = text[triggerOffset - 1];
    if (prev && !isWhitespace(prev)) {
      return null;
    }
  }

  const query = text.slice(triggerOffset + 1, caret);
  if (/\s/.test(query)) {
    return null;
  }

  return {
    caret,
    kind: triggerChar === "@" ? "at" : "slash",
    query,
    start: triggerOffset,
  };
}
