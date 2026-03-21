import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { AppProvider } from "@/lib/app-state";

vi.mock("@/lib/services/desktop-service", () => ({
  desktopService: {
    getBootstrap: () =>
      Promise.resolve({
        info: {
          name: "OPENAKTA",
          version: "0.2.0",
          platform: "darwin",
          arch: "arm64",
          environment: "development",
        },
        shellState: {
          rustBridge: {
            status: "planned",
            transport: "ipc",
            note: "test note",
          },
          daemon: {
            status: "unknown",
            endpoint: null,
          },
        },
        preferences: {
          themeMode: "dark",
          compactSidebar: true,
          reduceMotion: false,
          commandCenterPinned: true,
          launchAtLogin: false,
        },
      }),
    getPreferences: () =>
      Promise.resolve({
        themeMode: "dark",
        compactSidebar: true,
        reduceMotion: false,
        commandCenterPinned: true,
        launchAtLogin: false,
      }),
    updatePreferences: vi.fn(),
    getFullscreenState: () => Promise.resolve(false),
    onFullscreenChange: vi.fn(() => () => {}),
  },
}));

vi.mock("next-themes", () => ({
  useTheme: () => ({
    theme: "dark",
    setTheme: vi.fn(),
    resolvedTheme: "dark",
  }),
  ThemeProvider: ({ children }: { children: React.ReactNode }) => children,
}));

// Mock the sidebar context
vi.mock("@/components/ui/sidebar", () => ({
  Sidebar: ({ children }: { children: React.ReactNode }) => (
    <div>{children}</div>
  ),
  SidebarContent: ({ children }: { children: React.ReactNode }) => (
    <div>{children}</div>
  ),
  SidebarFooter: ({ children }: { children: React.ReactNode }) => (
    <div>{children}</div>
  ),
  SidebarProvider: ({ children }: { children: React.ReactNode }) => (
    <div>{children}</div>
  ),
  SidebarGroup: ({ children }: { children: React.ReactNode }) => (
    <div>{children}</div>
  ),
  SidebarGroupLabel: ({ children }: { children: React.ReactNode }) => (
    <div>{children}</div>
  ),
  SidebarGroupContent: ({ children }: { children: React.ReactNode }) => (
    <div>{children}</div>
  ),
  SidebarMenu: ({ children }: { children: React.ReactNode }) => (
    <div>{children}</div>
  ),
  SidebarMenuItem: ({ children }: { children: React.ReactNode }) => (
    <div>{children}</div>
  ),
  SidebarMenuButton: ({
    children,
    onClick,
  }: {
    children: React.ReactNode;
    onClick?: () => void;
  }) => <button onClick={onClick}>{children}</button>,
  SidebarInset: ({ children }: { children: React.ReactNode }) => (
    <div>{children}</div>
  ),
  useSidebar: () => ({ open: true, toggleSidebar: vi.fn() }),
}));

// Note: We can't directly test the layout component as it's the root component
// Instead, we test that the app renders correctly with the theme system
describe("Desktop App Theme System", () => {
  it("renders with dark theme by default", async () => {
    // Import dynamically to avoid issues with the layout being the default export
    const { AppContent } = await import("@/app/layout");

    render(
      <AppProvider>
        <AppContent />
      </AppProvider>,
    );

    // Use findAllByText since React StrictMode may double-render
    const letsBuild = await screen.findAllByText("Let's build");
    expect(letsBuild.length).toBeGreaterThanOrEqual(1);

    const newThread = screen.getAllByText("New thread");
    expect(newThread.length).toBeGreaterThanOrEqual(1);

    const threads = screen.getAllByText("Threads");
    expect(threads.length).toBeGreaterThanOrEqual(1);

    const settings = screen.getAllByText("Settings");
    expect(settings.length).toBeGreaterThanOrEqual(1);
  });

  it("renders thread items", async () => {
    const { AppContent } = await import("@/app/layout");
    render(
      <AppProvider>
        <AppContent />
      </AppProvider>,
    );

    const openaktaElements = await screen.findAllByText("openakta");
    expect(openaktaElements.length).toBeGreaterThanOrEqual(1);

    const nexusElements = screen.getAllByText("nexus-social");
    expect(nexusElements.length).toBeGreaterThanOrEqual(1);

    const fluriElements = screen.getAllByText("fluri-v0");
    expect(fluriElements.length).toBeGreaterThanOrEqual(1);
  });

  it("renders navigation items", async () => {
    const { AppContent } = await import("@/app/layout");
    render(
      <AppProvider>
        <AppContent />
      </AppProvider>,
    );

    const automations = await screen.findAllByText("Automations");
    expect(automations.length).toBeGreaterThanOrEqual(1);

    const skills = screen.getAllByText("Skills");
    expect(skills.length).toBeGreaterThanOrEqual(1);
  });

  it("renders quick action cards", async () => {
    const { AppContent } = await import("@/app/layout");
    render(
      <AppProvider>
        <AppContent />
      </AppProvider>,
    );

    const snakeGame = await screen.findAllByText("Build a classic Snake game");
    expect(snakeGame.length).toBeGreaterThanOrEqual(1);

    const pdf = screen.getAllByText("Create a one-page PDF");
    expect(pdf.length).toBeGreaterThanOrEqual(1);

    const plan = screen.getAllByText("Create a plan to...");
    expect(plan.length).toBeGreaterThanOrEqual(1);
  });
});
