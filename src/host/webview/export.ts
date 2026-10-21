import { invoke } from "@tauri-apps/api/core";
import type { ExportHost } from "../types.ts";

export function createWebviewExport(): ExportHost {
  return {
    exportPdf(path: string): Promise<void> {
      return invoke("export_pdf", { path });
    },
  };
}
