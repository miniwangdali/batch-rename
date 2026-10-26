import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// See https://v2.tauri.app/start/frontend/vite/
export default defineConfig({
  plugins: [react()],
  // Keep Rust compiler errors visible.
  clearScreen: false,
  server: {
    // Tauri expects a fixed port (see devUrl in src-tauri/tauri.conf.json).
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  // The bundle is embedded in the app binary, so web chunk-size advice doesn't apply.
  build: { chunkSizeWarningLimit: 1024 },
});
