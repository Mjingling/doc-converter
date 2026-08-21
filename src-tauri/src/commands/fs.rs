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

/// 获取平台默认输出目录：
/// - Windows：可执行文件所在目录下的 output 子目录
/// - macOS/Linux：~/Downloads/docMorph
/// 返回前尝试创建目录（失败不阻断，仅返回路径）
#[tauri::command]
pub fn get_default_output_dir() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let exe = std::env::current_exe().map_err(|e| format!("无法获取可执行文件路径: {e}"))?;
        let install_dir = exe.parent().ok_or("无法获取安装目录")?;
        let dir = install_dir.join("output");
        let _ = std::fs::create_dir_all(&dir); // 尝试创建，失败不阻断
        Ok(dir.to_string_lossy().to_string())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var_os("HOME").ok_or("无法获取用户主目录")?;
        let dir = std::path::PathBuf::from(home)
            .join("Downloads")
            .join("docMorph");
        let _ = std::fs::create_dir_all(&dir); // 尝试创建，失败不阻断
        Ok(dir.to_string_lossy().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_get_default_output_dir() {
        let dir = get_default_output_dir().unwrap();
        #[cfg(not(target_os = "windows"))]
        {
            assert!(dir.contains("Downloads"), "路径应包含 Downloads: {}", dir);
            assert!(dir.ends_with("docMorph"), "路径应以 docMorph 结尾: {}", dir);
        }
        #[cfg(target_os = "windows")]
        {
            assert!(dir.ends_with("output"), "路径应以 output 结尾: {}", dir);
        }
    }

    #[test]
    fn test_scan_directory_basic() {
        let tmp = std::env::temp_dir().join("docmorph_test_scan_basic");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        fs::write(tmp.join("a.pdf"), "test").unwrap();
        fs::write(tmp.join("b.pdf"), "test").unwrap();
        fs::write(tmp.join("c.txt"), "test").unwrap();
        fs::create_dir_all(tmp.join("sub")).unwrap();
        fs::write(tmp.join("sub").join("d.pdf"), "test").unwrap();
        fs::write(tmp.join(".hidden.pdf"), "test").unwrap();

        // 只找 PDF：a.pdf, b.pdf, sub/d.pdf（跳过隐藏文件）
        let result = scan_directory(tmp.to_string_lossy().to_string(), vec!["pdf".into()]).unwrap();
        assert_eq!(result.len(), 3, "应找到 3 个 PDF: {:?}", result);

        // 多扩展名：pdf + txt
        let result = scan_directory(tmp.to_string_lossy().to_string(), vec!["pdf".into(), "txt".into()]).unwrap();
        assert_eq!(result.len(), 4, "应找到 4 个文件: {:?}", result);

        // 带点前缀的扩展名也能识别
        let result = scan_directory(tmp.to_string_lossy().to_string(), vec![".pdf".into()]).unwrap();
        assert_eq!(result.len(), 3, "带点前缀应正确处理: {:?}", result);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_scan_directory_not_dir() {
        let result = scan_directory("not_a_real_dir_xyz".into(), vec!["pdf".into()]);
        assert!(result.is_err(), "非目录路径应返回错误");
    }

    #[test]
    fn test_scan_directory_empty_dir() {
        let tmp = std::env::temp_dir().join("docmorph_test_scan_empty");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let result = scan_directory(tmp.to_string_lossy().to_string(), vec!["pdf".into()]).unwrap();
        assert!(result.is_empty(), "空目录应返回空列表");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_scan_directory_case_insensitive() {
        let tmp = std::env::temp_dir().join("docmorph_test_scan_case");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        fs::write(tmp.join("upper.PDF"), "test").unwrap();
        fs::write(tmp.join("lower.pdf"), "test").unwrap();

        // 扩展名大小写不敏感
        let result = scan_directory(tmp.to_string_lossy().to_string(), vec!["pdf".into()]).unwrap();
        assert_eq!(result.len(), 2, "应不区分大小写: {:?}", result);

        let _ = fs::remove_dir_all(&tmp);
    }
}
