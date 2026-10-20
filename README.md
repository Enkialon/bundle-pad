# BundlePad

BundlePad 是一个 Windows 上的轻量 Markdown 编辑器。一次只打开一篇文档，用所见即所得的方式编辑，并写回原来的文件。

它能打开三种文档：

- 普通 Markdown：`.md`、`.markdown`
- [TextBundle](https://textbundle.org/) 目录包：`.textbundle`
- TextPack 压缩包：`.textpack`（一个 zip 压缩的 TextBundle）

编辑和保存都在原文件上完成。TextBundle 只改包里的正文，不重写 `info.json`。TextPack 保存时把正文和图片写回原来的压缩包。

## 能做什么

- 新建、打开、保存 Markdown，或把当前文档另存为 Markdown。
- 打开已有的 TextBundle 目录，编辑正文，并把新图片放进包内的 `assets/`。
- 打开已有的 TextPack，编辑正文和图片，保存后仍是同一个 `.textpack`。
- 拖进窗口，或用命令行把文档路径传给程序。
- 发现文件在编辑期间被其他程序改过时，可以选择重新加载、覆盖保存或取消。

不能新建 TextBundle 或 TextPack，只能打开已经存在的文档包。安装程序也不会把这些扩展名注册到资源管理器，双击文件不会自动用 BundlePad 打开。

## 环境

- Windows 10/11 x64
- Node.js 22 或更新版本
- Rust stable
- Microsoft Edge WebView2 Runtime（Windows 11 通常已经自带）

## 开发

```powershell
npm install
npm run tauri dev
```

## 测试

```powershell
npm test
npm run test:rust
```

## 安装包

```powershell
npm run tauri build
```

NSIS 安装包生成在 `src-tauri\target\release\bundle\nsis\`。安装不需要账户或网络服务，程序只读写用户选择的本地文档。

## 使用

文件菜单提供新建、打开 Markdown 或 TextPack、打开 TextBundle、保存、另存为 Markdown 和退出。

| 操作 | 快捷键 |
| --- | --- |
| 新建 Markdown | `Ctrl+N` |
| 打开 | `Ctrl+O` |
| 保存 | `Ctrl+S` |
| 另存为 Markdown | `Ctrl+Shift+S` |
| 撤销 / 重做 | `Ctrl+Z` / `Ctrl+Y` |

也可以把 `.md`、`.markdown`、`.textpack` 或 `.textbundle` 目录拖进窗口，或从命令行打开：

```powershell
BundlePad.exe "D:\docs\note.md"
BundlePad.exe "D:\docs\example.textbundle"
BundlePad.exe "D:\docs\example.textpack"
```

## 图片

- TextBundle 的新图片写入该包的 `assets/`，正文使用 `assets/文件名.ext`。
- TextPack 的新图片写入压缩包内的 `assets/`，引用同样是相对路径。
- 普通 Markdown 的新图片写入同级的 `<文档名>.assets/`。
- 已有的相对路径、绝对路径和远程地址不会被无条件改写。远程图片不会下载进文档。
- 图片缺失时显示「图片缺失」，正文里的原始引用保持不变。
- 新插入的图片接受 PNG、JPEG、GIF、WebP、BMP 和 AVIF。已有的 SVG 可以显示，但拒绝包含脚本的 SVG。

## 已知限制

- 不支持新建 TextBundle 或 TextPack。
- 不注册资源管理器文件关联。
- 不提供知识库、多标签、插件、同步或 AI 写作。
- 用户修改正文后，编辑器可能规范化 Markdown 语法；未修改的文件不会因此被重写。
- 不再被引用的旧图片会保留，不做自动清理。
