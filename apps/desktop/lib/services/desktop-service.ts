import type {
  DesktopPreferencesPatch,
  ReviewResolutionRequest,
} from "@/shared/contracts/desktop";

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
  getPendingReviewCount(workspaceRoot?: string) {
    return desktopClient.getPendingReviewCount(workspaceRoot);
  },
  listPendingReviews(input?: {
    workspaceRoot?: string;
    pageSize?: number;
    pageOffset?: number;
  }) {
    return desktopClient.listPendingReviews(input);
  },
  getReviewDetail(reviewId: string) {
    return desktopClient.getReviewDetail(reviewId);
  },
  submitReviewResolution(input: ReviewResolutionRequest) {
    return desktopClient.submitReviewResolution(input);
  },
  onFullscreenChange(callback: (isFullscreen: boolean) => void) {
    return desktopClient.onFullscreenChange(callback);
  },
};
