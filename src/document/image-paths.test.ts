import { describe, expect, it } from "vitest";
import {
  findBlobImageUrls,
  rewriteImageDestinations,
} from "./image-paths.ts";

const mapAsset = (url: string) =>
  url === "assets/a.png" ? "notes.assets/image-1.png" : url;

describe("rewriteImageDestinations", () => {
  it("只替换图片目标，保留标题、正文和代码块", () => {
    const markdown = [
      "见 assets/a.png",
      "",
      "![示例](assets/a.png \"标题\")",
      "",
      "```",
      "![示例](assets/a.png)",
      "```",
      "",
      "`![示例](assets/a.png)`",
    ].join("\n");

    const rewritten = rewriteImageDestinations(markdown, mapAsset);
    expect(rewritten).toContain("见 assets/a.png");
    expect(rewritten).toContain('![示例](notes.assets/image-1.png "标题")');
    expect(rewritten).toContain("```\n![示例](assets/a.png)\n```");
    expect(rewritten).toContain("`![示例](assets/a.png)`");
  });

  it("替换定义中的地址，并保留尖括号", () => {
    const markdown = ["[id]: assets/a.png", "", "![图][id]", "", "![空格](<assets/a.png>)"].join(
      "\n",
    );
    const rewritten = rewriteImageDestinations(markdown, mapAsset);
    expect(rewritten).toContain("[id]: notes.assets/image-1.png");
    expect(rewritten).toContain("![图][id]");
    expect(rewritten).toContain("![空格](<notes.assets/image-1.png>)");
  });

  it("发现编辑器临时地址，但忽略代码块中的示例", () => {
    const markdown = ["![图](blob:local/1)", "", "```", "blob:local/2", "```"].join("\n");
    expect(findBlobImageUrls(markdown)).toEqual(["blob:local/1"]);
  });
});
