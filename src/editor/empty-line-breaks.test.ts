import { describe, expect, it } from "vitest";
import { stripEmptyLineBreaks } from "./empty-line-breaks.ts";

describe("stripEmptyLineBreaks", () => {
  it("removes standalone empty-paragraph placeholders", () => {
    const input = [
      "![1.00](assets/a.png)",
      "",
      "<br />",
      "",
      "![1.00](assets/b.png)",
      "",
      "<br />",
      "",
      "的",
      "",
      "<br />",
      "",
      "<br />",
      "",
      "d",
      "",
    ].join("\n");

    expect(stripEmptyLineBreaks(input)).toBe(
      ["![1.00](assets/a.png)", "", "![1.00](assets/b.png)", "", "的", "", "d", ""].join("\n"),
    );
  });

  it("keeps inline br tags inside a line", () => {
    expect(stripEmptyLineBreaks("foo<br>bar\n")).toBe("foo<br>bar\n");
    expect(stripEmptyLineBreaks("line one <br /> line two\n")).toBe("line one <br /> line two\n");
  });

  it("accepts common br spellings on their own line", () => {
    expect(stripEmptyLineBreaks("a\n<br>\nb\n")).toBe("a\n\nb\n");
    expect(stripEmptyLineBreaks("a\n<br/>\nb\n")).toBe("a\n\nb\n");
    expect(stripEmptyLineBreaks("a\n<br />\nb\n")).toBe("a\n\nb\n");
  });
});
