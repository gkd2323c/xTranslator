import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "path";

const isE2E = process.env.VITE_E2E === "true";

// https://vitejs.dev/config/
export default defineConfig(async () => ({
  plugins: [react()],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 5173,
    strictPort: true,
    watch: {
      // 3. tell vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },

  // Resolve aliases: mock Tauri APIs in E2E test mode
  resolve: isE2E
    ? {
        alias: {
          "@tauri-apps/api/core": path.resolve(__dirname, "e2e/mocks/tauri-core.ts"),
          "@tauri-apps/api/event": path.resolve(__dirname, "e2e/mocks/tauri-event.ts"),
          "@tauri-apps/api/webview": path.resolve(__dirname, "e2e/mocks/tauri-webview.ts"),
        },
      }
    : undefined,

  // Vitest configuration
  test: {
    globals: true,
    environment: "jsdom",
    setupFiles: [],
    include: ["src/**/*.{test,spec}.{ts,tsx}"],
  },
}));
