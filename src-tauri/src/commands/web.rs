//! 网页转 PDF 命令
use crate::engine::light;
use std::path::Path;

/// 将网页（URL）转换为 PDF：先拉取 HTML 内容，再通过内置引擎渲染为 PDF
/// 异步 + spawn_blocking 避免阻塞主线程；临时文件使用唯一名避免并发踩踏
#[tauri::command]
pub async fn webpage_to_pdf(url: String, out_path: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        // 拉取 HTML 内容（配置显式超时）
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(std::time::Duration::from_secs(10))
            .timeout_read(std::time::Duration::from_secs(30))
            .build();

        let response = agent
            .get(&url)
            .call()
            .map_err(|e| format!("访问网页失败: {}", e))?;

        let html = response
            .into_string()
            .map_err(|e| format!("读取网页内容失败: {}", e))?;

        // 使用进程唯一名写入临时文件，避免并发调用互相踩踏
        let tmp_dir = std::env::temp_dir().join("docmorph-web");
        std::fs::create_dir_all(&tmp_dir)
            .map_err(|e| format!("创建临时目录失败: {}", e))?;
        let pid = std::process::id();
        let tmp_html = tmp_dir.join(format!("webpage_{}.html", pid));
        std::fs::write(&tmp_html, &html)
            .map_err(|e| format!("写入临时文件失败: {}", e))?;

        // 使用内置引擎的 HTML→PDF 转换
        if let Some(parent) = Path::new(&out_path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("创建输出目录失败: {}", e))?;
            }
        }

        light::html_to_pdf(&tmp_html, Path::new(&out_path))?;

        // 清理临时文件
        let _ = std::fs::remove_file(&tmp_html);

        Ok(out_path)
    })
    .await
    .map_err(|e| format!("网页转 PDF 内部错误: {}", e))?
}