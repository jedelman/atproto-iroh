// Picks the real backend when running inside Tauri's webview
// (`withGlobalTauri: true` in tauri.conf.json injects `window.__TAURI__`)
// and the mock one otherwise — `npm run dev` in a plain browser, and
// every test, get the mock automatically with zero configuration.
import type { Client } from "./client";
import { tauriClient } from "./tauriClient";
import { mockClient } from "./mockClient";

const isTauri = typeof window !== "undefined" && "__TAURI__" in window;

export const api: Client = isTauri ? tauriClient : mockClient;
export type { Client } from "./client";
export * from "./types";
