import { invoke } from "@tauri-apps/api/core";
import type { FileStamp } from "../document/document-session.ts";

export interface AppError {
  code: string;
  message: string;
}

export interface OpenedMarkdown {
  path: string;
  displayName: string;
  content: string;
  hasBom: boolean;
  stamp: FileStamp;
}

export interface OpenedTextBundle extends OpenedMarkdown {
  textFileName: string;
  version: number;
}

export interface SavedAsset {
  relativePath: string;
  mimeType: string;
}

export interface AssetPayload {
  mimeType: string;
  dataBase64: string;
}

export interface ClassifiedPath {
  kind: "markdown" | "textbundle";
  path: string;
}

export function asAppError(error: unknown): AppError {
  if (typeof error === "string") {
    const parsed = parseError(error);
    if (parsed) return parsed;
    return { code: "error", message: error };
  }
  if (error && typeof error === "object") {
    const record = error as Record<string, unknown>;
    if (typeof record.message === "string") {
      return {
        code: typeof record.code === "string" ? record.code : "error",
        message: record.message,
      };
    }
  }
  return { code: "error", message: "发生未知错误" };
}

export function encodeBase64(bytes: Uint8Array): string {
  let binary = "";
  const chunk = 0x8000;
  for (let index = 0; index < bytes.length; index += chunk) {
    binary += String.fromCharCode(...bytes.subarray(index, index + chunk));
  }
  return btoa(binary);
}

export function decodeBase64(data: string): Uint8Array {
  const binary = atob(data);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}

export function launchPaths(): Promise<string[]> {
  return invoke("launch_paths");
}

export function classifyPath(path: string): Promise<ClassifiedPath> {
  return invoke("classify_path", { path });
}

export function openMarkdown(path: string): Promise<OpenedMarkdown> {
  return invoke("open_markdown", { path });
}

export function openTextBundle(path: string): Promise<OpenedTextBundle> {
  return invoke("open_textbundle", { path });
}

export function saveMarkdown(
  path: string,
  content: string,
  hasBom: boolean,
  expectedSha256: string | null,
  overwriteExternal: boolean,
): Promise<FileStamp> {
  return invoke("save_markdown", {
    path,
    content,
    hasBom,
    expectedSha256,
    overwriteExternal,
  });
}

export function saveTextBundle(
  path: string,
  content: string,
  hasBom: boolean,
  textFileName: string,
  expectedSha256: string | null,
  overwriteExternal: boolean,
): Promise<FileStamp> {
  return invoke("save_textbundle", {
    path,
    content,
    hasBom,
    textFileName,
    expectedSha256,
    overwriteExternal,
  });
}

export function saveBundleAsset(bundlePath: string, dataBase64: string): Promise<SavedAsset> {
  return invoke("save_bundle_asset", { bundlePath, dataBase64 });
}

export function saveMarkdownAsset(docPath: string, dataBase64: string): Promise<SavedAsset> {
  return invoke("save_markdown_asset", { docPath, dataBase64 });
}

export function readBundleAsset(bundlePath: string, relativePath: string): Promise<AssetPayload> {
  return invoke("read_bundle_asset", { bundlePath, relativePath });
}

export function readMarkdownAsset(docPath: string, reference: string): Promise<AssetPayload> {
  return invoke("read_markdown_asset", { docPath, reference });
}

export function importLocalImage(
  documentPath: string,
  documentKind: "markdown" | "textbundle",
  sourcePath: string,
): Promise<SavedAsset> {
  return invoke("import_local_image", { documentPath, documentKind, sourcePath });
}

function parseError(value: string): AppError | null {
  try {
    const parsed = JSON.parse(value) as Record<string, unknown>;
    if (typeof parsed.message === "string") {
      return {
        code: typeof parsed.code === "string" ? parsed.code : "error",
        message: parsed.message,
      };
    }
  } catch {
    return null;
  }
  return null;
}
