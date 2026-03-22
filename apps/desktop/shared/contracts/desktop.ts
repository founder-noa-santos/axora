import { z } from "zod";

export const ipcChannels = {
  getAppInfo: "desktop:get-app-info",
  getShellState: "desktop:get-shell-state",
  getPreferences: "desktop:get-preferences",
  updatePreferences: "desktop:update-preferences",
  getFullscreenState: "desktop:get-fullscreen-state",
  onFullscreenChange: "desktop:on-fullscreen-change",
  getPendingReviewCount: "desktop:get-pending-review-count",
  listPendingReviews: "desktop:list-pending-reviews",
  getReviewDetail: "desktop:get-review-detail",
  submitReviewResolution: "desktop:submit-review-resolution",
} as const;

export const appInfoSchema = z.object({
  name: z.string(),
  version: z.string(),
  platform: z.string(),
  arch: z.string(),
  environment: z.enum(["development", "production"]),
});

export const rustBridgeSchema = z.object({
  status: z.enum(["not_connected", "planned", "connected"]),
  transport: z.enum(["ipc", "sidecar", "native"]).nullable(),
  note: z.string(),
});

export const shellStateSchema = z.object({
  rustBridge: rustBridgeSchema,
  daemon: z.object({
    status: z.enum(["unknown", "offline", "online"]),
    endpoint: z.string().nullable(),
  }),
});

export const reviewQueueItemSchema = z.object({
  reviewId: z.string(),
  reportId: z.string(),
  workspaceRoot: z.string(),
  createdAtMs: z.number(),
  confidenceScore: z.number(),
  primaryDocPath: z.string(),
  highestSeverity: z.string().optional(),
  summary: z.string().optional(),
});

export const reviewQueueListSchema = z.object({
  items: z.array(reviewQueueItemSchema),
  totalPending: z.number(),
});

export const reviewFlagSchema = z.object({
  fingerprint: z.string(),
  domain: z.string(),
  kind: z.string(),
  severity: z.string(),
  docPath: z.string(),
  codePath: z.string().optional(),
  symbolName: z.string().optional(),
  ruleIds: z.array(z.string()),
  message: z.string(),
  expectedExcerpt: z.string(),
  actualExcerpt: z.string(),
});

export const reviewDetailSchema = z.object({
  reviewId: z.string(),
  reportId: z.string(),
  workspaceRoot: z.string(),
  createdAtMs: z.number(),
  confidenceScore: z.number(),
  primaryDocPath: z.string(),
  highestSeverity: z.string().optional(),
  summary: z.string().optional(),
  flags: z.array(reviewFlagSchema),
  breakdownJson: z.string(),
  confidenceAuditActionId: z.string().optional(),
});

export const reviewResolutionChoiceSchema = z.enum([
  "update_doc",
  "update_code",
]);

export const reviewResolutionRequestSchema = z.object({
  reviewId: z.string(),
  choice: reviewResolutionChoiceSchema,
  clientResolutionId: z.string().uuid(),
  userNote: z.string().optional(),
});

export const reviewResolutionResponseSchema = z.object({
  serverResolutionId: z.string(),
  outcome: z.enum([
    "ok",
    "rejected",
    "conflict",
    "duplicate",
    "internal_error",
    "unspecified",
  ]),
  patchReceiptId: z.string().optional(),
  toonChangelogEntryId: z.string().optional(),
});

export const preferencesSchema = z.object({
  themeMode: z.enum(["dark", "system", "light"]),
  compactSidebar: z.boolean(),
  reduceMotion: z.boolean(),
  commandCenterPinned: z.boolean(),
  launchAtLogin: z.boolean(),
});

export const preferencesPatchSchema = preferencesSchema.partial();

export type AppInfo = z.infer<typeof appInfoSchema>;
export type RustBridge = z.infer<typeof rustBridgeSchema>;
export type ShellState = z.infer<typeof shellStateSchema>;
export type ReviewQueueItem = z.infer<typeof reviewQueueItemSchema>;
export type ReviewQueueList = z.infer<typeof reviewQueueListSchema>;
export type ReviewFlag = z.infer<typeof reviewFlagSchema>;
export type ReviewDetail = z.infer<typeof reviewDetailSchema>;
export type ReviewResolutionChoice = z.infer<typeof reviewResolutionChoiceSchema>;
export type ReviewResolutionRequest = z.infer<typeof reviewResolutionRequestSchema>;
export type ReviewResolutionResponse = z.infer<typeof reviewResolutionResponseSchema>;
export type DesktopPreferences = z.infer<typeof preferencesSchema>;
export type DesktopPreferencesPatch = z.infer<typeof preferencesPatchSchema>;

export const defaultPreferences: DesktopPreferences = {
  themeMode: "dark",
  compactSidebar: true,
  reduceMotion: false,
  commandCenterPinned: true,
  launchAtLogin: false,
};

export interface DesktopApi {
  app: {
    getInfo: () => Promise<AppInfo>;
    getShellState: () => Promise<ShellState>;
    getFullscreenState: () => Promise<boolean>;
    onFullscreenChange: (
      callback: (isFullscreen: boolean) => void,
    ) => () => void;
  };
  preferences: {
    get: () => Promise<DesktopPreferences>;
    update: (patch: DesktopPreferencesPatch) => Promise<DesktopPreferences>;
  };
  reviews: {
    getPendingCount: (workspaceRoot?: string) => Promise<number>;
    listPending: (input?: {
      workspaceRoot?: string;
      pageSize?: number;
      pageOffset?: number;
    }) => Promise<ReviewQueueList>;
    getDetail: (reviewId: string) => Promise<ReviewDetail>;
    submitResolution: (
      input: ReviewResolutionRequest,
    ) => Promise<ReviewResolutionResponse>;
  };
}
