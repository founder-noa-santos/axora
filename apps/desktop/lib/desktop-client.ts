import type {
  AppInfo,
  DesktopApi,
  DesktopPreferences,
  DesktopPreferencesPatch,
  ShellState,
} from "@/shared/contracts/desktop";

function getDesktopApi(): DesktopApi | null {
  if (typeof window === "undefined") {
    return null;
  }

  return window.axoraDesktop ?? null;
}

export const desktopClient = {
  async getInfo(): Promise<AppInfo | null> {
    return getDesktopApi()?.app.getInfo() ?? null;
  },
  async getShellState(): Promise<ShellState | null> {
    return getDesktopApi()?.app.getShellState() ?? null;
  },
  async getPreferences(): Promise<DesktopPreferences | null> {
    return getDesktopApi()?.preferences.get() ?? null;
  },
  async updatePreferences(
    patch: DesktopPreferencesPatch,
  ): Promise<DesktopPreferences | null> {
    return getDesktopApi()?.preferences.update(patch) ?? null;
  },
  async getFullscreenState(): Promise<boolean | null> {
    return getDesktopApi()?.app.getFullscreenState() ?? null;
  },
  onFullscreenChange(callback: (isFullscreen: boolean) => void): () => void {
    return getDesktopApi()?.app.onFullscreenChange(callback) ?? (() => {});
  },
};
