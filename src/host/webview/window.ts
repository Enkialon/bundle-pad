import { getCurrentWindow } from "@tauri-apps/api/window";
import type { WindowHost } from "../types.ts";

export function createWebviewWindow(): WindowHost {
  let closing = false;
  return {
    async setTitle(title: string): Promise<void> {
      await getCurrentWindow().setTitle(title);
    },
    noteUnsaved(_unsaved: boolean): void {},
    async requestClose(): Promise<void> {
      await getCurrentWindow().close();
    },
    async onCloseRequested(decide: () => Promise<boolean>): Promise<void> {
      await getCurrentWindow().onCloseRequested(async (event) => {
        if (closing) return;
        event.preventDefault();
        if (await decide()) {
          closing = true;
          await getCurrentWindow().destroy();
        }
      });
    },
    async onPathsDropped(handle: (paths: string[]) => void): Promise<void> {
      await getCurrentWindow().onDragDropEvent((event) => {
        if (event.payload.type !== "drop") return;
        handle(event.payload.paths);
      });
    },
  };
}
