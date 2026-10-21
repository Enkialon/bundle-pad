import type { FileStamp } from "../../document/document-session.ts";
import { ensureMarkdownExtension, ensurePdfExtension } from "../markdown-path.ts";
import type {
  AssetPayload,
  DialogHost,
  DocumentHost,
  ExportHost,
  Host,
  OpenedDocument,
  SavedAsset,
  WindowHost,
} from "../types.ts";
import { postJson } from "./client.ts";

export function createBrowserHost(): Host {
  const dialogs = createBrowserDialogs();
  return {
    documents: createBrowserDocuments(),
    dialogs,
    window: createBrowserWindow(dialogs),
    export: createBrowserExport(),
  };
}

function createBrowserDocuments(): DocumentHost {
  return {
    async launchPaths(): Promise<string[]> {
      const fromQuery = new URLSearchParams(window.location.search).get("path");
      return fromQuery ? [fromQuery] : [];
    },
    openDocument(path: string): Promise<OpenedDocument> {
      return postJson("/api/open-document", { path });
    },
    saveDocument(
      path: string,
      content: string,
      hasBom: boolean,
      textFileName: string | null,
      expectedSha256: string | null,
      overwriteExternal: boolean,
    ): Promise<FileStamp> {
      return postJson("/api/save-document", {
        path,
        content,
        hasBom,
        textFileName,
        expectedSha256,
        overwriteExternal,
      });
    },
    readAsset(documentPath: string, reference: string): Promise<AssetPayload> {
      return postJson("/api/read-asset", { documentPath, reference });
    },
    writeAsset(documentPath: string, dataBase64: string): Promise<SavedAsset> {
      return postJson("/api/write-asset", { documentPath, dataBase64 });
    },
    importLocalImage(documentPath: string, sourcePath: string): Promise<SavedAsset> {
      return postJson("/api/import-local-image", { documentPath, sourcePath });
    },
  };
}

function createBrowserDialogs(): DialogHost {
  return {
    async showError(text: string): Promise<void> {
      await postJson("/api/show-error", { text });
    },
    async showWarning(text: string): Promise<void> {
      await postJson("/api/show-warning", { text });
    },
    confirmUnsaved(): Promise<"save" | "discard" | "cancel"> {
      return postJson<"save" | "discard" | "cancel">("/api/confirm-unsaved");
    },
    confirmExternalChange(): Promise<"reload" | "overwrite" | "cancel"> {
      return postJson<"reload" | "overwrite" | "cancel">("/api/confirm-external-change");
    },
    async pickMarkdown(defaultPath?: string): Promise<string | null> {
      const result = await postJson<{ path: string | null }>("/api/pick-markdown", { defaultPath: defaultPath ?? null });
      return result.path;
    },
    async pickTextBundle(defaultPath?: string): Promise<string | null> {
      const result = await postJson<{ path: string | null }>("/api/pick-textbundle", { defaultPath: defaultPath ?? null });
      return result.path;
    },
    async pickSaveMarkdown(defaultPath?: string): Promise<string | null> {
      const result = await postJson<{ path: string | null }>("/api/pick-save-markdown", { defaultPath: defaultPath ?? null });
      if (!result.path) return null;
      return ensureMarkdownExtension(result.path);
    },
    async pickSavePdf(defaultPath?: string): Promise<string | null> {
      const result = await postJson<{ path: string | null }>("/api/pick-save-pdf", { defaultPath: defaultPath ?? null });
      if (!result.path) return null;
      return ensurePdfExtension(result.path);
    },
  };
}

function createBrowserExport(): ExportHost {
  return {
    async exportPdf(_path: string): Promise<void> {
      window.print();
    },
  };
}

function createBrowserWindow(dialogs: DialogHost): WindowHost {
  let unsaved = false;
  let allowUnload = false;
  let decide: () => Promise<boolean> = async () => true;

  window.addEventListener("beforeunload", (event) => {
    if (allowUnload || !unsaved) return;
    event.preventDefault();
  });

  return {
    async setTitle(title: string): Promise<void> {
      document.title = title;
    },
    noteUnsaved(next: boolean): void {
      unsaved = next;
    },
    async requestClose(): Promise<void> {
      if (await decide()) {
        allowUnload = true;
        window.close();
      }
    },
    async onCloseRequested(next: () => Promise<boolean>): Promise<void> {
      decide = next;
    },
    async onPathsDropped(handle: (paths: string[]) => void): Promise<void> {
      window.addEventListener("dragover", (event) => {
        if (!event.dataTransfer) return;
        event.preventDefault();
      });
      window.addEventListener("drop", (event) => {
        event.preventDefault();
        const files = [...(event.dataTransfer?.files ?? [])];
        const paths = files.flatMap((file) => {
          const candidate = (file as File & { path?: string }).path;
          return candidate ? [candidate] : [];
        });
        if (files.length > 0 && paths.length === 0) {
          void dialogs.showError("浏览器不能从拖放取得本地路径，请使用打开菜单。");
          return;
        }
        handle(paths);
      });
    },
  };
}
