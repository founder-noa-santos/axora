import type { DesktopPreferencesPatch } from "@/shared/contracts/desktop";

import { desktopClient } from "@/lib/desktop-client";

export const desktopService = {
  getBootstrap() {
    return Promise.all([
      desktopClient.getInfo(),
      desktopClient.getShellState(),
      desktopClient.getPreferences(),
    ]).then(([info, shellState, preferences]) => ({
      info,
      shellState,
      preferences,
    }));
  },
  getPreferences() {
    return desktopClient.getPreferences();
  },
  updatePreferences(patch: DesktopPreferencesPatch) {
    return desktopClient.updatePreferences(patch);
  },
  getFullscreenState() {
    return desktopClient.getFullscreenState();
  },
  onFullscreenChange(callback: (isFullscreen: boolean) => void) {
    return desktopClient.onFullscreenChange(callback);
  },
};
