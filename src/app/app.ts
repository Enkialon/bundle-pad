import { getCurrentWindow } from "@tauri-apps/api/window";
import type { Crepe } from "@milkdown/crepe";
import {
  asAppError,
  decodeBase64,
  encodeBase64,
  importLocalImage,
  launchPaths,
  openDocument,
  readAsset,
  saveDocument,
  writeAsset,
  type OpenedDocument,
} from "./api.ts";
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
  destroyEditor,
  focusEditor,
  formatCommands,
  insertImage,
  mountEditor,
  redo,
  runEditorCommand,
  undo,
} from "../editor/create-editor.ts";
import {
  confirmExternalChange,
  confirmUnsaved,
  pickMarkdown,
  pickSaveMarkdown,
  pickTextBundle,
  showError,
  showWarning,
} from "../ui/dialogs.ts";
import { mountMenu } from "../ui/menu.ts";

const IMAGE_EXTENSIONS = [".png", ".jpg", ".jpeg", ".gif", ".webp", ".bmp", ".avif"];

interface Runtime {
  generation: number;
  cache: Map<string, string>;
}

export async function startApp(): Promise<void> {
  const editorRoot = document.querySelector<HTMLElement>("#editor");
  const menuRoot = document.querySelector<HTMLElement>("#menubar");
  const kindNode = document.querySelector<HTMLElement>("#status-kind");
  const stateNode = document.querySelector<HTMLElement>("#status-state");
  if (!editorRoot || !menuRoot || !kindNode || !stateNode) {
    throw new Error("界面结构不完整");
  }

  const session = new DocumentSession();
  const assets = new AssetUrlMap();
  const runtime: Runtime = { generation: 0, cache: new Map() };
  let crepe: Crepe | null = null;
  let busy = false;
  let closing = false;
  let missingImages = false;

  const refreshChrome = async () => {
    kindNode.textContent = missingImages ? `${session.kindLabel()} · 部分图片缺失` : session.kindLabel();
    stateNode.textContent = session.stateLabel();
    stateNode.classList.toggle("dirty", session.dirty);
    const title = session.title();
    document.title = title;
    await getCurrentWindow().setTitle(title);
  };

  const currentMarkdown = () => crepe?.getMarkdown() ?? "";

  const rejectBlobUrls = (markdown: string) => {
    const blobs = findBlobImageUrls(markdown);
    if (blobs.length > 0) {
      throw new Error("Markdown 中出现了临时图片地址，已取消保存。");
    }
  };

  const readReference = async (reference: string) => {
    if (!session.path || session.kind === "untitled" || isSkippableUrl(reference)) {
      throw new Error("无法读取图片");
    }
    return readAsset(session.path, reference);
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
    if (!session.path || session.kind === "untitled") {
      const messageText = "请先保存文档，再插入图片。";
      await showError(messageText);
      throw new Error(messageText);
    }
    try {
      const dataBase64 = encodeBase64(new Uint8Array(await file.arrayBuffer()));
      const saved = await writeAsset(session.path, dataBase64);
      return saved.relativePath;
    } catch (error) {
      const appError = asAppError(error);
      await showError(appError.message);
      throw error;
    }
  };

  const loadEditor = async (markdown: string) => {
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
    await loadEditor(opened.content);
    session.kind = opened.kind;
    session.path = opened.path;
    session.displayName = opened.displayName;
    session.hasBom = opened.hasBom;
    session.stamp = opened.stamp;
    session.textFileName = opened.textFileName;
    session.dirty = false;
    await refreshChrome();
  };

  const reloadCurrent = async () => {
    if (!session.path || session.kind === "untitled") return;
    const opened = await openDocument(session.path);
    await applyOpened(opened);
  };

  const saveCurrent = async (overwriteExternal = false): Promise<boolean> => {
    if (!session.path || session.kind === "untitled") return saveAs();
    if (!session.dirty && !overwriteExternal) return true;
    const markdown = currentMarkdown();
    try {
      rejectBlobUrls(markdown);
      const stamp = await saveDocument(
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
        const choice = await confirmExternalChange();
        if (choice === "overwrite") return saveCurrent(true);
        if (choice === "reload") {
          try {
            await reloadCurrent();
          } catch (reloadError) {
            await showError(asAppError(reloadError).message);
          }
        }
        return false;
      }
      await showError(appError.message);
      return false;
    }
  };

  const saveAs = async (): Promise<boolean> => {
    const target = await pickSaveMarkdown(session.kind === "markdown" ? session.path ?? undefined : undefined);
    if (!target) return false;
    const markdown = currentMarkdown();
    try {
      rejectBlobUrls(markdown);
      const rewritten = await copyLocalImages(markdown, target);
      const stamp = await saveDocument(target, rewritten, false, null, null, true);
      const opened = await openDocument(target);
      opened.stamp = stamp;
      await applyOpened(opened);
      if (opened.content !== rewritten && crepe) {
        session.baseline = crepe.getMarkdown();
        session.dirty = false;
      }
      await refreshChrome();
      return true;
    } catch (error) {
      await showError(asAppError(error).message);
      return false;
    }
  };

  const copyLocalImages = async (markdown: string, target: string) => {
    const mapping = new Map<string, string>();
    const missing: string[] = [];
    for (const url of localImageUrls(markdown)) {
      if (!session.path || session.kind === "untitled") {
        missing.push(url);
        continue;
      }
      try {
        const payload = await readReference(url);
        const saved = await writeAsset(target, payload.dataBase64);
        mapping.set(url, saved.relativePath);
      } catch (error) {
        const appError = asAppError(error);
        if (appError.code === "asset_missing") missing.push(url);
        else throw error;
      }
    }
    if (missing.length > 0) {
      await showWarning(`以下图片未能复制，原引用会保留：\n${missing.join("\n")}`);
    }
    return rewriteImageDestinations(markdown, (url) => mapping.get(url) ?? url);
  };

  const confirmLeave = async (): Promise<boolean> => {
    if (!session.dirty) return true;
    const choice = await confirmUnsaved();
    if (choice === "cancel") return false;
    if (choice === "discard") return true;
    return saveCurrent();
  };

  const openClassified = async (path: string) => {
    if (!(await confirmLeave())) return;
    try {
      const opened = await openDocument(path);
      await applyOpened(opened);
    } catch (error) {
      await showError(asAppError(error).message);
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
          label: "打开 Markdown 文档",
          shortcut: "Ctrl+O",
          run: () => void guard(async () => {
            const path = await pickMarkdown(session.path ?? undefined);
            if (path) await openClassified(path);
          }),
        },
        {
          label: "打开 TextBundle 文档包",
          run: () => void guard(async () => {
            const path = await pickTextBundle(session.path ?? undefined);
            if (path) await openClassified(path);
          }),
        },
        { label: "保存", shortcut: "Ctrl+S", run: () => void guard(async () => { await saveCurrent(); }) },
        { label: "另存为 Markdown", shortcut: "Ctrl+Shift+S", run: () => void guard(async () => { await saveAs(); }) },
        { label: "退出", run: () => void getCurrentWindow().close() },
      ],
    },
    {
      label: "编辑",
      items: [
        { label: "撤销", shortcut: "Ctrl+Z", run: () => undo(crepe) },
        { label: "重做", shortcut: "Ctrl+Y", run: () => redo(crepe) },
      ],
    },
    {
      label: "格式",
      items: [
        { label: "正文", run: () => runEditorCommand(crepe, formatCommands.text) },
        { label: "标题 1", run: () => runEditorCommand(crepe, formatCommands.h1, 1) },
        { label: "标题 2", run: () => runEditorCommand(crepe, formatCommands.h2, 2) },
        { label: "标题 3", run: () => runEditorCommand(crepe, formatCommands.h3, 3) },
        { label: "粗体", run: () => runEditorCommand(crepe, formatCommands.strong) },
        { label: "斜体", run: () => runEditorCommand(crepe, formatCommands.emphasis) },
        { label: "行内代码", run: () => runEditorCommand(crepe, formatCommands.inlineCode) },
        { label: "引用", run: () => runEditorCommand(crepe, formatCommands.quote) },
        { label: "代码块", run: () => runEditorCommand(crepe, formatCommands.codeBlock) },
        { label: "无序列表", run: () => runEditorCommand(crepe, formatCommands.bullet) },
        { label: "有序列表", run: () => runEditorCommand(crepe, formatCommands.ordered) },
      ],
    },
  ]);

  bindShortcuts({
    newDocument: () => void guard(newDocument),
    openMarkdown: () => void guard(async () => {
      const path = await pickMarkdown(session.path ?? undefined);
      if (path) await openClassified(path);
    }),
    save: () => void guard(async () => { await saveCurrent(); }),
    saveAs: () => void guard(async () => { await saveAs(); }),
  });

  await getCurrentWindow().onCloseRequested(async (event) => {
    if (closing) return;
    event.preventDefault();
    if (busy) return;
    busy = true;
    try {
      if (await confirmLeave()) {
        closing = true;
        assets.revokeAll();
        await getCurrentWindow().destroy();
      }
    } finally {
      busy = false;
    }
  });

  await getCurrentWindow().onDragDropEvent((event) => {
    if (event.payload.type !== "drop") return;
    const [path, ...rest] = event.payload.paths;
    if (!path) return;
    if (rest.length > 0) {
      void showError("一次只能打开一个文档。");
      return;
    }
    if (isImagePath(path)) {
      void guard(() => insertDroppedImage(path));
      return;
    }
    void guard(() => openClassified(path));
  });

  const insertDroppedImage = async (path: string) => {
    if (!crepe || !session.path || session.kind === "untitled") {
      await showError("请先保存文档，再插入图片。");
      return;
    }
    try {
      const saved = await importLocalImage(session.path, path);
      insertImage(crepe, saved.relativePath);
      session.noteMarkdown(currentMarkdown());
      await refreshChrome();
    } catch (error) {
      await showError(asAppError(error).message);
    }
  };

  await loadEditor("");
  session.resetUntitled();
  await refreshChrome();

  const paths = await launchPaths();
  if (paths.length > 1) await showWarning("一次只能打开一个文档，将打开第一个路径。");
  if (paths[0]) await openClassified(paths[0]);
}

function isImagePath(path: string): boolean {
  const lower = path.toLowerCase();
  return IMAGE_EXTENSIONS.some((extension) => lower.endsWith(extension));
}
