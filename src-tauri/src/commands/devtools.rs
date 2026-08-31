//! 开发者工具：设置页排障入口，打开主窗口的 WebView 调试面板。
//! release 构建需 Cargo.toml 开启 tauri 的 devtools feature（已开启）。

use tauri::{AppHandle, Manager};

/// 最简单的 ping 测试：验证 Rust 命令是否被正确调用
#[tauri::command]
pub fn ping() -> String {
    eprintln!("[ping] called from frontend");
    "pong".to_string()
}

/// 打开主窗口的开发者工具（Console 看 JS 报错、Network 看请求）
#[tauri::command]
pub fn open_devtools(app: AppHandle) -> Result<(), String> {
    eprintln!("[open_devtools] called");
    let win = app
        .get_webview_window("main")
        .ok_or_else(|| "主窗口不存在".to_string())?;
    eprintln!("[open_devtools] window found, calling open_devtools()");
    // open_devtools() 返回 ()：release 未开 devtools feature 时会静默无效果，
    // 用 is_devtools_open 复核，避免前端误报"已打开"误导排障
    win.open_devtools();
    if win.is_devtools_open() {
        eprintln!("[open_devtools] devtools opened");
        Ok(())
    } else {
        Err("开发者工具未打开：release 构建需 Cargo.toml 为 tauri 启用 devtools feature".to_string())
    }
}
