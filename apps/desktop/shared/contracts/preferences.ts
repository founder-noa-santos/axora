import { z } from "zod";

/** Local / future IPC — not wired to Electron preferences store in this phase. */
export const GeneralPreferencesSchema = z.object({
  defaultOpenDestination: z.enum(["cursor", "vscode", "finder"]),
  language: z.string().default("auto"),
  threadDetail: z.enum(["minimal", "steps", "steps_with_code"]),
  preventSleep: z.boolean(),
  requireCmdEnterForLongPrompts: z.boolean(),
  speed: z.enum(["standard", "fast"]),
  followUpBehavior: z.enum(["queue", "steer"]),
  notifications: z.object({
    completion: z.enum(["always", "unfocused", "never"]),
    permission: z.boolean(),
    question: z.boolean(),
  }),
});
export type GeneralPreferences = z.infer<typeof GeneralPreferencesSchema>;

export const AppearancePreferencesSchema = z.object({
  themeMode: z.enum(["light", "dark", "system"]),
  compactSidebar: z.boolean(),
  reduceMotion: z.boolean(),
  accentColor: z.string().regex(/^#[0-9a-fA-F]{6}$/),
  contrastLight: z.number().min(0).max(100),
  contrastDark: z.number().min(0).max(100),
  translucentSidebar: z.boolean(),
  uiFontSizePx: z.number().min(10).max(20),
  usePointerCursors: z.boolean(),
});
export type AppearancePreferences = z.infer<typeof AppearancePreferencesSchema>;

export const ConfigurationSchema = z.object({
  approvalPolicy: z.enum(["on_request", "always", "never"]),
  sandboxMode: z.enum(["read_only", "read_write", "full_access"]),
});
export type ConfigurationPreferences = z.infer<typeof ConfigurationSchema>;

export const PersonalizationSchema = z.object({
  personality: z.enum(["pragmatic", "collaborative", "verbose", "concise"]),
  customInstructions: z.string().max(4000),
});
export type PersonalizationPreferences = z.infer<typeof PersonalizationSchema>;

export const UsageStateSchema = z.object({
  fiveHourLimitPercent: z.number().min(0).max(100),
  fiveHourResetAt: z.string().datetime(),
  weeklyLimitPercent: z.number().min(0).max(100),
  weeklyResetAt: z.string().datetime(),
  creditBalance: z.number().int().min(0),
  autoReloadEnabled: z.boolean(),
  plan: z.enum(["free", "pro", "team"]),
});
export type UsageState = z.infer<typeof UsageStateSchema>;

export const McpServerSchema = z.object({
  id: z.string().uuid(),
  name: z.string().min(1),
  url: z.string().url().optional(),
  command: z.string().optional(),
  envVars: z.record(z.string(), z.string()),
  enabled: z.boolean(),
  source: z.enum(["custom", "recommended"]),
});
export type McpServer = z.infer<typeof McpServerSchema>;

export const McpRegistrySchema = z.object({
  servers: z.array(McpServerSchema),
});
export type McpRegistry = z.infer<typeof McpRegistrySchema>;

export const GitPreferencesSchema = z.object({
  branchPrefix: z.string().default("codex/"),
  prMergeMethod: z.enum(["merge", "squash"]),
  showPrIconsInSidebar: z.boolean(),
  alwaysForcePush: z.boolean(),
  createDraftPrs: z.boolean(),
  commitInstructions: z.string().max(2000),
  prInstructions: z.string().max(2000),
});
export type GitPreferences = z.infer<typeof GitPreferencesSchema>;

export const EnvironmentProjectSchema = z.object({
  id: z.string(),
  name: z.string(),
  org: z.string().optional(),
  environments: z.array(
    z.object({
      name: z.string(),
      configPath: z.string(),
    }),
  ),
});
export type EnvironmentProject = z.infer<typeof EnvironmentProjectSchema>;

export const WorktreesConfigSchema = z.object({
  autoDeleteEnabled: z.boolean().default(true),
  autoDeleteLimit: z.number().int().min(1).max(100).default(15),
});
export type WorktreesConfig = z.infer<typeof WorktreesConfigSchema>;

export const WorktreeSchema = z.object({
  id: z.string(),
  path: z.string(),
  branch: z.string().optional(),
  createdAt: z.string().datetime(),
});
export type WorktreeRow = z.infer<typeof WorktreeSchema>;

export const ArchivedThreadSchema = z.object({
  id: z.string(),
  title: z.string(),
  projectId: z.string(),
  archivedAt: z.string().datetime(),
});
export type ArchivedThread = z.infer<typeof ArchivedThreadSchema>;
