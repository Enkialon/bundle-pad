export function ensureMarkdownExtension(path: string): string {
  if (/\.(md|markdown)$/i.test(path)) return path;
  return `${path}.md`;
}
