# Prompt Dock · 快捷指令提示词库

**简体中文** · [English](README.en.md)

Prompt Dock 是一个使用 Tauri v2、Rust 和原生 HTML、CSS、TypeScript 的桌面提示词库。你可以编辑并保存常用提示词，也可以从系统托盘直接复制已保存的内容。

[下载最新版](https://github.com/shaoxiangm/prompt-dock/releases/latest) · [GitHub 仓库](https://github.com/shaoxiangm/prompt-dock)

## 使用

- 双击启动后选择左侧磁贴。右侧编辑名称、配色和内容。「立即复制」复制当前编辑框里的内容，「保存修改」将改动写入本地文件，并更新托盘菜单。
- 用「新建指令」添加磁贴。删除需要再确认一次。切换磁贴会保留本次运行中的草稿。
- Windows 和 macOS 上左键单击托盘图标可唤醒窗口，右键打开菜单。选择指令标题，会直接复制已保存的提示词。程序运行时再次启动会唤醒指令库。点击窗口关闭按钮后，程序隐藏窗口、在内存中保留草稿并释放 WebView，仍可通过托盘操作。从托盘选择「彻底退出」才会结束进程。
- 顶部和托盘都能切换「开机自启」。启用后，下次系统登录时会以 `--silent` 启动，第一次打开窗口前无需创建 WebView。主题保存在本机 WebView 的 `localStorage` 中。

## 数据与便携模式

默认将数据写到 Tauri 的应用配置目录，首次运行会填入四条中文指令。

| 平台 | 配置文件 |
| --- | --- |
| Windows | `%APPDATA%\com.promptdock.desktop\prompts.json` |
| macOS | `~/Library/Application Support/com.promptdock.desktop/prompts.json` |

把 `prompts.example.json` 复制到可执行文件所在目录，改名为 `prompts.json`，再彻底退出并重新启动，即可使用该文件。目录需要有写权限。macOS 的可执行文件位于 `Prompt Dock.app/Contents/MacOS/`，建议已签名的应用使用标准配置目录。保存时会先写入同目录临时文件，再替换原文件。备份或导入时请先退出工具。

配置格式为 JSON 数组。每条记录有 `id`、`title`、`content`、`color` 四个字段，`id` 需要唯一，颜色支持 `blue`、`green`、`amber`、`purple`、`rose`、`teal`。

## 本地开发与构建

从本目录运行以下命令。需要 Node.js 24 LTS 和 Rust stable。Windows 需要 Microsoft C++ Build Tools、Windows SDK 与 Edge WebView2 Runtime。macOS 需要 Xcode 命令行工具。[Tauri 环境准备](https://v2.tauri.app/start/prerequisites/)

```sh
npm ci
npm run build
cargo check --locked --manifest-path src-tauri/Cargo.toml
npm run dev
```

Windows x64 安装包与独立 EXE。

```powershell
rustup target add x86_64-pc-windows-msvc
npm run tauri -- build --target x86_64-pc-windows-msvc --bundles nsis -- --locked
```

- 独立 EXE 位于 `src-tauri/target/x86_64-pc-windows-msvc/release/prompt-dock.exe`，可直接启动，使用系统 WebView2 Runtime。
- NSIS 安装包位于 `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/`。安装程序在需要时联网获取 WebView2。
- 只构建独立 EXE 时，可用 `--no-bundle` 替换 `--bundles nsis`。

在 Apple Silicon Mac 构建 `.app` 和 `.dmg`。

```sh
rustup target add aarch64-apple-darwin
npm run tauri -- build --target aarch64-apple-darwin --bundles app,dmg -- --locked
```

产物位于 `src-tauri/target/aarch64-apple-darwin/release/bundle/{macos,dmg}/`。默认工作流未配置 Developer ID 签名、公证或 Windows 代码签名，下载的构建可能触发系统拦截。[Tauri macOS 签名](https://v2.tauri.app/distribute/sign/macos/)

## GitHub Actions

手动触发 Actions 里的「打包和发布」可并行构建两个平台，产物可在对应运行记录中下载。发布版本时同步更新 `package.json`、`package-lock.json`、`src-tauri/Cargo.toml`、`src-tauri/Cargo.lock` 和 `src-tauri/tauri.conf.json` 的版本号，然后推送对应标签，例如 `git tag v0.1.0` 和 `git push origin v0.1.0`。两个构建均成功后，工作流会将独立 EXE、NSIS 安装包、DMG 和装有 `.app` 的 ZIP 发布到 GitHub Releases。[查看打包工作流](https://github.com/shaoxiangm/prompt-dock/actions/workflows/release.yml)

需要重新打包已有版本时，可在 `main` 上手动运行工作流，并在 `release_tag` 填入已有的 `v*` 标签。运行会从该标签读取源码，并把产物上传到对应版本。

Windows x64 的 0.1.0 本地构建中，独立 EXE 为 3.06 MiB，NSIS 安装包为 1.15 MiB。本机实测主进程工作集在静默启动后约为 12.65 MiB，在关闭窗口并回收 WebView2 后约为 8.55 MiB。两次采样均无 WebView2 子进程。
