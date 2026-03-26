"use strict";

// electron/preload/index.ts
var import_electron = require("electron");

// shared/contracts/desktop.ts
var import_zod = require("zod");
var ipcChannels = {
  getAppInfo: "desktop:get-app-info",
  getShellState: "desktop:get-shell-state",
  getPreferences: "desktop:get-preferences",
  updatePreferences: "desktop:update-preferences",
  getFullscreenState: "desktop:get-fullscreen-state",
  onFullscreenChange: "desktop:on-fullscreen-change",
  getPendingReviewCount: "desktop:get-pending-review-count",
  listPendingReviews: "desktop:list-pending-reviews",
  getReviewDetail: "desktop:get-review-detail",
  submitReviewResolution: "desktop:submit-review-resolution"
};
var appInfoSchema = import_zod.z.object({
  name: import_zod.z.string(),
  version: import_zod.z.string(),
  platform: import_zod.z.string(),
  arch: import_zod.z.string(),
  environment: import_zod.z.enum(["development", "production"])
});
var rustBridgeSchema = import_zod.z.object({
  status: import_zod.z.enum(["not_connected", "planned", "connected"]),
  transport: import_zod.z.enum(["ipc", "sidecar", "native"]).nullable(),
  note: import_zod.z.string()
});
var shellStateSchema = import_zod.z.object({
  rustBridge: rustBridgeSchema,
  daemon: import_zod.z.object({
    status: import_zod.z.enum(["unknown", "offline", "online"]),
    endpoint: import_zod.z.string().nullable()
  })
});
var reviewQueueItemSchema = import_zod.z.object({
  reviewId: import_zod.z.string(),
  reportId: import_zod.z.string(),
  workspaceRoot: import_zod.z.string(),
  createdAtMs: import_zod.z.number(),
  confidenceScore: import_zod.z.number(),
  primaryDocPath: import_zod.z.string(),
  highestSeverity: import_zod.z.string().optional(),
  summary: import_zod.z.string().optional()
});
var reviewQueueListSchema = import_zod.z.object({
  items: import_zod.z.array(reviewQueueItemSchema),
  totalPending: import_zod.z.number()
});
var reviewFlagSchema = import_zod.z.object({
  fingerprint: import_zod.z.string(),
  domain: import_zod.z.string(),
  kind: import_zod.z.string(),
  severity: import_zod.z.string(),
  docPath: import_zod.z.string(),
  codePath: import_zod.z.string().optional(),
  symbolName: import_zod.z.string().optional(),
  ruleIds: import_zod.z.array(import_zod.z.string()),
  message: import_zod.z.string(),
  expectedExcerpt: import_zod.z.string(),
  actualExcerpt: import_zod.z.string()
});
var reviewDetailSchema = import_zod.z.object({
  reviewId: import_zod.z.string(),
  reportId: import_zod.z.string(),
  workspaceRoot: import_zod.z.string(),
  createdAtMs: import_zod.z.number(),
  confidenceScore: import_zod.z.number(),
  primaryDocPath: import_zod.z.string(),
  highestSeverity: import_zod.z.string().optional(),
  summary: import_zod.z.string().optional(),
  flags: import_zod.z.array(reviewFlagSchema),
  breakdownJson: import_zod.z.string(),
  confidenceAuditActionId: import_zod.z.string().optional()
});
var reviewResolutionChoiceSchema = import_zod.z.enum([
  "update_doc",
  "update_code"
]);
var reviewResolutionRequestSchema = import_zod.z.object({
  reviewId: import_zod.z.string(),
  choice: reviewResolutionChoiceSchema,
  clientResolutionId: import_zod.z.string().uuid(),
  userNote: import_zod.z.string().optional()
});
var reviewResolutionResponseSchema = import_zod.z.object({
  serverResolutionId: import_zod.z.string(),
  outcome: import_zod.z.enum([
    "ok",
    "rejected",
    "conflict",
    "duplicate",
    "internal_error",
    "unspecified"
  ]),
  patchReceiptId: import_zod.z.string().optional(),
  toonChangelogEntryId: import_zod.z.string().optional()
});
var preferencesSchema = import_zod.z.object({
  themeMode: import_zod.z.enum(["dark", "system", "light"]),
  compactSidebar: import_zod.z.boolean(),
  reduceMotion: import_zod.z.boolean(),
  commandCenterPinned: import_zod.z.boolean(),
  launchAtLogin: import_zod.z.boolean()
});
var preferencesPatchSchema = preferencesSchema.partial();

// electron/preload/index.ts
var api = {
  app: {
    getInfo: () => import_electron.ipcRenderer.invoke(ipcChannels.getAppInfo),
    getShellState: () => import_electron.ipcRenderer.invoke(ipcChannels.getShellState),
    getFullscreenState: () => import_electron.ipcRenderer.invoke(ipcChannels.getFullscreenState),
    onFullscreenChange: (callback) => {
      const handler = (_event, isFullscreen) => callback(isFullscreen);
      import_electron.ipcRenderer.on(ipcChannels.onFullscreenChange, handler);
      return () => {
        import_electron.ipcRenderer.removeListener(ipcChannels.onFullscreenChange, handler);
      };
    }
  },
  preferences: {
    get: () => import_electron.ipcRenderer.invoke(
      ipcChannels.getPreferences
    ),
    update: (patch) => import_electron.ipcRenderer.invoke(
      ipcChannels.updatePreferences,
      patch
    )
  },
  reviews: {
    getPendingCount: (workspaceRoot) => import_electron.ipcRenderer.invoke(
      ipcChannels.getPendingReviewCount,
      workspaceRoot
    ),
    listPending: (input) => import_electron.ipcRenderer.invoke(
      ipcChannels.listPendingReviews,
      input
    ),
    getDetail: (reviewId) => import_electron.ipcRenderer.invoke(
      ipcChannels.getReviewDetail,
      reviewId
    ),
    submitResolution: (input) => import_electron.ipcRenderer.invoke(
      ipcChannels.submitReviewResolution,
      input
    )
  }
};
import_electron.contextBridge.exposeInMainWorld("openaktaDesktop", api);
//# sourceMappingURL=preload.cjs.map