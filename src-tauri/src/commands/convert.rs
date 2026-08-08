//! 转换引擎相关命令
use crate::engine::format::Format;
use crate::engine::libreoffice::LibreOfficeEngine;
use crate::engine::light;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::State;

/// 全局引擎状态（LibreOffice 转换需串行执行）
pub struct EngineState(pub Mutex<LibreOfficeEngine>);

#[derive(Serialize)]
pub struct EngineStatus {
    pub available: bool,
    pub path: Option<String>,
}

#[derive(Serialize)]
pub struct FormatInfo {
    pub ext: String,
    pub label: String,
    pub targets: Vec<FormatInfo>,
}

#[tauri::command]
pub fn get_engine_status(state: State<EngineState>) -> EngineStatus {
    let engine = state.0.lock().unwrap();
    EngineStatus {
        available: engine.available(),
        path: engine.binary.as_ref().map(|p| p.to_string_lossy().to_string()),
    }
}

/// 获取目标格式列表；engine 为 "builtin" 时返回轻量（文本提取）矩阵，否则返回 LibreOffice 完整矩阵
#[tauri::command]
pub fn get_target_formats(input_path: String, engine: String) -> Result<Vec<FormatInfo>, String> {
    let ext = Path::new(&input_path)
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| "无法识别文件扩展名".to_string())?;
    let fmt = Format::from_ext(ext).ok_or_else(|| format!("不支持的格式: .{}", ext))?;
    let targets = if engine == "builtin" {
        fmt.light_targets()
    } else {
        fmt.targets()
    };
    Ok(targets
        .into_iter()
        .map(|t| FormatInfo {
            ext: t.ext().to_string(),
            label: t.label().to_string(),
            targets: vec![],
        })
        .collect())
}

/// 执行文档转换；engine 为 "builtin" 时走轻量引擎（docx→txt/html/md、xlsx→csv、pptx→txt），
/// 否则走 LibreOffice 引擎（完整版式转换）
#[tauri::command]
pub fn convert_document(
    state: State<EngineState>,
    input_path: String,
    target_ext: String,
    out_dir: String,
    engine: String,
) -> Result<String, String> {
    let out_dir_p = PathBuf::from(&out_dir);
    if engine == "builtin" {
        let out = light::convert_light(Path::new(&input_path), &target_ext, &out_dir_p)?;
        return Ok(out.to_string_lossy().to_string());
    }
    let engine = state.0.lock().map_err(|_| "引擎锁获取失败".to_string())?;
    let out = engine.convert(Path::new(&input_path), &target_ext, &out_dir_p)?;
    Ok(out.to_string_lossy().to_string())
}
