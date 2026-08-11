use std::time::Duration;

/// 拉取远程版本元数据 JSON（检查更新用）。
/// 走 Rust 侧 HTTP 请求：Gitee raw 会 302 重定向到 CDN 且不带 CORS 头，
/// WebView 的 fetch 会被同源策略拦截，故由后端代拉。
#[tauri::command]
pub fn fetch_update_json(url: String) -> Result<String, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(5))
        .timeout_read(Duration::from_secs(8))
        .build();
    let resp = agent
        .get(&url)
        .call()
        .map_err(|e| e.to_string())?;
    resp.into_string().map_err(|e| e.to_string())
}
