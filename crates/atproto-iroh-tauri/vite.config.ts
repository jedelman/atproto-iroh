/// <reference types="vitest/config" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri-conventional dev port (tauri.conf.json's devUrl); fixed rather
// than left to Vite's auto-increment so `cargo tauri dev` can rely on it
// actually being there.
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [react()],
  // Tauri expects the built frontend at ../dist relative to src-tauri —
  // tauri.conf.json's build.frontendDist already points there.
  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: {
      // Don't rebuild on changes to the Rust side or generated Android project.
      ignored: ["**/src-tauri/**"],
    },
  },
  test: {
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
    globals: true,
  },
});
