//! 文件系统命令：递归扫描目录收集指定扩展名的文件（批量转换用）
use std::collections::HashSet;
use std::path::Path;

/// 扫描上限，防止误选超大目录导致卡死
const MAX_FILES: usize = 5000;

/// 递归扫描目录，返回所有扩展名匹配的文件绝对路径
/// - 跳过隐藏文件/目录（以 . 开头）
/// - 静默跳过无权限访问的目录
/// - 超过 MAX_FILES 上限时停止并返回已收集结果
#[tauri::command]
pub fn scan_directory(dir: String, exts: Vec<String>) -> Result<Vec<String>, String> {
    let root = Path::new(&dir);
    if !root.is_dir() {
        return Err("所选路径不是文件夹".into());
    }
    let exts: Vec<String> = exts
        .iter()
        .map(|e| e.trim_start_matches('.').to_lowercase())
        .collect();
    let mut result = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    // 已遍历目录（规范化路径）集合：防止符号链接环导致无限循环
    let mut visited = HashSet::new();
    while let Some(d) = stack.pop() {
        // 符号链接可能指回上级目录形成环，规范化后只遍历一次
        let canon = std::fs::canonicalize(&d).unwrap_or_else(|_| d.clone());
        if !visited.insert(canon) {
            continue;
        }
        let entries = match std::fs::read_dir(&d) {
            Ok(e) => e,
            Err(_) => continue, // 无权限等场景静默跳过
        };
        for entry in entries.flatten() {
            if result.len() >= MAX_FILES {
                return Ok(result);
            }
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with('.') {
                continue; // 隐藏文件/目录
            }
            if path.is_dir() {
                stack.push(path);
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if exts.contains(&ext.to_lowercase()) {
                    result.push(path.to_string_lossy().to_string());
                }
            }
        }
    }
    Ok(result)
}
