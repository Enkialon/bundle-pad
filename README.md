# BundlePad

BundlePad 是一个 Windows 轻量级、单文档、所见即所得的 Markdown 编辑器。它可以打开普通 Markdown，也可以直接打开、编辑并原位保存 `.textbundle` 目录包。

## 环境

- Windows 10/11 x64
- Node.js 22 或更新版本
- Rust stable（当前验证使用 1.97）
- Microsoft Edge WebView2 Runtime（Windows 11 通常已经自带）

## 开发

```powershell
npm install
npm run tauri dev
```

前端单独调试：

```powershell
npm run dev
```

## 测试

```powershell
npm test
npm run test:rust
npm run build
```

Rust 测试覆盖 TextBundle 读取与原位保存、未知元数据保留、路径穿越拒绝、外部修改冲突，以及 Windows 上已存在文件的替换。前端测试覆盖图片地址只在 Markdown 图片节点中转换，以及未修改文档不进入未保存状态。

## 前端源码（src）

`src` 是 Tauri 窗口里的前端。它负责菜单、快捷键、Milkdown 所见即所得编辑，以及把打开、保存、图片读写交给 Rust 后端。磁盘上的文件读写不在这里完成。

```text
src/
  main.ts                          入口：加载 Milkdown 主题并启动应用
  vite-env.d.ts                    让 TypeScript 识别 CSS 模块导入
  styles.css                       窗口、菜单栏、状态栏和编辑区的样式
  assets/                          Vite 默认图标（vite、typescript、tauri）
  app/                             应用编排、后端调用、快捷键
  editor/                          Milkdown 编辑器的创建和命令
  ui/                              菜单和系统对话框
  document/                        文档状态、图片路径、内存中的图片地址
```

### `app/`

应用怎么跑，集中在这里。

- `app.ts`：启动后的主流程。挂上菜单和快捷键，新建、打开、保存、另存为 Markdown，打开已有 TextBundle，处理拖放和命令行传入的路径。关闭窗口前会询问未保存的修改。另存为时把本地图片复制到目标文档旁边。图片在编辑器里用内存 Blob 地址显示，保存前如果正文里还留着 `blob:` 地址就取消保存。
- `api.ts`：前端调用 Rust 命令的入口。打开、保存、读图、写图和导入本地图片都按文档路径交给后端，由后端区分 Markdown 和 TextBundle。同时负责错误对象解析，以及图片字节和 Base64 的互相转换。
- `shortcuts.ts`：绑定 `Ctrl+N` 新建、`Ctrl+O` 打开、`Ctrl+S` 保存、`Ctrl+Shift+S` 另存为。撤销和重做不在这里，由编辑器自己处理。

### `editor/`

- `create-editor.ts`：创建和销毁 Milkdown Crepe 编辑器。打开光标、列表、链接、图片、块编辑、占位符、工具栏、代码块、表格和公式；关闭顶部栏和 AI。对外提供插入图片、撤销、重做，以及正文、标题、粗体、斜体、行内代码、引用、代码块、列表等格式命令。加载文档时先记下编辑器规范化后的正文，避免未改动的文件被标成未保存。

### `ui/`

- `menu.ts`：渲染「文件 / 编辑 / 格式」菜单。点击展开，点空白处或按 Esc 关闭。菜单项可以带快捷键提示。
- `dialogs.ts`：系统对话框。错误和警告提示，未保存确认（保存 / 不保存 / 取消），外部修改冲突（重新加载 / 覆盖保存 / 取消），以及打开 Markdown、打开 TextBundle 目录、另存为 Markdown 的文件选择。

### `document/`

文档在内存里的状态，以及图片地址怎么处理。

- `document-session.ts`：当前文档的种类（未命名、Markdown、TextBundle）、路径、显示名、BOM、TextBundle 正文文件名、文件指纹和是否已修改。窗口标题和状态栏文案也从这里生成。
- `image-paths.ts`：用 Markdown 语法树找出图片地址。只改图片和引用定义里的目标，正文、代码块里长得像图片的文字保持不动。远程地址、`mailto:`、`blob:`、`data:` 会跳过。另存为时用它把旧的本地图片路径改写成新文档下的相对路径。
- `asset-url-map.ts`：把后端读到的图片字节做成 Blob URL，供编辑器显示；换文档时统一释放。图片文件缺失时返回一张写着「图片缺失」的占位图。
- `document-session.test.ts`：确认正文没变时不会进入未保存，改回去后标记会清除。
- `image-paths.test.ts`：确认地址替换只发生在图片节点，代码块和行内代码里的相同文字不会被改。

## 构建 Windows 安装包

```powershell
npm run tauri build
```

成功后，NSIS 安装包位于：

```text
src-tauri\target\release\bundle\nsis\
```

安装包不需要账户或网络服务。应用只在本地读写用户选择的文档。

## 使用

- 文件菜单可以新建 Markdown、打开 Markdown、打开 TextBundle 目录、保存、另存为 Markdown 和退出。
- 快捷键：`Ctrl+N`、`Ctrl+O`、`Ctrl+S`、`Ctrl+Shift+S`。撤销和重做使用编辑器自带的 `Ctrl+Z`、`Ctrl+Y`。
- 可以把 `.md`、`.markdown` 或 `.textbundle` 目录拖进窗口。
- 可以用命令行打开一个文档：

```powershell
BundlePad.exe "D:\docs\note.md"
BundlePad.exe "D:\docs\example.textbundle"
```

## 图片

- TextBundle 里的新图片写入当前包的 `assets/`，正文使用 `assets/文件名.ext`。
- 普通 Markdown 的新图片写入同级的 `<文档名>.assets/`。
- 已有的相对路径、绝对路径和远程地址不会被无条件改写。远程图片不会下载进文档。
- 编辑器里的显示地址只存在于内存中的 Blob URL。保存前如果发现 `blob:` 地址，会取消保存。
- 图片缺失时显示“图片缺失”，正文里的原始引用保持不变。
- 不再被引用的旧图片会保留，第一版不做自动清理。

## 保存

- 没有修改时，保存不会重写文件。
- 正文使用同目录临时文件写入，成功后再替换原文件。
- 保存失败时保留原文件和编辑器中的内容。
- 如果发现文件在编辑期间被其他程序修改，可以选择重新加载、覆盖保存或取消。
- 只修改正文时，不会重写 `info.json`。

## 已知限制

- 不支持新建 TextBundle，只能打开已经存在的目录包。
- 不支持 `.textpack` 压缩包。
- 不注册资源管理器文件关联。
- 不提供知识库、多标签、插件、同步或 AI 写作。
- 用户实际修改正文后，编辑器可能规范化 Markdown 语法；未修改的文件不会因此被重写。
- 新插入的图片只接受 PNG、JPEG、GIF、WebP、BMP 和 AVIF。已有的 SVG 可以显示，但拒绝包含脚本的 SVG。
- 第一版没有在干净的 Windows 虚拟机里安装验证安装包。
