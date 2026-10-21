export function ensureMarkdownExtension(path: string): string {
  if (/\.(md|markdown)$/i.test(path)) return path;
  return `${path}.md`;
}

export function ensurePdfExtension(path: string): string {
  if (/\.pdf$/i.test(path)) return path;
  return `${path}.pdf`;
}

export function defaultPdfPath(displayName: string, documentPath?: string | null): string {
  const baseName = displayName.replace(/\.(md|markdown|textbundle|textpack)$/i, "") || "未命名";
  const fileName = `${baseName}.pdf`;
  if (!documentPath) return fileName;
  const separator = documentPath.includes("\\") ? "\\" : "/";
  const parts = documentPath.split(/[/\\]/);
  parts[parts.length - 1] = fileName;
  return parts.join(separator);
}
