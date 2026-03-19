export const workspaceCollections = [
  { name: "Inbox", count: 6, active: true },
  { name: "Missions", count: 12, active: false },
  { name: "Runs", count: 28, active: false },
  { name: "Artifacts", count: 4, active: false },
] as const;

export const activityFeed = [
  {
    title: "Repository indexed",
    detail: "Repository map is ready for workspace navigation and future Rust-backed search.",
    time: "Just now",
  },
  {
    title: "Bridge contract wired",
    detail: "Electron preload now exposes a minimal, typed API surface to the renderer.",
    time: "4m ago",
  },
  {
    title: "Frontend reset complete",
    detail: "The previous Tauri/Vite shell has been replaced with the new Electron + Next renderer foundation.",
    time: "12m ago",
  },
] as const;

export const emptyStates = [
  {
    title: "Command center",
    body: "Future repository actions, daemon commands, and Rust-backed capabilities will surface here through the preload bridge.",
  },
  {
    title: "Workspace canvas",
    body: "Use this region for mission execution, diff review, search, and rich tooling once the backend contracts are connected.",
  },
] as const;
