import { z } from "zod";

/**
 * Renderer-side aggregate shell snapshot (UI contracts). This is distinct from
 * `ShellState` in `desktop.ts`, which is the minimal payload for `desktop:get-shell-state`.
 */
export const RustBridgeStatusSchema = z.enum([
  "not_connected",
  "planned",
  "connected",
]);

export const DaemonStatusSchema = z.enum(["unknown", "offline", "online"]);

export const ShellStateSchema = z.object({
  rustBridge: RustBridgeStatusSchema,
  daemon: z.object({
    status: DaemonStatusSchema,
    endpoint: z.string().url().optional(),
  }),
  preferences: z.object({
    themeMode: z.enum(["light", "dark", "system"]),
    compactSidebar: z.boolean(),
    reduceMotion: z.boolean(),
    commandCenterPinned: z.boolean(),
    launchAtLogin: z.boolean(),
  }),

  mission: z.object({
    id: z.string().nullable(),
    status: z
      .enum(["idle", "planning", "running", "paused", "complete", "error"])
      .default("idle"),
    agentCount: z.number().int().default(0),
    activeTaskId: z.string().nullable(),
  }),

  model: z.object({
    provider: z.enum(["openai", "local", "auto"]).default("auto"),
    modelId: z.string().default("gpt-4o"),
    contextWindowK: z.number().optional(),
  }),

  usage: z.object({
    fiveHourPercent: z.number().min(0).max(100).default(100),
    weeklyPercent: z.number().min(0).max(100).default(100),
    creditBalance: z.number().int().default(0),
  }),

  mcpRegistry: z.object({
    connected: z.boolean().default(false),
    serverCount: z.number().int().default(0),
  }),
});

export type ShellStateV2 = z.infer<typeof ShellStateSchema>;
