import type { DesktopApi } from "@/shared/contracts/desktop";

declare global {
  interface Window {
    openaktaDesktop?: DesktopApi;
  }
}

export {};
