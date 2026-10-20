import { describe, expect, it } from "vitest";
import { editorThemes, normalizeThemeId, scopeThemeCss } from "./theme.ts";

describe("normalizeThemeId", () => {
  it("未知值回退到 Frame 浅色", () => {
    expect(normalizeThemeId(null)).toBe("frame");
    expect(normalizeThemeId("")).toBe("frame");
    expect(normalizeThemeId("nord-light")).toBe("frame");
  });

  it("接受内置主题", () => {
    for (const theme of editorThemes) {
      expect(normalizeThemeId(theme.id)).toBe(theme.id);
    }
  });
});

describe("scopeThemeCss", () => {
  it("把变量挂到页面根节点，编辑器仍然使用同一套变量", () => {
    const css = ".milkdown {\n  --crepe-color-background: #fff;\n}";
    expect(scopeThemeCss(css)).toBe(":root, .milkdown {\n  --crepe-color-background: #fff;\n}");
  });
});
