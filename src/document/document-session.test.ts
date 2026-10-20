import { describe, expect, it } from "vitest";
import { DocumentSession } from "./document-session.ts";

describe("DocumentSession", () => {
  it("未变化时不标记未保存，改回原文后清除标记", () => {
    const session = new DocumentSession();
    session.baseline = "hello";
    session.noteMarkdown("hello");
    expect(session.dirty).toBe(false);
    session.noteMarkdown("hello!");
    expect(session.dirty).toBe(true);
    expect(session.stateLabel()).toBe("未保存");
    session.noteMarkdown("hello");
    expect(session.dirty).toBe(false);
    expect(session.title()).toBe("BundlePad — 未命名");
  });
});
