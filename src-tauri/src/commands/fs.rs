//! 文件系统命令：递归扫描目录收集指定扩展名的文件（批量转换用），
//! 以及单层列目录 / 读文本 / 写文本（AI 助手文件工具用）
use std::collections::HashSet;
use std::path::Path;
use serde::{Deserialize, Serialize};

/// 扫描上限，防止误选超大目录导致卡死
const MAX_FILES: usize = 5000;

/// 读文本文件默认字节上限（256 KiB），防止超大文件撑爆 LLM 上下文
const READ_TEXT_MAX_BYTES: u64 = 256 * 1024;

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

/// 目录条目信息（list_dir 返回）
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirEntryInfo {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    /// 文件字节数（目录为 0）
    pub size: u64,
}

/// 单层列出目录内容：目录在前、同类型按名称排序，跳过隐藏文件
#[tauri::command]
pub fn list_dir(dir: String) -> Result<Vec<DirEntryInfo>, String> {
    let root = Path::new(&dir);
    if !root.is_dir() {
        return Err("所选路径不是文件夹".into());
    }
    let mut dirs: Vec<DirEntryInfo> = Vec::new();
    let mut files: Vec<DirEntryInfo> = Vec::new();
    let entries = std::fs::read_dir(root).map_err(|e| format!("读取目录失败: {e}"))?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue; // 隐藏文件/目录
        }
        let path = entry.path();
        let meta = entry.metadata();
        let is_dir = path.is_dir();
        let size = if is_dir { 0 } else { meta.map(|m| m.len()).unwrap_or(0) };
        let info = DirEntryInfo { name, path: path.to_string_lossy().to_string(), is_dir, size };
        if is_dir {
            dirs.push(info);
        } else {
            files.push(info);
        }
    }
    dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    dirs.extend(files);
    Ok(dirs)
}

/// 读文本文件结果：content 可能被截断（truncated=true 时 total_bytes 为原始大小）
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadTextResult {
    pub content: String,
    pub total_bytes: u64,
    pub truncated: bool,
}

/// 读取 UTF-8 文本文件，超过 max_bytes（默认 256 KiB）时截断
#[tauri::command]
pub fn read_text_file(path: String, max_bytes: Option<u64>) -> Result<ReadTextResult, String> {
    let limit = max_bytes.unwrap_or(READ_TEXT_MAX_BYTES).max(1024);
    let bytes = std::fs::read(&path).map_err(|e| format!("读取文件失败: {e}"))?;
    let total = bytes.len() as u64;
    let truncated = total > limit;
    let slice: &[u8] = if truncated { &bytes[..limit as usize] } else { &bytes };
    // 截断可能切断多字节字符：从后向前回退到合法 UTF-8 边界
    let mut end = slice.len();
    if truncated {
        while end > 0 && std::str::from_utf8(&slice[..end]).is_err() {
            end -= 1;
        }
    }
    let content = String::from_utf8(slice[..end].to_vec())
        .map_err(|_| "不是有效的 UTF-8 文本文件（可能是二进制文件）".to_string())?;
    Ok(ReadTextResult { content, total_bytes: total, truncated })
}

/// 写入 UTF-8 文本文件：自动创建父目录；overwrite=false（默认）时已存在则报错
#[tauri::command]
pub fn write_text_file(
    path: String,
    content: String,
    overwrite: Option<bool>,
) -> Result<String, String> {
    let target = Path::new(&path);
    if target.exists() && !overwrite.unwrap_or(false) {
        return Err(format!("文件已存在（未允许覆盖）: {path}"));
    }
    if let Some(parent) = target.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建输出目录失败: {e}"))?;
        }
    }
    std::fs::write(target, content).map_err(|e| format!("写入文件失败: {e}"))?;
    Ok(path)
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

    #[test]
    fn test_list_dir_basic() {
        let tmp = std::env::temp_dir().join("docmorph_test_list_dir");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("sub")).unwrap();
        fs::write(tmp.join("b.txt"), "hello").unwrap();
        fs::write(tmp.join("a.txt"), "hi").unwrap();
        fs::write(tmp.join(".hidden"), "x").unwrap();

        let result = list_dir(tmp.to_string_lossy().to_string()).unwrap();
        // 目录在前：sub, a.txt, b.txt（跳过隐藏文件）
        assert_eq!(result.len(), 3, "应列出 3 个条目: {:?}", result);
        assert!(result[0].is_dir, "目录应排在最前");
        assert_eq!(result[0].name, "sub");
        assert_eq!(result[1].name, "a.txt");
        assert_eq!(result[2].name, "b.txt");
        assert_eq!(result[1].size, 2, "a.txt 大小应为 2 字节");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_list_dir_not_dir() {
        assert!(list_dir("not_a_real_dir_xyz".into()).is_err(), "非目录应报错");
    }

    #[test]
    fn test_read_text_file_roundtrip() {
        let tmp = std::env::temp_dir().join("docmorph_test_read_text.txt");
        fs::write(&tmp, "你好，世界 Hello").unwrap();

        let r = read_text_file(tmp.to_string_lossy().to_string(), None).unwrap();
        assert_eq!(r.content, "你好，世界 Hello");
        assert!(!r.truncated);
        assert_eq!(r.total_bytes, "你好，世界 Hello".len() as u64);

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_read_text_file_truncate() {
        let tmp = std::env::temp_dir().join("docmorph_test_read_trunc.txt");
        // 3000 字节中文（每字 3 字节），限制 2000 字节 → 截断在 UTF-8 边界
        let text = "好".repeat(1000);
        fs::write(&tmp, &text).unwrap();

        let r = read_text_file(tmp.to_string_lossy().to_string(), Some(2000)).unwrap();
        assert!(r.truncated, "应标记截断");
        assert_eq!(r.total_bytes, 3000);
        // 2000 / 3 = 666 完整字符 + 2 字节残缺回退 → 666 字符
        assert_eq!(r.content.chars().count(), 666, "截断应落在 UTF-8 边界");

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_read_text_file_invalid_utf8() {
        let tmp = std::env::temp_dir().join("docmorph_test_read_bin.bin");
        fs::write(&tmp, [0xFFu8, 0xFE, 0x00, 0x01, 0x02]).unwrap();
        assert!(read_text_file(tmp.to_string_lossy().to_string(), None).is_err(), "二进制应报错");
        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_write_text_file_protect_and_overwrite() {
        let tmp = std::env::temp_dir().join("docmorph_test_write/nested");
        let _ = fs::remove_dir_all(tmp.parent().unwrap());
        let file = tmp.join("out.md");

        // 默认不覆盖：新路径可写（自动创建父目录）
        let p = write_text_file(file.to_string_lossy().to_string(), "v1".into(), None).unwrap();
        assert_eq!(p, file.to_string_lossy());
        assert!(file.exists(), "应自动创建父目录并写入");

        // 已存在 + 未允许覆盖 → 报错
        assert!(write_text_file(file.to_string_lossy().to_string(), "v2".into(), None).is_err());

        // 显式允许覆盖 → 成功
        write_text_file(file.to_string_lossy().to_string(), "v3".into(), Some(true)).unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), "v3", "内容应为覆盖后的 v3");

        let _ = fs::remove_dir_all(tmp.parent().unwrap());
    }
}
