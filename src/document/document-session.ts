export type DocumentKind = "untitled" | "markdown" | "textbundle";

export interface FileStamp {
  sha256: string;
  len: number;
  modifiedMs: number;
}

export class DocumentSession {
  kind: DocumentKind = "untitled";
  path: string | null = null;
  displayName = "未命名";
  hasBom = false;
  textFileName: string | null = null;
  stamp: FileStamp | null = null;
  baseline = "";
  dirty = false;

  resetUntitled(): void {
    this.kind = "untitled";
    this.path = null;
    this.displayName = "未命名";
    this.hasBom = false;
    this.textFileName = null;
    this.stamp = null;
    this.baseline = "";
    this.dirty = false;
  }

  noteMarkdown(markdown: string): void {
    this.dirty = markdown !== this.baseline;
  }

  markSaved(markdown: string, stamp: FileStamp): void {
    this.baseline = markdown;
    this.stamp = stamp;
    this.dirty = false;
  }

  kindLabel(): string {
    if (this.kind === "textbundle") return "TextBundle";
    if (this.kind === "markdown") return "Markdown";
    return "未命名";
  }

  stateLabel(): string {
    return this.dirty ? "未保存" : "已保存";
  }

  title(): string {
    return `BundlePad — ${this.displayName}${this.dirty ? " *" : ""}`;
  }
}
