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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU32, Ordering};

    static TMP_SEQ: AtomicU32 = AtomicU32::new(0);

    fn tmp_dir() -> std::path::PathBuf {
        let n = TMP_SEQ.fetch_add(1, Ordering::Relaxed);
        let d = std::env::temp_dir().join(format!("rename_test_{}_{}", std::process::id(), n));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn test_batch_rename_ok_and_errors() {
        let d = tmp_dir();
        let a = d.join("a.txt");
        let b = d.join("b.txt");
        fs::write(&a, "A").unwrap();
        fs::write(&b, "B").unwrap();

        // 正常重命名：文件保留在原目录并改名
        let r = batch_rename(vec![vec![
            a.to_string_lossy().to_string(),
            "a_new.txt".into(),
        ]])
        .unwrap();
        assert_eq!(r.len(), 1);
        assert!(r[0].ok, "重命名应成功: {:?}", r);
        assert!(d.join("a_new.txt").exists(), "新文件名应存在");
        assert!(!d.join("a.txt").exists(), "旧文件名应不存在");
        assert_eq!(fs::read_to_string(d.join("a_new.txt")).unwrap(), "A", "内容应保留");

        // 目标已存在 → 单项失败且不覆盖
        let r = batch_rename(vec![vec![
            b.to_string_lossy().to_string(),
            "a_new.txt".into(),
        ]])
        .unwrap();
        assert!(!r[0].ok, "目标已存在应失败: {:?}", r);
        assert!(
            r[0].error.as_ref().unwrap().contains("目标文件已存在"),
            "错误信息应明确: {:?}",
            r[0].error
        );
        assert_eq!(fs::read_to_string(d.join("a_new.txt")).unwrap(), "A", "已有文件不应被覆盖");

        // 源文件不存在 → 单项失败
        let r = batch_rename(vec![vec![
            d.join("missing.txt").to_string_lossy().to_string(),
            "x.txt".into(),
        ]])
        .unwrap();
        assert!(!r[0].ok, "源文件不存在应失败: {:?}", r);

        // 格式错误（非两项）→ 整体报错
        assert!(batch_rename(vec![vec!["only-one".into()]]).is_err(), "单项格式错误应整体报错");
        assert!(
            batch_rename(vec![vec!["a".into(), "b".into(), "c".into()]]).is_err(),
            "三项也应报错"
        );
    }
}