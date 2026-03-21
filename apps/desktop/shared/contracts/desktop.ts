import { z } from "zod";

export const ipcChannels = {
  getAppInfo: "desktop:get-app-info",
  getShellState: "desktop:get-shell-state",
  getPreferences: "desktop:get-preferences",
  updatePreferences: "desktop:update-preferences",
  getFullscreenState: "desktop:get-fullscreen-state",
  onFullscreenChange: "desktop:on-fullscreen-change",
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
}
