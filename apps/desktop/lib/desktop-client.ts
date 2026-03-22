import type {
  AppInfo,
  DesktopApi,
  DesktopPreferences,
  DesktopPreferencesPatch,
  ReviewDetail,
  ReviewQueueList,
  ReviewResolutionRequest,
  ReviewResolutionResponse,
  ShellState,
} from "@/shared/contracts/desktop";

function getDesktopApi(): DesktopApi | null {
  if (typeof window === "undefined") {
    return null;
  }

  return window.openaktaDesktop ?? null;
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
  async getPendingReviewCount(workspaceRoot = ""): Promise<number | null> {
    return getDesktopApi()?.reviews.getPendingCount(workspaceRoot) ?? null;
  },
  async listPendingReviews(input?: {
    workspaceRoot?: string;
    pageSize?: number;
    pageOffset?: number;
  }): Promise<ReviewQueueList | null> {
    return getDesktopApi()?.reviews.listPending(input) ?? null;
  },
  async getReviewDetail(reviewId: string): Promise<ReviewDetail | null> {
    return getDesktopApi()?.reviews.getDetail(reviewId) ?? null;
  },
  async submitReviewResolution(
    input: ReviewResolutionRequest,
  ): Promise<ReviewResolutionResponse | null> {
    return getDesktopApi()?.reviews.submitResolution(input) ?? null;
  },
  onFullscreenChange(callback: (isFullscreen: boolean) => void): () => void {
    return getDesktopApi()?.app.onFullscreenChange(callback) ?? (() => {});
  },
};
