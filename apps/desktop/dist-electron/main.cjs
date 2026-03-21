"use strict";

// electron/main/index.ts
var import_node_path2 = require("path");
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
  onFullscreenChange: "desktop:on-fullscreen-change"
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

// electron/main/preferences-store.ts
var import_promises = require("fs/promises");
var import_node_path = require("path");
var import_electron = require("electron");
var PREFERENCES_PATH = (0, import_node_path.join)(import_electron.app.getPath("userData"), "preferences.json");
async function ensurePreferencesDir() {
  await (0, import_promises.mkdir)((0, import_node_path.dirname)(PREFERENCES_PATH), { recursive: true });
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
  import_electron2.ipcMain.handle(ipcChannels.getAppInfo, async () => appInfoSchema.parse(info));
  import_electron2.ipcMain.handle(
    ipcChannels.getShellState,
    async () => shellStateSchema.parse(shellState2)
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
var rendererEntry = (0, import_node_path2.join)(__dirname, "../out/index.html");
var preloadPath = (0, import_node_path2.join)(__dirname, "preload.cjs");
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