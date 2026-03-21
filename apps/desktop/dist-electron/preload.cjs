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
  }
};
import_electron.contextBridge.exposeInMainWorld("openaktaDesktop", api);
//# sourceMappingURL=preload.cjs.map