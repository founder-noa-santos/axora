"use strict";
var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// electron/main/index.ts
var import_node_path3 = require("path");
var import_electron3 = require("electron");

// electron/main/ipc.ts
var import_electron2 = require("electron");

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
var defaultPreferences = {
  themeMode: "dark",
  compactSidebar: true,
  reduceMotion: false,
  commandCenterPinned: true,
  launchAtLogin: false
};

// electron/main/livingdocs-review-client.ts
var import_node_path = require("path");
var import_node_util = require("util");
var grpc = __toESM(require("@grpc/grpc-js"), 1);
var protoLoader = __toESM(require("@grpc/proto-loader"), 1);
var PROTO_PATH = (0, import_node_path.join)(
  __dirname,
  "..",
  "..",
  "..",
  "proto",
  "livingdocs",
  "v1",
  "review.proto"
);
var packageDefinition = protoLoader.loadSync(PROTO_PATH, {
  longs: Number,
  enums: String,
  defaults: true,
  oneofs: true
});
var loaded = grpc.loadPackageDefinition(packageDefinition);
var choiceToProto = {
  update_doc: 1,
  update_code: 2
};
var LivingDocsReviewClient = class {
  endpoint;
  client;
  constructor(endpoint = daemonEndpoint()) {
    this.endpoint = endpoint;
    this.client = new loaded.livingdocs.v1.LivingDocsReviewService(
      endpoint,
      grpc.credentials.createInsecure()
    );
  }
  async getShellState() {
    try {
      await this.getPendingReviewCount("");
      return {
        rustBridge: {
          status: "connected",
          transport: "ipc",
          note: "Electron main brokers LivingDocs review gRPC calls to the daemon."
        },
        daemon: {
          status: "online",
          endpoint: this.endpoint
        }
      };
    } catch {
      return {
        rustBridge: {
          status: "connected",
          transport: "ipc",
          note: "Electron main brokers LivingDocs review gRPC calls to the daemon."
        },
        daemon: {
          status: "offline",
          endpoint: this.endpoint
        }
      };
    }
  }
  async getPendingReviewCount(workspaceRoot = "") {
    const call = (0, import_node_util.promisify)(this.client.getPendingReviewCount.bind(this.client));
    const response = await call({ workspaceRoot });
    return response.count ?? 0;
  }
  async listPendingReviews(input) {
    const call = (0, import_node_util.promisify)(this.client.listPendingReviews.bind(this.client));
    const response = await call({
      workspaceRoot: input?.workspaceRoot ?? "",
      pageSize: input?.pageSize ?? 50,
      pageOffset: input?.pageOffset ?? 0
    });
    return reviewQueueListSchema.parse({
      items: response.items ?? [],
      totalPending: response.totalPending ?? 0
    });
  }
  async getReviewDetail(reviewId) {
    const call = (0, import_node_util.promisify)(this.client.getReviewDetail.bind(this.client));
    const response = await call({ reviewId });
    if (!response.header) {
      throw new Error("missing review detail header");
    }
    return reviewDetailSchema.parse({
      ...response.header,
      flags: response.flags ?? [],
      breakdownJson: response.breakdownJson,
      confidenceAuditActionId: response.confidenceAuditActionId
    });
  }
  async submitResolution(input) {
    const safeInput = reviewResolutionRequestSchema.parse(input);
    const call = (0, import_node_util.promisify)(this.client.submitResolution.bind(this.client));
    const response = await call({
      reviewId: safeInput.reviewId,
      choice: choiceToProto[safeInput.choice],
      clientResolutionId: safeInput.clientResolutionId,
      userNote: safeInput.userNote
    });
    return reviewResolutionResponseSchema.parse({
      serverResolutionId: response.serverResolutionId,
      outcome: normalizeOutcome(response.outcome),
      patchReceiptId: response.patchReceiptId,
      toonChangelogEntryId: response.toonChangelogEntryId
    });
  }
};
function daemonEndpoint() {
  const raw = process.env.OPENAKTA_MCP_ENDPOINT ?? process.env.OPENAKTA_REVIEW_DAEMON_ENDPOINT ?? "http://127.0.0.1:50061";
  return raw.replace(/^https?:\/\//, "");
}
function normalizeOutcome(value) {
  switch ((value ?? "").toLowerCase()) {
    case "ok":
    case "resolution_outcome_ok":
      return "ok";
    case "rejected":
    case "resolution_outcome_rejected":
      return "rejected";
    case "conflict":
    case "resolution_outcome_conflict":
      return "conflict";
    case "duplicate":
    case "resolution_outcome_duplicate":
      return "duplicate";
    case "internal_error":
    case "resolution_outcome_internal_error":
      return "internal_error";
    default:
      return "unspecified";
  }
}

// electron/main/preferences-store.ts
var import_promises = require("fs/promises");
var import_node_path2 = require("path");
var import_electron = require("electron");
var PREFERENCES_PATH = (0, import_node_path2.join)(import_electron.app.getPath("userData"), "preferences.json");
async function ensurePreferencesDir() {
  await (0, import_promises.mkdir)((0, import_node_path2.dirname)(PREFERENCES_PATH), { recursive: true });
}
async function readPreferences() {
  try {
    const raw = await (0, import_promises.readFile)(PREFERENCES_PATH, "utf8");
    return preferencesSchema.parse(JSON.parse(raw));
  } catch {
    await writePreferences(defaultPreferences);
    return defaultPreferences;
  }
}
async function writePreferences(patch) {
  const safePatch = preferencesPatchSchema.parse(patch);
  const current = await readPreferences();
  const next = preferencesSchema.parse({ ...current, ...safePatch });
  await ensurePreferencesDir();
  await (0, import_promises.writeFile)(PREFERENCES_PATH, JSON.stringify(next, null, 2), "utf8");
  return next;
}

// electron/main/ipc.ts
function registerIpcHandlers(info, shellState2, mainWindow) {
  const reviewClient = new LivingDocsReviewClient();
  import_electron2.ipcMain.handle(ipcChannels.getAppInfo, async () => appInfoSchema.parse(info));
  import_electron2.ipcMain.handle(
    ipcChannels.getShellState,
    async () => shellStateSchema.parse(await reviewClient.getShellState())
  );
  import_electron2.ipcMain.handle(ipcChannels.getPreferences, async () => readPreferences());
  import_electron2.ipcMain.handle(
    ipcChannels.updatePreferences,
    async (_event, payload) => writePreferences(payload)
  );
  import_electron2.ipcMain.handle(
    ipcChannels.getFullscreenState,
    async () => mainWindow.isFullScreen()
  );
  import_electron2.ipcMain.handle(
    ipcChannels.getPendingReviewCount,
    async (_event, workspaceRoot) => reviewClient.getPendingReviewCount(workspaceRoot)
  );
  import_electron2.ipcMain.handle(
    ipcChannels.listPendingReviews,
    async (_event, input) => reviewClient.listPendingReviews(input)
  );
  import_electron2.ipcMain.handle(
    ipcChannels.getReviewDetail,
    async (_event, reviewId) => reviewClient.getReviewDetail(reviewId)
  );
  import_electron2.ipcMain.handle(
    ipcChannels.submitReviewResolution,
    async (_event, payload) => reviewClient.submitResolution(reviewResolutionRequestSchema.parse(payload))
  );
  mainWindow.on("enter-full-screen", () => {
    mainWindow.webContents.send(ipcChannels.onFullscreenChange, true);
  });
  mainWindow.on("leave-full-screen", () => {
    mainWindow.webContents.send(ipcChannels.onFullscreenChange, false);
  });
}

// electron/main/index.ts
var isDev = process.env.NODE_ENV === "development";
var rendererUrl = process.env.ELECTRON_RENDERER_URL;
var rendererEntry = (0, import_node_path3.join)(__dirname, "../out/index.html");
var preloadPath = (0, import_node_path3.join)(__dirname, "preload.cjs");
var appInfo = {
  name: "OPENAKTA",
  version: import_electron3.app.getVersion(),
  platform: process.platform,
  arch: process.arch,
  environment: isDev ? "development" : "production"
};
var shellState = {
  rustBridge: {
    status: "planned",
    transport: "ipc",
    note: "Electron main will broker future Rust daemon and crate access without exposing transport details to React."
  },
  daemon: {
    status: "unknown",
    endpoint: null
  }
};
function createMainWindow() {
  const win = new import_electron3.BrowserWindow({
    width: 1480,
    height: 960,
    minWidth: 1180,
    minHeight: 780,
    show: false,
    backgroundColor: "#0b0d12",
    titleBarStyle: process.platform === "darwin" ? "hiddenInset" : "hidden",
    trafficLightPosition: process.platform === "darwin" ? { x: 18, y: 18 } : void 0,
    vibrancy: process.platform === "darwin" ? "under-window" : void 0,
    visualEffectState: process.platform === "darwin" ? "active" : void 0,
    webPreferences: {
      preload: preloadPath,
      contextIsolation: true,
      nodeIntegration: false,
      devTools: isDev,
      sandbox: true
    }
  });
  win.once("ready-to-show", () => {
    win.show();
  });
  win.webContents.setWindowOpenHandler(({ url }) => {
    void import_electron3.shell.openExternal(url);
    return { action: "deny" };
  });
  if (rendererUrl) {
    void win.loadURL(rendererUrl);
  } else {
    void win.loadFile(rendererEntry);
  }
  return win;
}
async function bootstrap() {
  import_electron3.nativeTheme.themeSource = "dark";
  const mainWindow = createMainWindow();
  registerIpcHandlers(appInfo, shellState, mainWindow);
  if (isDev) {
    mainWindow.webContents.openDevTools({ mode: "detach" });
  }
}
import_electron3.app.whenReady().then(bootstrap);
import_electron3.app.on("window-all-closed", () => {
  if (process.platform !== "darwin") {
    import_electron3.app.quit();
  }
});
import_electron3.app.on("activate", () => {
  if (import_electron3.BrowserWindow.getAllWindows().length === 0) {
    const mainWindow = createMainWindow();
    registerIpcHandlers(appInfo, shellState, mainWindow);
  }
});
//# sourceMappingURL=main.cjs.map