//! 文件夹监控命令：启动 / 停止 / 查询状态
use crate::engine::watcher::{self, WatcherHandle};
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, State};

/// 当前监控实例（同一时刻最多一个；重新启动前先停止旧实例）
pub struct WatcherState(pub Mutex<Option<WatcherHandle>>);

#[derive(Serialize)]
pub struct WatcherStatus {
    pub running: bool,
    pub folder: Option<String>,
}

/// 启动文件夹监控；targets 为「扩展名 → 目标扩展名」映射（如 docx → pdf）
#[tauri::command]
pub fn watcher_start(
    app: AppHandle,
    state: State<WatcherState>,
    folder: String,
    targets: HashMap<String, String>,
) -> Result<(), String> {
    let mut guard = state.0.lock().map_err(|_| "监控状态锁获取失败".to_string())?;
    if let Some(old) = guard.take() {
        let _ = old.stop_tx.send(());
    }
    let handle = watcher::start_watcher(app, PathBuf::from(&folder), targets)?;
    *guard = Some(handle);
    Ok(())
}

/// 停止文件夹监控
#[tauri::command]
pub fn watcher_stop(state: State<WatcherState>) -> Result<(), String> {
    let mut guard = state.0.lock().map_err(|_| "监控状态锁获取失败".to_string())?;
    if let Some(old) = guard.take() {
        let _ = old.stop_tx.send(());
    }
    Ok(())
}

/// 查询监控状态（是否运行中、监控目录）
#[tauri::command]
pub fn watcher_status(state: State<WatcherState>) -> Result<WatcherStatus, String> {
    let guard = state.0.lock().map_err(|_| "监控状态锁获取失败".to_string())?;
    Ok(match guard.as_ref() {
        Some(h) => WatcherStatus {
            running: true,
            folder: Some(h.folder.to_string_lossy().to_string()),
        },
        None => WatcherStatus {
            running: false,
            folder: None,
        },
    })
}
