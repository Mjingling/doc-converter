//! 全局快捷键配置：前端设置页可修改（settings store 持久化，启动后由前端推送注册）
//!
//! 两类快捷键（kind）：
//! - main：唤起主窗口
//! - assistant：唤起主窗口并切换到 AI 助手（emit "open-assistant"，前端 Home 已监听）

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Mutex;

use tauri::{AppHandle, Emitter};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

/// 每类快捷键当前已注册的键位（kind → shortcut 原文）
static REGISTERED: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

/// 记录某类快捷键已注册的键位（供后续修改时精准注销，避免误删另一类）
fn remember(kind: &str, shortcut: &str) {
    let mut map = REGISTERED.lock().expect("shortcut registry poisoned");
    let map = map.get_or_insert_with(HashMap::new);
    map.insert(kind.to_string(), shortcut.to_string());
}

/// 注销并遗忘某类快捷键（kind 已注册时返回其旧键位）
fn forget(kind: &str) -> Option<String> {
    let mut map = REGISTERED.lock().expect("shortcut registry poisoned");
    map.as_mut()?.remove(kind)
}

/// 设置全局快捷键；空字符串表示禁用该类快捷键
///
/// 变更流程：校验 kind → 解析校验新键 → 只注销该类旧键 → on_shortcut 注册
/// 注意：on_shortcut 内部已含 OS 层注册，不可再对同一快捷键调用 register（会重复注册报错）
#[tauri::command]
pub fn set_global_shortcut(
    app: AppHandle,
    kind: String,
    shortcut: String,
) -> Result<(), String> {
    if kind != "main" && kind != "assistant" {
        return Err(format!("未知的快捷键类型: {kind}"));
    }
    let gs = app.global_shortcut();
    let shortcut = shortcut.trim().to_string();

    // 禁用：仅注销该类旧键，不注册
    if shortcut.is_empty() {
        if let Some(old) = forget(&kind) {
            gs.unregister(old.as_str()).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }

    // 先校验格式，避免注销旧键后新键注册失败导致快捷键丢失
    if Shortcut::from_str(&shortcut).is_err() {
        return Err(format!("无效的快捷键格式: {shortcut}"));
    }

    // 注销该类旧键（不 unregister_all，避免误删另一类）
    if let Some(old) = forget(&kind) {
        gs.unregister(old.as_str()).map_err(|e| e.to_string())?;
    }

    let handler_app = app.clone();
    let kind_clone = kind.clone();
    gs.on_shortcut(shortcut.as_str(), move |_app, _sc, event| {
        if event.state != ShortcutState::Pressed {
            return;
        }
        crate::show_main_window(&handler_app);
        if kind_clone == "assistant" {
            // 通知前端切到 AI 助手面板（Home 侧 listen("open-assistant")）
            let _ = handler_app.emit("open-assistant", ());
        }
    })
    .map_err(|e| e.to_string())?;

    remember(&kind, &shortcut);
    Ok(())
}
