import classicDarkCss from "@milkdown/crepe/theme/classic-dark.css?inline";
import classicCss from "@milkdown/crepe/theme/classic.css?inline";
import frameDarkCss from "@milkdown/crepe/theme/frame-dark.css?inline";
import frameCss from "@milkdown/crepe/theme/frame.css?inline";
import nordDarkCss from "@milkdown/crepe/theme/nord-dark.css?inline";
import nordCss from "@milkdown/crepe/theme/nord.css?inline";

const STORAGE_KEY = "bundlepad.theme";
const STYLE_ID = "crepe-theme";

export interface EditorTheme {
  id: string;
  label: string;
  family: string;
  appearance: "light" | "dark";
  css: string;
}

export const editorThemes: readonly EditorTheme[] = [
  { id: "frame", label: "Frame 浅色", family: "frame", appearance: "light", css: frameCss },
  { id: "frame-dark", label: "Frame 深色", family: "frame", appearance: "dark", css: frameDarkCss },
  { id: "classic", label: "Classic 浅色", family: "classic", appearance: "light", css: classicCss },
  { id: "classic-dark", label: "Classic 深色", family: "classic", appearance: "dark", css: classicDarkCss },
  { id: "nord", label: "Nord 浅色", family: "nord", appearance: "light", css: nordCss },
  { id: "nord-dark", label: "Nord 深色", family: "nord", appearance: "dark", css: nordDarkCss },
];

const fallback = editorThemes[0];
let active = fallback;

export function normalizeThemeId(value: string | null | undefined): string {
  return editorThemes.find((theme) => theme.id === value)?.id ?? fallback.id;
}

export function scopeThemeCss(css: string): string {
  return css.replace(/\.milkdown\b/, ":root, .milkdown");
}

export function currentTheme(): EditorTheme {
  return active;
}

export function applyTheme(id: string): EditorTheme {
  const theme = editorThemes.find((item) => item.id === normalizeThemeId(id)) ?? fallback;
  active = theme;
  const style = ensureStyle();
  style.textContent = scopeThemeCss(theme.css);
  document.documentElement.dataset.theme = theme.id;
  document.documentElement.dataset.appearance = theme.appearance;
  try {
    localStorage.setItem(STORAGE_KEY, theme.id);
  } catch {
    // 隐私模式等环境下写不进去时，本次会话仍然使用所选主题。
  }
  return theme;
}

export function applyStoredTheme(): EditorTheme {
  let stored: string | null = null;
  try {
    stored = localStorage.getItem(STORAGE_KEY);
  } catch {
    stored = null;
  }
  return applyTheme(normalizeThemeId(stored));
}

function ensureStyle(): HTMLStyleElement {
  const existing = document.getElementById(STYLE_ID);
  if (existing instanceof HTMLStyleElement) return existing;
  const style = document.createElement("style");
  style.id = STYLE_ID;
  document.head.append(style);
  return style;
}
