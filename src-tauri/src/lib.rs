//! DocMorph - Tauri 后端入口
mod commands;
mod engine;

use commands::ai::{ai_cloud_chat, ai_cloud_embed};
use commands::convert::{convert_document, get_engine_status, get_target_formats, EngineState};
use commands::fs::scan_directory;
use commands::pdf_tools::{
    docx_extract_images, get_pdf_page_count, image_compress, images_to_pdf, open_path,
    pdf_compare, pdf_compress, pdf_crop, pdf_decrypt, pdf_delete_pages, pdf_encrypt,
        pdf_extract_images, pdf_extract_pages, pdf_extract_text, pdf_merge, pdf_metadata,
    pdf_outline, pdf_page_numbers, pdf_remove_watermark, pdf_rotate, pdf_split,
    pdf_watermark, extract_text,
};
use commands::rename::batch_rename;
use commands::update::fetch_update_json;
use commands::watcher::{watcher_start, watcher_status, watcher_stop, WatcherState};
use commands::web::webpage_to_pdf;
use engine::libreoffice::LibreOfficeEngine;
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, AppHandle, Emitter, Manager, WindowEvent,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        // 单实例：重复启动时唤起已有实例的主窗口；Finder 右键传入的文件转发给前端
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            show_main_window(app);
            let files = collect_file_args(&args);
            if !files.is_empty() {
                let _ = app.emit("open-files", files);
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        // 设置持久化：JSON 文件存储于应用数据目录
        .plugin(tauri_plugin_store::Builder::default().build())
        // 系统通知：转换任务完成提醒
        .plugin(tauri_plugin_notification::init())
        // 全局快捷键：CommandOrControl+Shift+D 唤起主窗口（窗口可能隐藏到托盘）
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        // 开机启动：macOS 使用 LaunchAgent 方式
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .manage(EngineState(Arc::new(Mutex::new(LibreOfficeEngine::detect()))))
        // 文件夹监控实例（同一时刻最多一个）
        .manage(WatcherState(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            get_engine_status,
            get_target_formats,
            convert_document,
            pdf_merge,
            pdf_split,
            pdf_compress,
            pdf_watermark,
            pdf_page_numbers,
            pdf_rotate,
            pdf_encrypt,
            pdf_decrypt,
            pdf_extract_pages,
            pdf_delete_pages,
            images_to_pdf,
            get_pdf_page_count,
            scan_directory,
            open_path,
            get_launch_files,
            // 批次 A：轻量引擎扩展
            docx_extract_images,
            // 批次 B：PDF 工具箱扩展
            pdf_metadata,
            pdf_crop,
            pdf_outline,
            image_compress,
            // 批次 C：文件夹监控
            watcher_start,
            watcher_stop,
            watcher_status,
            // 新功能
            pdf_extract_images,
            pdf_remove_watermark,
            pdf_compare,
                        pdf_extract_text,
            extract_text,
            batch_rename,
            webpage_to_pdf,
            // AI 云端能力（OpenAI 兼容 API 转发）
            ai_cloud_chat,
            ai_cloud_embed,
            // 检查更新（后端代拉版本 JSON，规避 CORS）
            fetch_update_json,
        ])
        // 关闭窗口时隐藏到托盘（而不是退出应用），macOS 常规行为
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(|app| {
            // 收集启动参数中的文件（Finder「用 DocMorph 打开」首次唤起），供前端挂载后拉取
            let launch_files = collect_file_args(&std::env::args().collect::<Vec<_>>());
            app.manage(LaunchFiles(Mutex::new(launch_files)));
            setup_tray(app)?;
            // 全局快捷键：窗口隐藏到托盘后也能随时唤起；注册失败不阻断启动
            if let Err(e) = app.global_shortcut().register("CommandOrControl+Shift+D") {
                eprintln!("注册全局快捷键失败: {e}");
            }
            app.global_shortcut()
                .on_shortcut("CommandOrControl+Shift+D", |app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        show_main_window(app);
                    }
                })
                .unwrap_or_else(|e| eprintln!("注册快捷键处理回调失败: {e}"));
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        // macOS：处理 Finder「用 DocMorph 打开」的多选文件（Apple Events 方式传入）
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Opened { urls } = event {
            let files: Vec<String> = urls
                .iter()
                .filter(|u| u.scheme() == "file")
                .filter_map(|u| u.to_file_path().ok())
                .filter_map(|p| p.to_str().map(|s| s.to_string()))
                .collect();
            if !files.is_empty() {
                show_main_window(app_handle);
                let _ = app_handle.emit("open-files", files);
            }
        }
    });
}

/// 开机启动菜单项句柄（用于切换后同步勾选状态）
struct TrayState(Mutex<Option<CheckMenuItem<tauri::Wry>>>);

/// 启动时由 Finder 等外部唤起传入的文件路径（前端挂载完成后拉取一次）
struct LaunchFiles(Mutex<Vec<String>>);

/// 前端拉取启动参数中的文件路径（取走后清空，避免重复处理）
#[tauri::command]
fn get_launch_files(state: tauri::State<LaunchFiles>) -> Vec<String> {
    std::mem::take(&mut *state.0.lock().unwrap())
}

/// 从命令行参数中提取存在的文件路径（Finder 右键服务通过 `open -a DocMorph` 唤起）
fn collect_file_args(args: &[String]) -> Vec<String> {
    args.iter()
        .skip(1) // 第一个参数是程序自身路径
        .filter(|a| Path::new(a).is_file())
        .cloned()
        .collect()
}

/// 创建系统托盘：图标 + 菜单（显示主窗口 / 设置 / 开机启动 / 退出）
/// 左键单击切换主窗口显示/隐藏，右键弹出菜单
fn setup_tray(app: &App) -> tauri::Result<()> {
    let show_i = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
    let settings_i = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    // 开机启动开关：初始勾选状态与 LaunchAgent 实际状态同步
    let autostart_enabled = app.autolaunch().is_enabled().unwrap_or(false);
    let autostart_i = CheckMenuItem::with_id(
        app,
        "autostart",
        "Launch at Login",
        true,
        autostart_enabled,
        None::<&str>,
    )?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[&show_i, &settings_i, &sep1, &autostart_i, &sep2, &quit_i],
    )?;
    // 保存开机启动菜单项句柄，供菜单事件中同步勾选状态
    app.manage(TrayState(Mutex::new(Some(autostart_i))));

    // 托盘图标使用 macOS 模板图（纯黑 + alpha），系统自动适配明暗菜单栏
    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray-icon.png"))?;

    TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .icon_as_template(true)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
            "settings" => {
                // 窗口可能已隐藏：先恢复窗口，再通知前端打开设置弹窗
                show_main_window(app);
                let _ = app.emit("open-settings", ());
            }
            "autostart" => {
                // 切换开机启动，并将菜单勾选状态同步为实际结果
                let current = app.autolaunch().is_enabled().unwrap_or(false);
                let result = if current {
                    app.autolaunch().disable()
                } else {
                    app.autolaunch().enable()
                };
                if let Err(e) = result {
                    eprintln!("切换开机启动失败: {e}");
                }
                let enabled = app.autolaunch().is_enabled().unwrap_or(false);
                let state = app.state::<TrayState>();
                let guard = state.0.lock().unwrap();
                if let Some(item) = guard.as_ref() {
                    let _ = item.set_checked(enabled);
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // 左键单击：显示/隐藏主窗口
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        show_main_window(app);
                    }
                }
            }
        })
        .build(app)?;
    Ok(())
}

/// 显示并聚焦主窗口（从隐藏/最小化状态恢复）
fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
