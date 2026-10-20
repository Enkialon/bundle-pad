import { isTauri } from "@tauri-apps/api/core";
import type { Host } from "./types.ts";

export async function createHost(): Promise<Host> {
  if (isTauriRuntime()) {
    const { createWebviewHost } = await import("./webview/host.ts");
    return createWebviewHost();
  }
  const { createBrowserHost } = await import("./browser/host.ts");
  return createBrowserHost();
}

function isTauriRuntime(): boolean {
  if (isTauri()) return true;
  const internals = (globalThis as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  return typeof internals === "object" && internals !== null;
}
