//! 批量重命名命令
use serde::{Deserialize, Serialize};
use std::path::Path;

/// 单个重命名操作的结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameResult {
    pub old_path: String,
    pub new_path: String,
    pub ok: bool,
    pub error: Option<String>,
}

/// 批量重命名文件：items 为 [[old_path, new_name], ...] 列表
/// new_name 只包含文件名（不含目录），文件保留在原目录
#[tauri::command]
pub fn batch_rename(items: Vec<Vec<String>>) -> Result<Vec<RenameResult>, String> {
    let parsed: Vec<(&str, &str)> = items
        .iter()
        .map(|v| {
            if v.len() != 2 {
                return Err("每项必须是 [old_path, new_name] 格式".to_string());
            }
            Ok((v[0].as_str(), v[1].as_str()))
        })
        .collect::<Result<_, String>>()?;

    let mut results = Vec::new();

    for (old_path, new_name) in parsed {
        let old = Path::new(old_path);
        let new = old.with_file_name(new_name);

        // 禁止覆盖已有文件：避免 AI 命名或 pattern 生成同名导致数据丢失
        if new.exists() && new != old {
            let result = RenameResult {
                old_path: old_path.to_string(),
                new_path: new.to_string_lossy().to_string(),
                ok: false,
                error: Some(format!("目标文件已存在：{}", new.display())),
            };
            results.push(result);
            continue;
        }

        let result = match std::fs::rename(old, &new) {
            Ok(()) => RenameResult {
                old_path: old_path.to_string(),
                new_path: new.to_string_lossy().to_string(),
                ok: true,
                error: None,
            },
            Err(e) => RenameResult {
                old_path: old_path.to_string(),
                new_path: new.to_string_lossy().to_string(),
                ok: false,
                error: Some(e.to_string()),
            },
        };
        results.push(result);
    }

    Ok(results)
}