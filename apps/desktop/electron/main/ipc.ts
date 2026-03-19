import { BrowserWindow, ipcMain } from "electron";

import {
  appInfoSchema,
  ipcChannels,
  shellStateSchema,
  type AppInfo,
  type ShellState,
} from "@/shared/contracts/desktop";

import { readPreferences, writePreferences } from "./preferences-store";

export function registerIpcHandlers(info: AppInfo, shellState: ShellState, mainWindow: BrowserWindow) {
  ipcMain.handle(ipcChannels.getAppInfo, async () => appInfoSchema.parse(info));
  ipcMain.handle(ipcChannels.getShellState, async () =>
    shellStateSchema.parse(shellState),
  );
  ipcMain.handle(ipcChannels.getPreferences, async () => readPreferences());
  ipcMain.handle(ipcChannels.updatePreferences, async (_event, payload) =>
    writePreferences(payload),
  );
  ipcMain.handle(ipcChannels.getFullscreenState, async () => mainWindow.isFullScreen());
  
  // Listen for fullscreen changes and notify renderer
  mainWindow.on("enter-full-screen", () => {
    mainWindow.webContents.send(ipcChannels.onFullscreenChange, true);
  });
  mainWindow.on("leave-full-screen", () => {
    mainWindow.webContents.send(ipcChannels.onFullscreenChange, false);
  });
}
