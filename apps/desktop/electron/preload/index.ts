import { contextBridge, ipcRenderer } from "electron";

import {
  ipcChannels,
  type AppInfo,
  type DesktopApi,
  type DesktopPreferences,
  type DesktopPreferencesPatch,
  type ShellState,
} from "@/shared/contracts/desktop";

const api: DesktopApi = {
  app: {
    getInfo: () => ipcRenderer.invoke(ipcChannels.getAppInfo) as Promise<AppInfo>,
    getShellState: () =>
      ipcRenderer.invoke(ipcChannels.getShellState) as Promise<ShellState>,
    getFullscreenState: () =>
      ipcRenderer.invoke(ipcChannels.getFullscreenState) as Promise<boolean>,
    onFullscreenChange: (callback) => {
      const handler = (_event: Electron.IpcRendererEvent, isFullscreen: boolean) => callback(isFullscreen);
      ipcRenderer.on(ipcChannels.onFullscreenChange, handler);
      return () => {
        ipcRenderer.removeListener(ipcChannels.onFullscreenChange, handler);
      };
    },
  },
  preferences: {
    get: () =>
      ipcRenderer.invoke(ipcChannels.getPreferences) as Promise<DesktopPreferences>,
    update: (patch) =>
      ipcRenderer.invoke(
        ipcChannels.updatePreferences,
        patch satisfies DesktopPreferencesPatch,
      ) as Promise<DesktopPreferences>,
  },
};

contextBridge.exposeInMainWorld("axoraDesktop", api);
