import type { FileStamp } from "../document/document-session.ts";

export interface AppError {
  code: string;
  message: string;
}

export interface OpenedDocument {
  kind: "markdown" | "textbundle";
  path: string;
  displayName: string;
  content: string;
  hasBom: boolean;
  stamp: FileStamp;
  textFileName: string | null;
  version: number | null;
}

export interface SavedAsset {
  relativePath: string;
  mimeType: string;
}

export interface AssetPayload {
  mimeType: string;
  dataBase64: string;
}

/** 文档读写。Tauri 走 IPC，浏览器走本地 HTTP，语义相同。 */
export interface DocumentHost {
  launchPaths(): Promise<string[]>;
  openDocument(path: string): Promise<OpenedDocument>;
  saveDocument(
    path: string,
    content: string,
    hasBom: boolean,
    textFileName: string | null,
    expectedSha256: string | null,
    overwriteExternal: boolean,
  ): Promise<FileStamp>;
  readAsset(documentPath: string, reference: string): Promise<AssetPayload>;
  writeAsset(documentPath: string, dataBase64: string): Promise<SavedAsset>;
  importLocalImage(documentPath: string, sourcePath: string): Promise<SavedAsset>;
}

/** 系统对话框。返回值与界面流程对齐，不暴露具体平台控件。 */
export interface DialogHost {
  showError(text: string): Promise<void>;
  showWarning(text: string): Promise<void>;
  confirmUnsaved(): Promise<"save" | "discard" | "cancel">;
  confirmExternalChange(): Promise<"reload" | "overwrite" | "cancel">;
  pickMarkdown(defaultPath?: string): Promise<string | null>;
  pickTextBundle(defaultPath?: string): Promise<string | null>;
  pickSaveMarkdown(defaultPath?: string): Promise<string | null>;
  pickSavePdf(defaultPath?: string): Promise<string | null>;
}

/** 窗口壳。关窗确认由界面给出，适配器负责真正关闭。 */
export interface WindowHost {
  setTitle(title: string): Promise<void>;
  noteUnsaved(unsaved: boolean): void;
  requestClose(): Promise<void>;
  onCloseRequested(decide: () => Promise<boolean>): Promise<void>;
  onPathsDropped(handle: (paths: string[]) => void): Promise<void>;
}

export interface ExportHost {
  exportPdf(path: string): Promise<void>;
}

export interface Host {
  documents: DocumentHost;
  dialogs: DialogHost;
  window: WindowHost;
  export: ExportHost;
}
