#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod storage;

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use storage::{Library, Prompt};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, State, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_clipboard_manager::ClipboardExt;

const TRAY: &str = "prompt-dock";
const MAIN: &str = "main";
const OPEN: &str = "open";
const AUTOSTART: &str = "autostart";
const EXIT: &str = "exit";
const PROMPT_PREFIX: &str = "prompt:";

struct Store(Mutex<Result<Library, String>>);
struct MainWindow(Mutex<WindowPhase>);
struct Editor(Mutex<Option<EditorState>>);

enum WindowPhase {
    Closed,
    Opening,
    Active,
    Closing { reopen: bool },
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct EditorState {
    selected_id: Option<String>,
    new_id: Option<String>,
    drafts: Vec<Prompt>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Change {
    Save { prompt: Prompt },
    Delete { id: String },
    Autostart { enabled: bool },
}

#[derive(Serialize)]
struct MenuStatus {
    autostart: Option<bool>,
    warning: Option<String>,
}

#[tauri::command]
fn load_prompts(store: State<'_, Store>) -> Result<Vec<Prompt>, String> {
    let library = store.0.lock().map_err(|error| error.to_string())?;
    Ok(library.as_ref().map_err(Clone::clone)?.prompts.clone())
}

#[tauri::command]
fn restore_editor(editor: State<'_, Editor>) -> Result<Option<EditorState>, String> {
    Ok(editor.0.lock().map_err(|error| error.to_string())?.clone())
}

#[tauri::command]
fn suspend_editor(app: AppHandle, workspace: EditorState) -> Result<(), String> {
    let Some(window) = app.get_webview_window(MAIN) else {
        return Ok(());
    };
    let lifecycle = app.state::<MainWindow>();
    let mut phase = lifecycle.0.lock().map_err(|error| error.to_string())?;
    if !matches!(*phase, WindowPhase::Active) || window.is_visible().map_err(|e| e.to_string())? {
        return Ok(());
    }
    let editor = app.state::<Editor>();
    *editor.0.lock().map_err(|error| error.to_string())? = Some(workspace);
    *phase = WindowPhase::Closing { reopen: false };
    if let Err(error) = window.destroy() {
        *phase = WindowPhase::Active;
        return Err(error.to_string());
    }
    Ok(())
}

fn set_autostart(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let manager = app.autolaunch();
    if enabled {
        manager.enable()
    } else {
        manager.disable()
    }
    .map_err(|error| format!("无法切换开机自启：{error}"))
}

#[tauri::command]
fn sync_tray_menu(app: AppHandle, change: Option<Change>) -> Result<MenuStatus, String> {
    match change {
        Some(Change::Autostart { enabled }) => {
            if let Err(error) = set_autostart(&app, enabled) {
                let _ = refresh_tray(&app);
                return Err(error);
            }
        }
        Some(change) => {
            let store = app.state::<Store>();
            let mut library = store.0.lock().map_err(|error| error.to_string())?;
            let library = library.as_mut().map_err(|error| error.clone())?;
            let mut prompts = library.prompts.clone();
            match change {
                Change::Save { mut prompt } => {
                    prompt.title = prompt.title.trim().to_owned();
                    if prompt.id.trim().is_empty() || prompt.title.is_empty() {
                        return Err("请输入指令名称。".into());
                    }
                    if prompt.content.trim().is_empty() {
                        return Err("请输入提示词内容。".into());
                    }
                    if let Some(existing) = prompts.iter_mut().find(|item| item.id == prompt.id) {
                        *existing = prompt;
                    } else {
                        prompts.push(prompt);
                    }
                }
                Change::Delete { id } => prompts.retain(|prompt| prompt.id != id),
                Change::Autostart { .. } => unreachable!(),
            }
            library.replace(prompts)?;
        }
        None => {}
    }
    Ok(refresh_tray(&app))
}

fn build_menu(app: &AppHandle, autostart: Option<bool>) -> tauri::Result<Menu<tauri::Wry>> {
    let menu = Menu::new(app)?;
    let store = app.state::<Store>();
    let prompts = store
        .0
        .lock()
        .ok()
        .and_then(|library| library.as_ref().ok().map(|library| library.prompts.clone()));
    for prompt in prompts.unwrap_or_default() {
        let item = MenuItem::with_id(
            app,
            format!("{PROMPT_PREFIX}{}", prompt.id),
            prompt.title,
            true,
            None::<&str>,
        )?;
        menu.append(&item)?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(
        app,
        OPEN,
        "打开指令库窗口",
        true,
        None::<&str>,
    )?)?;
    menu.append(&CheckMenuItem::with_id(
        app,
        AUTOSTART,
        "开机自启动",
        autostart.is_some(),
        autostart.unwrap_or(false),
        None::<&str>,
    )?)?;
    menu.append(&MenuItem::with_id(
        app,
        EXIT,
        "彻底退出",
        true,
        None::<&str>,
    )?)?;
    Ok(menu)
}

fn refresh_tray(app: &AppHandle) -> MenuStatus {
    let (autostart, mut warning) = match app.autolaunch().is_enabled() {
        Ok(enabled) => (Some(enabled), None),
        Err(error) => (None, Some(format!("无法读取开机自启状态：{error}"))),
    };
    if let Some(tray) = app.tray_by_id(TRAY) {
        if let Err(error) = build_menu(app, autostart).and_then(|menu| tray.set_menu(Some(menu))) {
            let message = format!("无法更新托盘菜单：{error}");
            warning =
                Some(warning.map_or_else(|| message.clone(), |old| format!("{old}；{message}")));
        }
    }
    MenuStatus { autostart, warning }
}

fn show_main(app: &AppHandle) {
    {
        let lifecycle = app.state::<MainWindow>();
        let mut phase = lifecycle.0.lock().expect("window lifecycle");
        match &mut *phase {
            WindowPhase::Opening => return,
            WindowPhase::Closing { reopen } => {
                *reopen = true;
                return;
            }
            WindowPhase::Closed | WindowPhase::Active => *phase = WindowPhase::Opening,
        }
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let show = || -> tauri::Result<()> {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Regular)?;
            let window = match app.get_webview_window(MAIN) {
                Some(window) => window,
                None => {
                    let config = &app.config().app.windows[0];
                    WebviewWindowBuilder::from_config(&app, config)?.build()?
                }
            };
            window.unminimize()?;
            window.show()?;
            window.set_focus()
        };
        if let Err(error) = show() {
            eprintln!("无法打开指令库窗口：{error}");
        }
        *app.state::<MainWindow>()
            .0
            .lock()
            .expect("window lifecycle") = if app.get_webview_window(MAIN).is_some() {
            WindowPhase::Active
        } else {
            WindowPhase::Closed
        };
    });
}

fn window_destroyed(app: &AppHandle) {
    let reopen = {
        let lifecycle = app.state::<MainWindow>();
        let mut phase = lifecycle.0.lock().expect("window lifecycle");
        let previous = std::mem::replace(&mut *phase, WindowPhase::Closed);
        matches!(previous, WindowPhase::Closing { reopen: true })
    };
    if reopen {
        show_main(app);
    } else {
        #[cfg(windows)]
        {
            use windows_sys::Win32::System::{
                ProcessStatus::K32EmptyWorkingSet, Threading::GetCurrentProcess,
            };
            // Release resident pages left by WebView2 after the last window is destroyed.
            // The current-process pseudo-handle is valid here and does not need to be closed.
            unsafe { K32EmptyWorkingSet(GetCurrentProcess()) };
        }
    }
}

fn handle_menu(app: &AppHandle, id: &str) {
    match id {
        OPEN => show_main(app),
        EXIT => app.exit(0),
        AUTOSTART => {
            let result = app
                .autolaunch()
                .is_enabled()
                .map_err(|error| format!("无法读取开机自启状态：{error}"))
                .and_then(|enabled| set_autostart(app, !enabled));
            let mut status = refresh_tray(app);
            if let Err(error) = result {
                status.warning = Some(error);
            }
            if let Some(message) = &status.warning {
                eprintln!("{message}");
            }
            let _ = app.emit("autostart-changed", &status);
        }
        id => {
            if let Some(id) = id.strip_prefix(PROMPT_PREFIX) {
                let store = app.state::<Store>();
                let content = store.0.lock().ok().and_then(|library| {
                    library
                        .as_ref()
                        .ok()?
                        .prompts
                        .iter()
                        .find(|prompt| prompt.id == id)
                        .map(|prompt| prompt.content.clone())
                });
                if let Some(content) = content {
                    if let Err(error) = app.clipboard().write_text(content) {
                        let message = format!("无法写入剪贴板：{error}");
                        eprintln!("{message}");
                        let _ = app.emit("tray-error", &message);
                    }
                }
            }
        }
    }
}

fn is_silent(args: impl IntoIterator<Item = impl AsRef<str>>) -> bool {
    args.into_iter().any(|arg| arg.as_ref() == "--silent")
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if !is_silent(args.iter().skip(1)) {
                show_main(app);
            }
        }))
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .app_name("PromptDock")
                .args(["--silent"])
                .build(),
        )
        .manage(MainWindow(Mutex::new(WindowPhase::Closed)))
        .manage(Editor(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            load_prompts,
            sync_tray_menu,
            restore_editor,
            suspend_editor,
        ])
        .on_window_event(|window, event| {
            if window.label() == MAIN {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    if let Err(error) = window.hide() {
                        eprintln!("无法隐藏窗口：{error}");
                    } else {
                        let _ = window.emit("editor-hidden", ());
                    }
                    #[cfg(target_os = "macos")]
                    let _ = window
                        .app_handle()
                        .set_activation_policy(tauri::ActivationPolicy::Accessory);
                }
            }
        })
        .setup(|app| {
            let library = Library::open(app.handle()).map_err(|error| error.to_string());
            if let Err(error) = &library {
                eprintln!("无法读取指令库：{error}");
            }
            app.manage(Store(Mutex::new(library)));
            let silent = is_silent(
                std::env::args_os()
                    .skip(1)
                    .filter_map(|arg| arg.into_string().ok()),
            );
            #[cfg(target_os = "macos")]
            if silent {
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            }
            #[cfg(target_os = "macos")]
            let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray-macos.png"))?;
            #[cfg(not(target_os = "macos"))]
            let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png"))?;
            TrayIconBuilder::with_id(TRAY)
                .icon(icon)
                .icon_as_template(cfg!(target_os = "macos"))
                .tooltip("快捷指令提示词库")
                .menu(&build_menu(
                    app.handle(),
                    app.autolaunch().is_enabled().ok(),
                )?)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| handle_menu(app, event.id().as_ref()))
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main(tray.app_handle());
                    }
                })
                .build(app)?;
            if !silent {
                show_main(app.handle());
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("无法启动快捷指令提示词库")
        .run(|app, event| match event {
            tauri::RunEvent::WindowEvent {
                label,
                event: WindowEvent::Destroyed,
                ..
            } if label == MAIN => window_destroyed(app),
            tauri::RunEvent::ExitRequested {
                code: None, api, ..
            } => api.prevent_exit(),
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => show_main(app),
            _ => {}
        });
}
