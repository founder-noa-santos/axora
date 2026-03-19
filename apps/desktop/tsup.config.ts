import { defineConfig } from "tsup";

export default defineConfig({
  entry: {
    main: "electron/main/index.ts",
    preload: "electron/preload/index.ts",
  },
  bundle: true,
  clean: false,
  external: ["electron"],
  format: ["cjs"],
  minify: process.env.NODE_ENV === "production",
  outDir: "dist-electron",
  outExtension: () => ({
    js: ".cjs",
  }),
  platform: "node",
  sourcemap: true,
  target: "node20",
});
