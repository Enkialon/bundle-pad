import { isTauri } from "@tauri-apps/api/core";
import type { Host } from "./types.ts";

export async function createHost(): Promise<Host> {
  if (!isTauriRuntime()) {
    throw new Error("当前仅支持在 Tauri 桌面窗口中运行。");
  }
  const { createWebviewHost } = await import("./webview/host.ts");
  return createWebviewHost();
}

function isTauriRuntime(): boolean {
  if (isTauri()) return true;
  const internals = (globalThis as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  return typeof internals === "object" && internals !== null;
}
