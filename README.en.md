# Prompt Dock

**English** · [简体中文](README.md)

Prompt Dock is a desktop prompt library built with Tauri v2, Rust, and plain HTML, CSS, and TypeScript. Save and edit frequently used prompts, or copy saved prompts directly from the system tray.

[Download the latest release](https://github.com/shaoxiangm/prompt-dock/releases/latest) · [GitHub repository](https://github.com/shaoxiangm/prompt-dock)

## Usage

The app interface and its four starter prompts are in Simplified Chinese.

- Double-click the app to launch it and select a tile on the left. Edit its name, color, and text on the right. **立即复制** (Copy now) copies the current editor text. **保存修改** (Save changes) writes your changes to disk and updates the tray menu.
- Select **新建指令** (New prompt) to add a tile. Deleting a prompt requires confirmation. Switching tiles preserves drafts for the current session.
- On Windows and macOS, left-click the tray icon to open the library and right-click to show the tray menu. Select a prompt title to copy its saved text. Launching the app again brings the library to the front. Closing the window hides it, keeps drafts in memory, and releases the WebView. Choose **彻底退出** (Quit) from the tray menu to exit the app.
- **开机自启** (Start at login) is available in both the toolbar and the tray menu. When enabled, the app starts with `--silent` at your next login and creates the WebView when you first open the window. The app stores your theme preference in the local WebView's `localStorage`.

## Data and portable mode

By default, Prompt Dock writes data to the Tauri application configuration directory. It adds four starter prompts on first launch.

| Platform | Configuration file |
| --- | --- |
| Windows | `%APPDATA%\com.promptdock.desktop\prompts.json` |
| macOS | `~/Library/Application Support/com.promptdock.desktop/prompts.json` |

To use portable mode, copy `prompts.example.json` into the executable's directory, rename it to `prompts.json`, then quit and relaunch the app. The directory must be writable. On macOS, the executable is in `Prompt Dock.app/Contents/MacOS/`. Use the standard configuration directory for signed apps. When saving, Prompt Dock writes a temporary file in the same directory and then replaces the original. Quit the app before backing up or importing data.

The configuration is a JSON array. Each entry has `id`, `title`, `content`, and `color` fields. Each `id` must be unique. Supported colors are `blue`, `green`, `amber`, `purple`, `rose`, and `teal`.

## Local development and builds

Run these commands from the repository root. You need Node.js 24 LTS and stable Rust. Windows requires Microsoft C++ Build Tools, the Windows SDK, and the Edge WebView2 Runtime. macOS requires the Xcode Command Line Tools. [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

```sh
npm ci
npm run build
cargo check --locked --manifest-path src-tauri/Cargo.toml
npm run dev
```

Build the Windows x64 installer and standalone executable.

```powershell
rustup target add x86_64-pc-windows-msvc
npm run tauri -- build --target x86_64-pc-windows-msvc --bundles nsis -- --locked
```

- The standalone executable is at `src-tauri/target/x86_64-pc-windows-msvc/release/prompt-dock.exe`. It runs directly and uses the system WebView2 Runtime.
- The NSIS installer is under `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/`. It downloads WebView2 if needed.
- To build just the standalone executable, replace `--bundles nsis` with `--no-bundle`.

Build the `.app` and `.dmg` on an Apple Silicon Mac.

```sh
rustup target add aarch64-apple-darwin
npm run tauri -- build --target aarch64-apple-darwin --bundles app,dmg -- --locked
```

Find the outputs under `src-tauri/target/aarch64-apple-darwin/release/bundle/{macos,dmg}/`. The default workflow leaves Developer ID signing, macOS notarization, and Windows code signing unconfigured. Your operating system may block downloaded builds. [Tauri macOS signing guide](https://v2.tauri.app/distribute/sign/macos/)

## GitHub Actions

Run the **打包和发布** (Build and release) workflow manually to build both platforms in parallel and download the artifacts from the run. For a new release, update the version in `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, and `src-tauri/tauri.conf.json`. Create and push the matching tag, for example with `git tag v0.1.0` followed by `git push origin v0.1.0`. Once both builds succeed, the workflow uploads the standalone EXE, NSIS installer, DMG, and ZIP containing the `.app` to GitHub Releases. [View the workflow](https://github.com/shaoxiangm/prompt-dock/actions/workflows/release.yml)

To rebuild an existing release, run the workflow manually on `main` and set `release_tag` to an existing `v*` tag. The workflow checks out that tag and uploads the resulting artifacts to the matching release.

In a local Windows x64 build of version 0.1.0, the standalone EXE was 3.06 MiB and the NSIS installer was 1.15 MiB. On the test machine, the main process working set was approximately 12.65 MiB after silent startup and 8.55 MiB after the window closed and WebView2 was released. No WebView2 child processes were present in either sample.
