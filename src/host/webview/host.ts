import type { Host } from "../types.ts";
import { createWebviewDialogs } from "./dialogs.ts";
import { createWebviewDocuments } from "./documents.ts";
import { createWebviewExport } from "./export.ts";
import { createWebviewWindow } from "./window.ts";

export function createWebviewHost(): Host {
  return {
    documents: createWebviewDocuments(),
    dialogs: createWebviewDialogs(),
    window: createWebviewWindow(),
    export: createWebviewExport(),
  };
}
