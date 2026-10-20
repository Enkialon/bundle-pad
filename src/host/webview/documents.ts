import { invoke } from "@tauri-apps/api/core";
import type { FileStamp } from "../../document/document-session.ts";
import type { AssetPayload, DocumentHost, OpenedDocument, SavedAsset } from "../types.ts";

export function createWebviewDocuments(): DocumentHost {
  return {
    launchPaths(): Promise<string[]> {
      return invoke("launch_paths");
    },
    openDocument(path: string): Promise<OpenedDocument> {
      return invoke("open_document", { path });
    },
    saveDocument(
      path: string,
      content: string,
      hasBom: boolean,
      textFileName: string | null,
      expectedSha256: string | null,
      overwriteExternal: boolean,
    ): Promise<FileStamp> {
      return invoke("save_document", {
        path,
        content,
        hasBom,
        textFileName,
        expectedSha256,
        overwriteExternal,
      });
    },
    readAsset(documentPath: string, reference: string): Promise<AssetPayload> {
      return invoke("read_asset", { documentPath, reference });
    },
    writeAsset(documentPath: string, dataBase64: string): Promise<SavedAsset> {
      return invoke("write_asset", { documentPath, dataBase64 });
    },
    importLocalImage(documentPath: string, sourcePath: string): Promise<SavedAsset> {
      return invoke("import_local_image", { documentPath, sourcePath });
    },
  };
}
