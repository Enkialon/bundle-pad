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
