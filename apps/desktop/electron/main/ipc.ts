import { BrowserWindow, ipcMain } from "electron";

import {
  appInfoSchema,
  ipcChannels,
  reviewResolutionRequestSchema,
  shellStateSchema,
  type AppInfo,
  type ShellState,
} from "@/shared/contracts/desktop";

import { LivingDocsReviewClient } from "./livingdocs-review-client";
import { readPreferences, writePreferences } from "./preferences-store";

export function registerIpcHandlers(
  info: AppInfo,
  shellState: ShellState,
  mainWindow: BrowserWindow,
) {
  const reviewClient = new LivingDocsReviewClient();

  ipcMain.handle(ipcChannels.getAppInfo, async () => appInfoSchema.parse(info));
  ipcMain.handle(ipcChannels.getShellState, async () =>
    shellStateSchema.parse(await reviewClient.getShellState()),
  );
  ipcMain.handle(ipcChannels.getPreferences, async () => readPreferences());
  ipcMain.handle(ipcChannels.updatePreferences, async (_event, payload) =>
    writePreferences(payload),
  );
  ipcMain.handle(ipcChannels.getFullscreenState, async () =>
    mainWindow.isFullScreen(),
  );
  ipcMain.handle(
    ipcChannels.getPendingReviewCount,
    async (_event, workspaceRoot?: string) =>
      reviewClient.getPendingReviewCount(workspaceRoot),
  );
  ipcMain.handle(
    ipcChannels.listPendingReviews,
    async (
      _event,
      input?: { workspaceRoot?: string; pageSize?: number; pageOffset?: number },
    ) => reviewClient.listPendingReviews(input),
  );
  ipcMain.handle(ipcChannels.getReviewDetail, async (_event, reviewId: string) =>
    reviewClient.getReviewDetail(reviewId),
  );
  ipcMain.handle(
    ipcChannels.submitReviewResolution,
    async (_event, payload: unknown) =>
      reviewClient.submitResolution(reviewResolutionRequestSchema.parse(payload)),
  );

  // Listen for fullscreen changes and notify renderer
  mainWindow.on("enter-full-screen", () => {
    mainWindow.webContents.send(ipcChannels.onFullscreenChange, true);
  });
  mainWindow.on("leave-full-screen", () => {
    mainWindow.webContents.send(ipcChannels.onFullscreenChange, false);
  });
}
