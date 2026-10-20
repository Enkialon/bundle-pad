import type { Crepe } from "@milkdown/crepe";
import { decodeBase64, encodeBase64 } from "../host/codec.ts";
import { asAppError } from "../host/error.ts";
import type { Host, OpenedDocument } from "../host/types.ts";
import { bindShortcuts } from "./shortcuts.ts";
import { AssetUrlMap, missingImageUrl } from "../document/asset-url-map.ts";
import { DocumentSession } from "../document/document-session.ts";
import {
  findBlobImageUrls,
  isSkippableUrl,
  localImageUrls,
  rewriteImageDestinations,
} from "../document/image-paths.ts";
import {
  applyMarkdown,
  destroyEditor,
  focusEditor,
  formatCommands,
  insertImage,
  mountEditor,
  redo,
  runEditorCommand,
  undo,
} from "../editor/create-editor.ts";
import { mountMenu } from "../ui/menu.ts";

const IMAGE_EXTENSIONS = [".png", ".jpg", ".jpeg", ".gif", ".webp", ".bmp", ".avif"];

interface Runtime {
  generation: number;
  cache: Map<string, string>;
}

export async function startApp(host: Host): Promise<void> {
  const editorRoot = document.querySelector<HTMLElement>("#editor");
  const menuRoot = document.querySelector<HTMLElement>("#menubar");
  const sourceNode = document.querySelector<HTMLTextAreaElement>("#source");
  const sourceButton = document.querySelector<HTMLButtonElement>("#source-mode");
  const stateNode = document.querySelector<HTMLElement>("#status-state");
  if (!editorRoot || !menuRoot || !sourceNode || !sourceButton || !stateNode) {
    throw new Error("界面结构不完整");
  }

  const session = new DocumentSession();
  const assets = new AssetUrlMap();
  const runtime: Runtime = { generation: 0, cache: new Map() };
  let crepe: Crepe | null = null;
  let busy = false;
  let closing = false;
  let missingImages = false;
  let sourceMode = false;

  const syncSourceButton = () => {
    sourceButton.textContent = sourceMode ? "退出源码模式" : "源码模式";
    sourceButton.setAttribute("aria-pressed", sourceMode ? "true" : "false");
  };

  const showWysiwyg = () => {
    sourceMode = false;
    sourceNode.hidden = true;
    editorRoot.hidden = false;
    syncSourceButton();
  };

  const refreshChrome = async () => {
    stateNode.textContent = missingImages ? `${session.stateLabel()} · 部分图片缺失` : session.stateLabel();
    stateNode.classList.toggle("dirty", session.dirty);
    const title = session.title();
    document.title = title;
    host.window.noteUnsaved(session.dirty);
    await host.window.setTitle(title);
  };

  const currentMarkdown = () => (sourceMode ? sourceNode.value : crepe?.getMarkdown() ?? "");

  const rejectBlobUrls = (markdown: string) => {
    const blobs = findBlobImageUrls(markdown);
    if (blobs.length > 0) {
      throw new Error("Markdown 中出现了临时图片地址，已取消保存。");
    }
  };

  const documentPath = () => (session.path && session.kind !== "untitled" ? session.path : "");

  const readReference = async (reference: string) => {
    if (isSkippableUrl(reference)) throw new Error("无法读取图片");
    return host.documents.readAsset(documentPath(), reference);
  };

  const proxyDomURL = async (url: string): Promise<string> => {
    if (!url || isSkippableUrl(url)) return url;
    const generation = runtime.generation;
    const cached = runtime.cache.get(url);
    if (cached) return cached;
    try {
      const payload = await readReference(url);
      if (generation !== runtime.generation) return url;
      const blob = assets.create(decodeBase64(payload.dataBase64), payload.mimeType);
      runtime.cache.set(url, blob);
      return blob;
    } catch (error) {
      const appError = asAppError(error);
      if (appError.code === "asset_missing") missingImages = true;
      void refreshChrome();
      return missingImageUrl();
    }
  };

  const uploadFile = async (file: File): Promise<string> => {
    try {
      const dataBase64 = encodeBase64(new Uint8Array(await file.arrayBuffer()));
      const saved = await host.documents.writeAsset(documentPath(), dataBase64);
      return saved.relativePath;
    } catch (error) {
      const appError = asAppError(error);
      await host.dialogs.showError(appError.message);
      throw error;
    }
  };

  const loadEditor = async (markdown: string) => {
    showWysiwyg();
    runtime.generation += 1;
    runtime.cache.clear();
    assets.revokeAll();
    missingImages = false;
    await destroyEditor(crepe, editorRoot);
    const mounted = await mountEditor(editorRoot, markdown, {
      onMarkdown: (next) => {
        session.noteMarkdown(next);
        void refreshChrome();
      },
      onUpload: uploadFile,
      proxyDomURL,
    });
    crepe = mounted.crepe;
    session.baseline = mounted.baseline;
    session.dirty = false;
    focusEditor();
  };

  const applyOpened = async (opened: OpenedDocument) => {
    // 图片在编辑器挂载过程中就会读取。路径必须先就位，否则冷启动（双击、命令行）时会话还是未命名，读不到包内图片。
    session.kind = opened.kind;
    session.path = opened.path;
    session.displayName = opened.displayName;
    session.hasBom = opened.hasBom;
    session.stamp = opened.stamp;
    session.textFileName = opened.textFileName;
    await loadEditor(opened.content);
    session.dirty = false;
    await refreshChrome();
  };

  const reloadCurrent = async () => {
    if (!session.path || session.kind === "untitled") return;
    const opened = await host.documents.openDocument(session.path);
    await applyOpened(opened);
  };

  const saveCurrent = async (overwriteExternal = false): Promise<boolean> => {
    if (!session.path || session.kind === "untitled") return saveAs();
    if (!session.dirty && !overwriteExternal) return true;
    const markdown = currentMarkdown();
    try {
      rejectBlobUrls(markdown);
      const stamp = await host.documents.saveDocument(
        session.path,
        markdown,
        session.hasBom,
        session.textFileName,
        session.stamp?.sha256 ?? null,
        overwriteExternal,
      );
      session.markSaved(markdown, stamp);
      await refreshChrome();
      return true;
    } catch (error) {
      const appError = asAppError(error);
      if (appError.code === "external_change") {
        const choice = await host.dialogs.confirmExternalChange();
        if (choice === "overwrite") return saveCurrent(true);
        if (choice === "reload") {
          try {
            await reloadCurrent();
          } catch (reloadError) {
            await host.dialogs.showError(asAppError(reloadError).message);
          }
        }
        return false;
      }
      await host.dialogs.showError(appError.message);
      return false;
    }
  };

  const saveAs = async (): Promise<boolean> => {
    const target = await host.dialogs.pickSaveMarkdown(session.kind === "markdown" ? session.path ?? undefined : undefined);
    if (!target) return false;
    const markdown = currentMarkdown();
    try {
      rejectBlobUrls(markdown);
      const rewritten = await copyLocalImages(markdown, target);
      const stamp = await host.documents.saveDocument(target, rewritten, false, null, null, true);
      const opened = await host.documents.openDocument(target);
      opened.stamp = stamp;
      await applyOpened(opened);
      if (opened.content !== rewritten && crepe) {
        session.baseline = crepe.getMarkdown();
        session.dirty = false;
      }
      await refreshChrome();
      return true;
    } catch (error) {
      await host.dialogs.showError(asAppError(error).message);
      return false;
    }
  };

  const copyLocalImages = async (markdown: string, target: string) => {
    const mapping = new Map<string, string>();
    const missing: string[] = [];
    for (const url of localImageUrls(markdown)) {
      try {
        const payload = await readReference(url);
        if (!payload.mimeType.startsWith("image/")) continue;
        const saved = await host.documents.writeAsset(target, payload.dataBase64);
        mapping.set(url, saved.relativePath);
      } catch (error) {
        const appError = asAppError(error);
        if (appError.code === "asset_missing") missing.push(url);
        else throw error;
      }
    }
    if (missing.length > 0) {
      await host.dialogs.showWarning(`以下图片未能复制，原引用会保留：\n${missing.join("\n")}`);
    }
    return rewriteImageDestinations(markdown, (url) => mapping.get(url) ?? url);
  };

  const confirmLeave = async (): Promise<boolean> => {
    if (!session.dirty) return true;
    const choice = await host.dialogs.confirmUnsaved();
    if (choice === "cancel") return false;
    if (choice === "discard") return true;
    return saveCurrent();
  };

  const openClassified = async (path: string) => {
    if (!(await confirmLeave())) return;
    try {
      const opened = await host.documents.openDocument(path);
      await applyOpened(opened);
    } catch (error) {
      await host.dialogs.showError(asAppError(error).message);
    }
  };

  const newDocument = async () => {
    if (!(await confirmLeave())) return;
    await loadEditor("");
    session.resetUntitled();
    await refreshChrome();
  };

  const guard = async (action: () => Promise<void>) => {
    if (busy) return;
    busy = true;
    try {
      await action();
    } finally {
      busy = false;
    }
  };

  mountMenu(menuRoot, [
    {
      label: "文件",
      items: [
        { label: "新建 Markdown 文档", shortcut: "Ctrl+N", run: () => void guard(newDocument) },
        {
          label: "打开 Markdown 或 TextPack",
          shortcut: "Ctrl+O",
          run: () => void guard(async () => {
            const path = await host.dialogs.pickMarkdown(session.path ?? undefined);
            if (path) await openClassified(path);
          }),
        },
        {
          label: "打开 TextBundle 文档包",
          run: () => void guard(async () => {
            const path = await host.dialogs.pickTextBundle(session.path ?? undefined);
            if (path) await openClassified(path);
          }),
        },
        { label: "保存", shortcut: "Ctrl+S", separatorBefore: true, run: () => void guard(async () => { await saveCurrent(); }) },
        { label: "另存为 Markdown", shortcut: "Ctrl+Shift+S", run: () => void guard(async () => { await saveAs(); }) },
        { label: "退出", separatorBefore: true, run: () => void host.window.requestClose() },
      ],
    },
    {
      label: "编辑",
      items: [
        { label: "撤销", shortcut: "Ctrl+Z", run: () => { if (!sourceMode) undo(crepe); } },
        { label: "重做", shortcut: "Ctrl+Y", run: () => { if (!sourceMode) redo(crepe); } },
      ],
    },
    {
      label: "格式",
      items: [
        { label: "正文", run: () => { if (!sourceMode) runEditorCommand(crepe, formatCommands.text); } },
        { label: "标题 1", run: () => { if (!sourceMode) runEditorCommand(crepe, formatCommands.h1, 1); } },
        { label: "标题 2", run: () => { if (!sourceMode) runEditorCommand(crepe, formatCommands.h2, 2); } },
        { label: "标题 3", run: () => { if (!sourceMode) runEditorCommand(crepe, formatCommands.h3, 3); } },
        { label: "粗体", separatorBefore: true, run: () => { if (!sourceMode) runEditorCommand(crepe, formatCommands.strong); } },
        { label: "斜体", run: () => { if (!sourceMode) runEditorCommand(crepe, formatCommands.emphasis); } },
        { label: "行内代码", run: () => { if (!sourceMode) runEditorCommand(crepe, formatCommands.inlineCode); } },
        { label: "引用", separatorBefore: true, run: () => { if (!sourceMode) runEditorCommand(crepe, formatCommands.quote); } },
        { label: "代码块", run: () => { if (!sourceMode) runEditorCommand(crepe, formatCommands.codeBlock); } },
        { label: "无序列表", run: () => { if (!sourceMode) runEditorCommand(crepe, formatCommands.bullet); } },
        { label: "有序列表", run: () => { if (!sourceMode) runEditorCommand(crepe, formatCommands.ordered); } },
      ],
    },
  ]);

  bindShortcuts({
    newDocument: () => void guard(newDocument),
    openMarkdown: () => void guard(async () => {
      const path = await host.dialogs.pickMarkdown(session.path ?? undefined);
      if (path) await openClassified(path);
    }),
    save: () => void guard(async () => { await saveCurrent(); }),
    saveAs: () => void guard(async () => { await saveAs(); }),
  });

  await host.window.onCloseRequested(async () => {
    if (closing || busy) return false;
    busy = true;
    try {
      if (await confirmLeave()) {
        closing = true;
        assets.revokeAll();
        return true;
      }
      return false;
    } finally {
      if (!closing) busy = false;
    }
  });

  await host.window.onPathsDropped((paths) => {
    const [path, ...rest] = paths;
    if (!path) return;
    if (rest.length > 0) {
      void host.dialogs.showError("一次只能打开一个文档。");
      return;
    }
    const untitled = !documentPath();
    if (isImagePath(path) || (untitled && !isDocumentPath(path))) {
      void guard(() => insertDroppedImage(path));
      return;
    }
    void guard(() => openClassified(path));
  });

  const insertDroppedImage = async (path: string) => {
    if (!sourceMode && !crepe) {
      await host.dialogs.showError("编辑器还没有准备好。");
      return;
    }
    try {
      const saved = await host.documents.importLocalImage(documentPath(), path);
      if (sourceMode) {
        const snippet = `![](${saved.relativePath})`;
        sourceNode.setRangeText(snippet, sourceNode.selectionStart, sourceNode.selectionEnd, "end");
      } else if (crepe) {
        insertImage(crepe, saved.relativePath);
      }
      session.noteMarkdown(currentMarkdown());
      await refreshChrome();
    } catch (error) {
      await host.dialogs.showError(asAppError(error).message);
    }
  };

  sourceNode.addEventListener("input", () => {
    if (!sourceMode) return;
    session.noteMarkdown(sourceNode.value);
    void refreshChrome();
  });

  sourceButton.addEventListener("click", () => {
    if (!sourceMode) {
      sourceNode.value = crepe?.getMarkdown() ?? "";
      sourceMode = true;
      editorRoot.hidden = true;
      sourceNode.hidden = false;
      syncSourceButton();
      sourceNode.focus();
      return;
    }
    const markdown = sourceNode.value;
    showWysiwyg();
    if (!crepe) return;
    session.noteMarkdown(applyMarkdown(crepe, markdown));
    void refreshChrome();
    focusEditor();
  });

  await loadEditor("");
  session.resetUntitled();
  await refreshChrome();

  const paths = await host.documents.launchPaths();
  if (paths.length > 1) await host.dialogs.showWarning("一次只能打开一个文档，将打开第一个路径。");
  if (paths[0]) await openClassified(paths[0]);
}

function isImagePath(path: string): boolean {
  const lower = path.toLowerCase();
  return IMAGE_EXTENSIONS.some((extension) => lower.endsWith(extension));
}

function isDocumentPath(path: string): boolean {
  const lower = path.toLowerCase();
  return lower.endsWith(".md") || lower.endsWith(".markdown") || lower.endsWith(".textbundle") || lower.endsWith(".textpack");
}
