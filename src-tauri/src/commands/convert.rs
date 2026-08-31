//! 转换引擎相关命令
use crate::engine::format::Format;
use crate::engine::libreoffice::LibreOfficeEngine;
use crate::engine::light;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::State;

/// 全局引擎状态（LibreOffice 转换需串行执行）
pub struct EngineState(pub Arc<Mutex<LibreOfficeEngine>>);

#[derive(Serialize)]
pub struct EngineStatus {
    pub available: bool,
    pub path: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct FormatInfo {
    pub ext: String,
    pub label: String,
    pub targets: Vec<FormatInfo>,
}

#[tauri::command]
pub fn get_engine_status(state: State<EngineState>) -> Result<EngineStatus, String> {
    // 每次调用重新检测并刷新状态：用户可能刚安装 LibreOffice，避免返回启动时的过期缓存
    let fresh = LibreOfficeEngine::detect();
    let mut engine = state.0.lock().map_err(|_| "引擎状态锁获取失败".to_string())?;
    *engine = fresh;
    Ok(EngineStatus {
        available: engine.available(),
        path: engine.binary.as_ref().map(|p| p.to_string_lossy().to_string()),
    })
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
/// conflict 为输出文件已存在时的策略："overwrite" 直接覆盖，"rename" 自动递增序号
#[tauri::command]
pub async fn convert_document(
    state: State<'_, EngineState>,
    input_path: String,
    target_ext: String,
    out_dir: String,
    engine: String,
    conflict: String,
) -> Result<String, String> {
    let out_dir_p = PathBuf::from(&out_dir);
    let rename_on_conflict = conflict == "rename";
    let engine_state = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let base_out = build_out_path(Path::new(&input_path), &target_ext, &out_dir_p)?;
        // rename 策略且目标已存在：先转到临时目录转换，产物再改名到可用序号名，避免覆盖旧文件
        let needs_unique = rename_on_conflict && base_out.exists();
        let work_dir = if needs_unique {
            let tmp = std::env::temp_dir()
                .join(format!("docmorph-convert-{}", std::process::id()));
            std::fs::create_dir_all(&tmp).map_err(|e| format!("临时目录创建失败: {e}"))?;
            tmp
        } else {
            out_dir_p.clone()
        };

        let produced = if engine == "builtin" {
            light::convert_light(Path::new(&input_path), &target_ext, &work_dir)?
        } else {
            let eng = engine_state
                .lock()
                .map_err(|_| "引擎锁获取失败".to_string())?;
            eng.convert(Path::new(&input_path), &target_ext, &work_dir)?
        };

        if needs_unique {
            let final_out = next_available_path(&base_out);
            std::fs::rename(&produced, &final_out)
                .map_err(|e| format!("输出改名失败: {e}"))?;
            Ok(final_out.to_string_lossy().to_string())
        } else {
            Ok(produced.to_string_lossy().to_string())
        }
    })
    .await
    .map_err(|e| format!("转换线程失败: {}", e))?
}

/// 目标输出路径（out_dir / 输入文件名.目标扩展名）
fn build_out_path(input: &Path, target_ext: &str, out_dir: &Path) -> Result<PathBuf, String> {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "无法识别输入文件名".to_string())?;
    Ok(out_dir.join(format!("{stem}.{target_ext}")))
}

/// rename 冲突策略：name.ext 已存在时返回 name (1).ext、name (2).ext … 第一个不存在的序号
fn next_available_path(base: &Path) -> PathBuf {
    if !base.exists() {
        return base.to_path_buf();
    }
    let dir = base.parent().unwrap_or(Path::new("."));
    let stem = base
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let ext = base.extension().and_then(|e| e.to_str()).unwrap_or("");
    for i in 1..10_000 {
        let name = if ext.is_empty() {
            format!("{stem} ({i})")
        } else {
            format!("{stem} ({i}).{ext}")
        };
        let candidate = dir.join(name);
        if !candidate.exists() {
            return candidate;
        }
    }
    base.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_next_available_path() {
        let base = PathBuf::from("/tmp/docmorph-test/out.txt");
        // 基础路径不存在时原样返回
        assert_eq!(next_available_path(&base), base);
    }

    #[test]
    fn test_next_available_path_takes_index_when_exists() {
        let dir = std::env::temp_dir().join(format!("docmorph-conflict-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let base = dir.join("out.txt");
        std::fs::write(&base, "old").unwrap();
        let next = next_available_path(&base);
        assert_eq!(next, dir.join("out (1).txt"));
        std::fs::write(&next, "old2").unwrap();
        assert_eq!(next_available_path(&base), dir.join("out (2).txt"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_get_target_formats_docx_builtin() {
        let result = get_target_formats("test.docx".to_string(), "builtin".to_string());
        assert!(result.is_ok());
        let formats = result.unwrap();
        // docx 轻量引擎支持 txt/html/md
        let exts: Vec<&str> = formats.iter().map(|f| f.ext.as_str()).collect();
        assert!(exts.contains(&"txt"), "docx builtin 应支持 txt");
    }

    #[test]
    fn test_get_target_formats_docx_libreoffice() {
        let result = get_target_formats("test.docx".to_string(), "libreoffice".to_string());
        assert!(result.is_ok());
        let formats = result.unwrap();
        let exts: Vec<&str> = formats.iter().map(|f| f.ext.as_str()).collect();
        assert!(exts.contains(&"pdf"), "docx libreoffice 应支持 pdf");
    }

    #[test]
    fn test_get_target_formats_pdf_builtin() {
        let result = get_target_formats("test.pdf".to_string(), "builtin".to_string());
        assert!(result.is_ok());
        let formats = result.unwrap();
        // PDF 的 light_targets 为空（不支持轻量转换）
        assert!(formats.is_empty(), "pdf builtin 应返回空列表");
    }

    #[test]
    fn test_get_target_formats_unsupported_ext() {
        let result = get_target_formats("test.xyz".to_string(), "builtin".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不支持"));
    }

    #[test]
    fn test_get_target_formats_no_ext() {
        let result = get_target_formats("noextension".to_string(), "builtin".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("扩展名"));
    }

    #[test]
    fn test_get_target_formats_case_insensitive() {
        let result = get_target_formats("TEST.DOCX".to_string(), "builtin".to_string());
        assert!(result.is_ok());
        let formats = result.unwrap();
        assert!(!formats.is_empty());
    }
}
