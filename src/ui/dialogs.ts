import { message, open, save } from "@tauri-apps/plugin-dialog";

export async function showError(text: string): Promise<void> {
  await message(text, { title: "BundlePad", kind: "error", buttons: { ok: "确定" } });
}

export async function showWarning(text: string): Promise<void> {
  await message(text, { title: "BundlePad", kind: "warning", buttons: { ok: "确定" } });
}

export async function confirmUnsaved(): Promise<"save" | "discard" | "cancel"> {
  const result = await message("当前文档有未保存的修改。", {
    title: "BundlePad",
    kind: "warning",
    buttons: { yes: "保存", no: "不保存", cancel: "取消" },
  });
  if (result === "Yes" || result === "保存") return "save";
  if (result === "No" || result === "不保存") return "discard";
  return "cancel";
}

export async function confirmExternalChange(): Promise<"reload" | "overwrite" | "cancel"> {
  const result = await message(
    "这个文档已被其他程序修改。重新加载会丢弃当前未保存的修改；覆盖保存会用当前内容替换磁盘上的文件。",
    {
      title: "BundlePad",
      kind: "warning",
      buttons: { yes: "重新加载", no: "覆盖保存", cancel: "取消" },
    },
  );
  if (result === "Yes" || result === "重新加载") return "reload";
  if (result === "No" || result === "覆盖保存") return "overwrite";
  return "cancel";
}

export async function pickMarkdown(defaultPath?: string): Promise<string | null> {
  const selected = await open({
    title: "打开 Markdown 文档",
    multiple: false,
    directory: false,
    defaultPath,
    filters: [{ name: "Markdown", extensions: ["md", "markdown"] }],
  });
  return typeof selected === "string" ? selected : null;
}

export async function pickTextBundle(defaultPath?: string): Promise<string | null> {
  const selected = await open({
    title: "打开 TextBundle 文档包",
    directory: true,
    multiple: false,
    defaultPath,
  });
  return typeof selected === "string" ? selected : null;
}

export async function pickSaveMarkdown(defaultPath?: string): Promise<string | null> {
  const selected = await save({
    title: "另存为 Markdown",
    defaultPath,
    filters: [{ name: "Markdown", extensions: ["md", "markdown"] }],
  });
  if (!selected) return null;
  if (/\.(md|markdown)$/i.test(selected)) return selected;
  return `${selected}.md`;
}
