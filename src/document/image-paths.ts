import type { Definition, Html, Image, Root } from "mdast";
import remarkParse from "remark-parse";
import { unified } from "unified";
import { visit } from "unist-util-visit";

const parser = unified().use(remarkParse);

export class ImagePathError extends Error {
  constructor(message = "无法安全转换图片路径，已取消保存。") {
    super(message);
    this.name = "ImagePathError";
  }
}

export function isSkippableUrl(url: string): boolean {
  const lower = url.toLowerCase();
  return (
    lower.startsWith("http://") ||
    lower.startsWith("https://") ||
    lower.startsWith("mailto:") ||
    lower.startsWith("blob:") ||
    lower.startsWith("data:")
  );
}

export function isBlobUrl(url: string): boolean {
  return url.toLowerCase().startsWith("blob:");
}

export function collectImageUrls(markdown: string): string[] {
  const urls: string[] = [];
  visit(parse(markdown), (node) => {
    if (node.type === "image" || node.type === "definition") {
      urls.push((node as Image | Definition).url);
      return;
    }
    if (node.type === "html") {
      for (const url of htmlSources((node as Html).value)) urls.push(url);
    }
  });
  return urls;
}

export function findBlobImageUrls(markdown: string): string[] {
  return collectImageUrls(markdown).filter(isBlobUrl);
}

export function localImageUrls(markdown: string): string[] {
  const seen = new Set<string>();
  const urls: string[] = [];
  for (const url of collectImageUrls(markdown)) {
    if (!url || isSkippableUrl(url) || seen.has(url)) continue;
    seen.add(url);
    urls.push(url);
  }
  return urls;
}

export function rewriteImageDestinations(
  markdown: string,
  mapUrl: (url: string) => string,
): string {
  const replacements: Replacement[] = [];
  visit(parse(markdown), (node) => {
    if (node.type === "image" || node.type === "definition") {
      pushNodeReplacement(markdown, node as Image | Definition, mapUrl, replacements);
      return;
    }
    if (node.type === "html") {
      pushHtmlReplacement(markdown, node as Html, mapUrl, replacements);
    }
  });
  replacements.sort((left, right) => right.start - left.start);
  let output = markdown;
  let cursor = markdown.length;
  for (const replacement of replacements) {
    if (replacement.end > cursor) throw new ImagePathError();
    output =
      output.slice(0, replacement.start) +
      replacement.text +
      output.slice(replacement.end);
    cursor = replacement.start;
  }
  return output;
}

interface Replacement {
  start: number;
  end: number;
  text: string;
}

function parse(markdown: string): Root {
  return parser.parse(markdown) as Root;
}

function pushNodeReplacement(
  markdown: string,
  node: Image | Definition,
  mapUrl: (url: string) => string,
  replacements: Replacement[],
): void {
  const next = mapUrl(node.url);
  if (next === node.url) return;
  const position = node.position;
  if (position?.start.offset == null || position.end.offset == null) {
    throw new ImagePathError();
  }
  const slice = markdown.slice(position.start.offset, position.end.offset);
  const local = locateDestination(slice, node.url);
  if (!local) throw new ImagePathError();
  const insideBrackets = slice[local.start - 1] === "<" && slice[local.end] === ">";
  replacements.push({
    start: position.start.offset + local.start,
    end: position.start.offset + local.end,
    text: encodeDestination(next, insideBrackets),
  });
}

function pushHtmlReplacement(
  markdown: string,
  node: Html,
  mapUrl: (url: string) => string,
  replacements: Replacement[],
): void {
  const position = node.position;
  if (position?.start.offset == null || position.end.offset == null) return;
  const slice = markdown.slice(position.start.offset, position.end.offset);
  const next = slice.replace(
    /\b(src\s*=\s*)(["'])(.*?)\2/gi,
    (full, prefix: string, quote: string, url: string) => {
      const decoded = decodeHtml(url);
      const mapped = mapUrl(decoded);
      if (mapped === decoded) return full;
      return `${prefix}${quote}${escapeHtml(mapped)}${quote}`;
    },
  );
  if (next === slice) return;
  replacements.push({
    start: position.start.offset,
    end: position.end.offset,
    text: next,
  });
}

function locateDestination(
  slice: string,
  url: string,
): { start: number; end: number } | null {
  const marker = slice.startsWith("![")
    ? findUnescaped(slice, "](")
    : findUnescaped(slice, "]:");
  if (marker < 0) return null;
  const regionStart = marker + 2;
  const span = urlSpan(slice.slice(regionStart));
  if (!span) return null;
  const raw = slice.slice(regionStart + span.start, regionStart + span.end);
  if (!urlsMatch(raw, url)) return null;
  return { start: regionStart + span.start, end: regionStart + span.end };
}

function urlSpan(text: string): { start: number; end: number } | null {
  let index = 0;
  while (index < text.length && /\s/.test(text[index])) index += 1;
  if (index >= text.length) return null;
  if (text[index] === "<") {
    const close = text.indexOf(">", index + 1);
    if (close < 0) return null;
    return { start: index + 1, end: close };
  }
  const start = index;
  while (index < text.length && !/\s/.test(text[index]) && text[index] !== ")") {
    index += 1;
  }
  if (index === start) return null;
  return { start, end: index };
}

function findUnescaped(slice: string, marker: string): number {
  for (let index = 0; index <= slice.length - marker.length; index += 1) {
    if (!slice.startsWith(marker, index)) continue;
    let slashes = 0;
    for (let cursor = index - 1; cursor >= 0 && slice[cursor] === "\\"; cursor -= 1) {
      slashes += 1;
    }
    if (slashes % 2 === 0) return index;
  }
  return -1;
}

function urlsMatch(raw: string, url: string): boolean {
  if (raw === url) return true;
  try {
    if (decodeURI(raw) === url) return true;
  } catch {
    // 非法百分号编码按原文比较。
  }
  return false;
}

function encodeDestination(url: string, insideBrackets: boolean): string {
  if (insideBrackets) return url.replaceAll(">", "%3E");
  return url
    .replaceAll("%", "%25")
    .replaceAll(" ", "%20")
    .replaceAll("(", "%28")
    .replaceAll(")", "%29")
    .replaceAll("<", "%3C")
    .replaceAll(">", "%3E");
}

function htmlSources(value: string): string[] {
  const urls: string[] = [];
  for (const match of value.matchAll(/\bsrc\s*=\s*(["'])(.*?)\1/gi)) {
    urls.push(decodeHtml(match[2] ?? ""));
  }
  return urls;
}

function decodeHtml(value: string): string {
  return value
    .replaceAll("&quot;", '"')
    .replaceAll("&#39;", "'")
    .replaceAll("&lt;", "<")
    .replaceAll("&gt;", ">")
    .replaceAll("&amp;", "&");
}

function escapeHtml(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll('"', "&quot;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
}
