import { Crepe } from "@milkdown/crepe";
import { editorViewCtx, type CmdKey } from "@milkdown/kit/core";
import { redoCommand, undoCommand } from "@milkdown/kit/plugin/history";
import {
  createCodeBlockCommand,
  toggleEmphasisCommand,
  toggleInlineCodeCommand,
  toggleStrongCommand,
  turnIntoTextCommand,
  wrapInBlockquoteCommand,
  wrapInBulletListCommand,
  wrapInHeadingCommand,
  wrapInOrderedListCommand,
} from "@milkdown/kit/preset/commonmark";
import { callCommand } from "@milkdown/kit/utils";

export interface EditorCallbacks {
  onMarkdown: (markdown: string) => void;
  onUpload: (file: File) => Promise<string>;
  proxyDomURL: (url: string) => Promise<string> | string;
}

export async function mountEditor(
  root: HTMLElement,
  markdown: string,
  callbacks: EditorCallbacks,
): Promise<{ crepe: Crepe; baseline: string }> {
  root.replaceChildren();
  let hydrating = true;
  const crepe = new Crepe({
    root,
    defaultValue: markdown,
    features: {
      [Crepe.Feature.Cursor]: true,
      [Crepe.Feature.ListItem]: true,
      [Crepe.Feature.LinkTooltip]: true,
      [Crepe.Feature.ImageBlock]: true,
      [Crepe.Feature.BlockEdit]: true,
      [Crepe.Feature.Placeholder]: true,
      [Crepe.Feature.Toolbar]: true,
      [Crepe.Feature.CodeMirror]: true,
      [Crepe.Feature.Table]: true,
      [Crepe.Feature.Latex]: true,
      [Crepe.Feature.TopBar]: false,
      [Crepe.Feature.AI]: false,
    },
    featureConfigs: {
      [Crepe.Feature.Placeholder]: {
        text: "开始书写…",
        mode: "doc",
      },
      [Crepe.Feature.LinkTooltip]: {
        inputPlaceholder: "输入链接",
        editButton: "编辑",
        removeButton: "移除",
        confirmButton: "确定",
      },
      [Crepe.Feature.Toolbar]: {
        boldLabel: "粗体",
        italicLabel: "斜体",
        codeLabel: "行内代码",
        linkLabel: "链接",
        strikethroughLabel: "删除线",
        latexLabel: "公式",
      },
      [Crepe.Feature.ImageBlock]: {
        onUpload: callbacks.onUpload,
        inlineOnUpload: callbacks.onUpload,
        blockOnUpload: callbacks.onUpload,
        proxyDomURL: callbacks.proxyDomURL,
        inlineUploadButton: "上传",
        inlineConfirmButton: "确定",
        inlineUploadPlaceholderText: "或粘贴链接",
        blockUploadButton: "上传图片",
        blockConfirmButton: "确定",
        blockCaptionPlaceholderText: "图片说明",
        blockUploadPlaceholderText: "或粘贴链接",
      },
      [Crepe.Feature.BlockEdit]: {
        textGroup: {
          label: "文本",
          text: { label: "正文" },
          h1: { label: "标题 1" },
          h2: { label: "标题 2" },
          h3: { label: "标题 3" },
          h4: { label: "标题 4" },
          h5: { label: "标题 5" },
          h6: { label: "标题 6" },
          quote: { label: "引用" },
          divider: { label: "分隔线" },
        },
        listGroup: {
          label: "列表",
          bulletList: { label: "无序列表" },
          orderedList: { label: "有序列表" },
          taskList: { label: "任务列表" },
        },
        advancedGroup: {
          label: "高级",
          image: { label: "图片" },
          codeBlock: { label: "代码块" },
          table: { label: "表格" },
          math: { label: "公式" },
        },
      },
    },
  });

  crepe.on((listener) => {
    listener.markdownUpdated((_ctx, next) => {
      if (hydrating) return;
      callbacks.onMarkdown(next);
    });
  });

  await crepe.create();
  await new Promise((resolve) => setTimeout(resolve, 0));
  hydrating = false;
  return { crepe, baseline: crepe.getMarkdown() };
}

export async function destroyEditor(crepe: Crepe | null, root: HTMLElement): Promise<void> {
  if (crepe) await crepe.destroy();
  root.replaceChildren();
}

export function insertImage(crepe: Crepe, src: string): void {
  crepe.editor.action((ctx) => {
    const view = ctx.get(editorViewCtx);
    const type = view.state.schema.nodes["image-block"] ?? view.state.schema.nodes.image;
    if (!type) return false;
    const node = type.createAndFill({ src, caption: "", alt: "" });
    if (!node) return false;
    view.dispatch(view.state.tr.replaceSelectionWith(node).scrollIntoView());
    return true;
  });
}

export function runEditorCommand<T>(
  crepe: Crepe | null,
  command: { key: CmdKey<T> },
  payload?: T,
): void {
  if (!crepe) return;
  focusEditor();
  crepe.editor.action(callCommand(command.key, payload));
}

export function undo(crepe: Crepe | null): void {
  runEditorCommand(crepe, undoCommand);
}

export function redo(crepe: Crepe | null): void {
  runEditorCommand(crepe, redoCommand);
}

export function focusEditor(): void {
  const editor = document.querySelector("#editor .ProseMirror") as HTMLElement | null;
  editor?.focus();
}

export const formatCommands = {
  text: turnIntoTextCommand,
  h1: wrapInHeadingCommand,
  h2: wrapInHeadingCommand,
  h3: wrapInHeadingCommand,
  strong: toggleStrongCommand,
  emphasis: toggleEmphasisCommand,
  inlineCode: toggleInlineCodeCommand,
  quote: wrapInBlockquoteCommand,
  codeBlock: createCodeBlockCommand,
  bullet: wrapInBulletListCommand,
  ordered: wrapInOrderedListCommand,
};
