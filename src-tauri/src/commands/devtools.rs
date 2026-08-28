//! 开发者工具：设置页排障入口，打开主窗口的 WebView 调试面板。
//! release 构建需 Cargo.toml 开启 tauri 的 devtools feature（已开启）。

use tauri::{AppHandle, Manager};

/// 打开主窗口的开发者工具（Console 看 JS 报错、Network 看请求）
#[tauri::command]
pub fn open_devtools(app: AppHandle) -> Result<(), String> {
    let win = app
        .get_webview_window("main")
        .ok_or_else(|| "主窗口不存在".to_string())?;
    win.open_devtools();
    Ok(())
}
