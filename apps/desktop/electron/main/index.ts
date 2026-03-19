import { join } from "node:path";

import { app, BrowserWindow, nativeTheme, shell } from "electron";

import { registerIpcHandlers } from "./ipc";

const isDev = process.env.NODE_ENV === "development";
const rendererUrl = process.env.ELECTRON_RENDERER_URL;
const rendererEntry = join(__dirname, "../out/index.html");
const preloadPath = join(__dirname, "preload.cjs");

const appInfo = {
  name: "AXORA",
  version: app.getVersion(),
  platform: process.platform,
  arch: process.arch,
  environment: isDev ? "development" : "production",
} as const;

const shellState = {
  rustBridge: {
    status: "planned",
    transport: "ipc",
    note: "Electron main will broker future Rust daemon and crate access without exposing transport details to React.",
  },
  daemon: {
    status: "unknown",
    endpoint: null,
  },
} as const;

function createMainWindow() {
  const win = new BrowserWindow({
    width: 1480,
    height: 960,
    minWidth: 1180,
    minHeight: 780,
    show: false,
    backgroundColor: "#0b0d12",
    titleBarStyle: process.platform === "darwin" ? "hiddenInset" : "hidden",
    trafficLightPosition: process.platform === "darwin" ? { x: 18, y: 18 } : undefined,
    vibrancy: process.platform === "darwin" ? "under-window" : undefined,
    visualEffectState: process.platform === "darwin" ? "active" : undefined,
    webPreferences: {
      preload: preloadPath,
      contextIsolation: true,
      nodeIntegration: false,
      devTools: isDev,
      sandbox: true,
    },
  });

  win.once("ready-to-show", () => {
    win.show();
  });

  win.webContents.setWindowOpenHandler(({ url }) => {
    void shell.openExternal(url);
    return { action: "deny" };
  });

  if (rendererUrl) {
    void win.loadURL(rendererUrl);
  } else {
    void win.loadFile(rendererEntry);
  }

  return win;
}

async function bootstrap() {
  nativeTheme.themeSource = "dark";

  const mainWindow = createMainWindow();
  registerIpcHandlers(appInfo, shellState, mainWindow);

  if (isDev) {
    mainWindow.webContents.openDevTools({ mode: "detach" });
  }
}

app.whenReady().then(bootstrap);

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") {
    app.quit();
  }
});

app.on("activate", () => {
  if (BrowserWindow.getAllWindows().length === 0) {
    const mainWindow = createMainWindow();
    registerIpcHandlers(appInfo, shellState, mainWindow);
  }
});
