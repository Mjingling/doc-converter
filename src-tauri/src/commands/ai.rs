//! AI 云端命令：通过 ureq 转发 OpenAI 兼容 API 的 chat 和 embedding 请求
//! 支持 OpenAI、DeepSeek、通义千问、智谱 BigModel、Ollama 等兼容服务

use serde::{Deserialize, Serialize};

/// 聊天消息（OpenAI 兼容格式；content 在 assistant 携带工具调用时可为 null）
#[derive(Deserialize, Serialize)]
pub struct ChatMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// assistant 消息携带的工具调用（function calling 多轮回传）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<serde_json::Value>>,
    /// 工具执行结果消息（role=tool）对应的调用 id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// 模型返回的工具调用（响应解析用）
#[derive(Serialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    /// JSON 字符串形式的函数参数
    pub arguments: String,
}

/// 云端 chat 补全响应：content 与 tool_calls 至少存在一个
#[derive(Serialize)]
pub struct ChatReply {
    pub content: Option<String>,
    pub tool_calls: Vec<ToolCall>,
}

/// 云端 chat 补全（非流式，支持 function calling）；返回 content 与 tool_calls
/// 异步 + spawn_blocking 避免阻塞主线程；配置显式读写超时防止 UI 永久冻结
#[tauri::command]
pub async fn ai_cloud_chat(
    messages: Vec<ChatMessage>,
    model: String,
    base_url: String,
    api_key: String,
    tools: Option<Vec<serde_json::Value>>,
) -> Result<ChatReply, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
        let mut body = serde_json::json!({
            "messages": messages,
            "stream": false,
        });
        if !model.is_empty() {
            body["model"] = serde_json::Value::String(model);
        }
        if let Some(tools) = tools {
            body["tools"] = serde_json::Value::Array(tools);
        }

        let agent = ureq::AgentBuilder::new()
            .timeout_connect(std::time::Duration::from_secs(10))
            .timeout_read(std::time::Duration::from_secs(60))
            .build();

        let resp = agent
            .post(&url)
            .set("Authorization", &format!("Bearer {}", api_key))
            .set("Content-Type", "application/json")
            .send_json(&body)
            .map_err(|e| format!("云端 API 请求失败: {}", e))?;

        let json: serde_json::Value = resp
            .into_json()
            .map_err(|e| format!("解析响应失败: {}", e))?;

        let msg = &json["choices"][0]["message"];
        let content = msg["content"].as_str().map(|s| s.to_string());
        let mut tool_calls = Vec::new();
        if let Some(arr) = msg["tool_calls"].as_array() {
            for tc in arr {
                tool_calls.push(ToolCall {
                    id: tc["id"].as_str().unwrap_or("").to_string(),
                    name: tc["function"]["name"].as_str().unwrap_or("").to_string(),
                    arguments: tc["function"]["arguments"].as_str().unwrap_or("{}").to_string(),
                });
            }
        }
        if content.is_none() && tool_calls.is_empty() {
            return Err(format!("API 响应格式异常: {:?}", json));
        }

        Ok(ChatReply { content, tool_calls })
    })
    .await
    .map_err(|e| format!("云端 chat 内部错误: {}", e))?
}

/// 云端 embedding（文本 → 向量）；返回向量数组，与输入 texts 顺序一致
/// 异步 + spawn_blocking 避免阻塞主线程
#[tauri::command]
pub async fn ai_cloud_embed(
    texts: Vec<String>,
    model: String,
    base_url: String,
    api_key: String,
) -> Result<Vec<Vec<f32>>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let url = format!("{}/embeddings", base_url.trim_end_matches('/'));
        let mut body = serde_json::json!({
            "input": texts,
        });
        if !model.is_empty() {
            body["model"] = serde_json::Value::String(model);
        }

        let agent = ureq::AgentBuilder::new()
            .timeout_connect(std::time::Duration::from_secs(10))
            .timeout_read(std::time::Duration::from_secs(60))
            .build();

        let resp = agent
            .post(&url)
            .set("Authorization", &format!("Bearer {}", api_key))
            .set("Content-Type", "application/json")
            .send_json(&body)
            .map_err(|e| format!("云端 API 请求失败: {}", e))?;

        let json: serde_json::Value = resp
            .into_json()
            .map_err(|e| format!("解析响应失败: {}", e))?;

        let data = json["data"]
            .as_array()
            .ok_or_else(|| format!("API 响应缺少 data 字段: {:?}", json))?;

        let mut results = vec![vec![0.0f32; 0]; texts.len()];
        for item in data {
            let idx = item["index"].as_u64().unwrap_or(0) as usize;
            let emb: Vec<f32> = item["embedding"]
                .as_array()
                .ok_or_else(|| "embedding 字段异常".to_string())?
                .iter()
                .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                .collect();
            if idx < results.len() {
                results[idx] = emb;
            }
        }

        Ok(results)
    })
    .await
    .map_err(|e| format!("云端 embedding 内部错误: {}", e))?
}
/* ---------- 网页搜索（AI 助手 web_search 工具后端） ---------- */

/// 搜索结果条目（统一两提供商的字段名）
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WebSearchResult {
    pub title: String,
    pub link: String,
    pub snippet: String,
}

/// 解析智谱 web-search 响应：{"search_result": [{title, link, content}, ...]}
/// 字段缺失的条目跳过；解析不出任何条目返回 None（由调用方报"无结果"）
fn parse_zhipu_results(json: &serde_json::Value) -> Option<Vec<WebSearchResult>> {
    let arr = json["search_result"].as_array()?;
    let items = arr
        .iter()
        .filter_map(|it| {
            Some(WebSearchResult {
                title: it["title"].as_str()?.to_string(),
                link: it["link"].as_str()?.to_string(),
                snippet: it["content"].as_str().unwrap_or_default().to_string(),
            })
        })
        .collect::<Vec<_>>();
    if items.is_empty() { None } else { Some(items) }
}

/// 解析 Tavily 响应：{"results": [{title, url, content}, ...]}
fn parse_tavily_results(json: &serde_json::Value) -> Option<Vec<WebSearchResult>> {
    let arr = json["results"].as_array()?;
    let items = arr
        .iter()
        .filter_map(|it| {
            Some(WebSearchResult {
                title: it["title"].as_str()?.to_string(),
                link: it["url"].as_str()?.to_string(),
                snippet: it["content"].as_str().unwrap_or_default().to_string(),
            })
        })
        .collect::<Vec<_>>();
    if items.is_empty() { None } else { Some(items) }
}

/* ---------- 云端连接分阶段诊断：DNS → TCP → HTTP/TLS，一次运行定位失败环节 ---------- */

/// 单个地址的 TCP 连接探测结果
#[derive(Serialize)]
pub struct TcpProbe {
    pub addr: String,
    pub ok: bool,
    pub ms: u128,
    pub error: Option<String>,
}

/// 云端连接诊断结果：DNS 解析 → 逐 IP TCP 连通 → HTTP 整链路（含 TLS 握手）
#[derive(Serialize)]
pub struct CloudDiag {
    /// DNS 解析出的地址列表（host:port；代理软件 fake-ip 模式会在此暴露为 198.18.x.x）
    pub dns_addrs: Vec<String>,
    pub dns_ms: u128,
    /// 逐地址 TCP 探测（连接超时直指代理分流/防火墙拦截）
    pub tcp: Vec<TcpProbe>,
    /// HTTP 探测状态码：收到任意状态码（含 401/404/405）即整链路通
    pub http_status: Option<u16>,
    pub http_ms: Option<u128>,
    pub http_error: Option<String>,
}

/// 从 base_url 提取 (host, port)：支持 http/https 与显式端口
fn split_host_port(base_url: &str) -> Result<(String, u16), String> {
    let trimmed = base_url.trim();
    let rest = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .unwrap_or(trimmed);
    let authority = rest.split('/').next().unwrap_or("");
    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) => (h.to_string(), p.parse::<u16>().map_err(|_| format!("端口非法: {p}"))? ),
        None => (
            authority.to_string(),
            if trimmed.starts_with("http://") { 80 } else { 443 },
        ),
    };
    if host.is_empty() {
        return Err(format!("无法从 URL 提取主机名: {base_url}"));
    }
    Ok((host, port))
}

/// 云端连接诊断：测试连接超时时前端自动调用，把各阶段结果拼进错误提示。
/// 纯网络探测（GET，无需 API key），不触发真实推理。
#[tauri::command]
pub async fn ai_cloud_diag(base_url: String) -> Result<CloudDiag, String> {
    tauri::async_runtime::spawn_blocking(move || {
        use std::net::ToSocketAddrs;

        let (host, port) = split_host_port(&base_url)?;
        let mut diag = CloudDiag {
            dns_addrs: vec![],
            dns_ms: 0,
            tcp: vec![],
            http_status: None,
            http_ms: None,
            http_error: None,
        };

        // 1) DNS（系统解析器：能反映代理软件接管后的 fake-ip / DNS 污染）
        let t0 = std::time::Instant::now();
        let addrs: Vec<std::net::SocketAddr> = match (host.as_str(), port).to_socket_addrs() {
            Ok(it) => it.collect(),
            Err(e) => {
                diag.http_error = Some(format!("DNS 解析失败: {e}"));
                return Ok(diag);
            }
        };
        diag.dns_ms = t0.elapsed().as_millis();
        diag.dns_addrs = addrs.iter().map(|a| a.to_string()).collect();
        if addrs.is_empty() {
            diag.http_error = Some("DNS 解析结果为空".to_string());
            return Ok(diag);
        }

        // 2) 逐地址 TCP（3s 超时；连接成功但后续 HTTP 超时多为代理中间人/丢包）
        for addr in &addrs {
            let t = std::time::Instant::now();
            let (ok, error) =
                match std::net::TcpStream::connect_timeout(addr, std::time::Duration::from_secs(3)) {
                    Ok(_) => (true, None),
                    Err(e) => (false, Some(e.to_string())),
                };
            diag.tcp.push(TcpProbe {
                addr: addr.to_string(),
                ok,
                ms: t.elapsed().as_millis(),
                error,
            });
        }

        // 3) HTTP 整链路（含 TLS 握手）：GET base_url，收到任意状态码即通
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(std::time::Duration::from_secs(5))
            .timeout_read(std::time::Duration::from_secs(10))
            .build();
        let t = std::time::Instant::now();
        match agent.get(base_url.trim_end_matches('/')).call() {
            Ok(_) => diag.http_status = Some(200),
            Err(ureq::Error::Status(code, _)) => diag.http_status = Some(code),
            Err(ureq::Error::Transport(te)) => diag.http_error = Some(te.to_string()),
        }
        diag.http_ms = Some(t.elapsed().as_millis());

        Ok(diag)
    })
    .await
    .map_err(|e| format!("诊断内部错误: {e}"))?
}

/// 网页搜索：provider = "zhipu"（复用云端 API 的 baseUrl 与密钥）或 "tavily"（body 传 key）
#[tauri::command]
pub async fn web_search(
    provider: String,
    api_key: String,
    base_url: String,
    query: String,
    max_results: Option<u32>,
) -> Result<Vec<WebSearchResult>, String> {
    let count = max_results.unwrap_or(8).clamp(1, 10);
    tauri::async_runtime::spawn_blocking(move || {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(std::time::Duration::from_secs(10))
            .timeout_read(std::time::Duration::from_secs(20))
            .build();

        let (url, body, use_bearer) = if provider == "zhipu" {
            // 智谱 web-search：与 chat 同 baseUrl（.../api/paas/v4），Bearer 鉴权
            let url = format!("{}/web-search", base_url.trim_end_matches('/'));
            let body = serde_json::json!({
                "search_engine": "search_std",
                "search_query": query,
                "count": count,
            });
            (url, body, true)
        } else if provider == "tavily" {
            // Tavily：key 放 body 字段
            let body = serde_json::json!({
                "api_key": api_key,
                "query": query,
                "max_results": count,
            });
            ("https://api.tavily.com/search".to_string(), body, false)
        } else {
            return Err(format!("不支持的搜索提供商: {provider}"));
        };

        let mut req = agent
            .post(&url)
            .set("Content-Type", "application/json");
        if use_bearer {
            req = req.set("Authorization", &format!("Bearer {}", api_key));
        }
        let resp = req
            .send_json(&body)
            .map_err(|e| format!("搜索请求失败: {}", e))?;
        let json: serde_json::Value = resp
            .into_json()
            .map_err(|e| format!("解析搜索响应失败: {}", e))?;

        let parsed = if provider == "zhipu" {
            parse_zhipu_results(&json)
        } else {
            parse_tavily_results(&json)
        };
        parsed.ok_or_else(|| "搜索无结果，请换个关键词重试".to_string())
    })
    .await
    .map_err(|e| format!("网页搜索内部错误: {}", e))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_zhipu_results() {
        let json = serde_json::json!({
            "search_result": [
                {"title": "结果一", "link": "https://a.com", "content": "摘要甲", "icon": "x"},
                {"title": "结果二", "link": "https://b.com"}
            ]
        });
        let items = parse_zhipu_results(&json).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0], WebSearchResult {
            title: "结果一".into(), link: "https://a.com".into(), snippet: "摘要甲".into()
        });
        // content 缺失时 snippet 为空字符串而非跳过条目
        assert_eq!(items[1].snippet, "");
    }

    #[test]
    fn test_parse_zhipu_results_missing_fields_skipped() {
        // link 缺失的条目跳过；全部缺失 → None
        let json = serde_json::json!({"search_result": [{"title": "无链接"}]});
        assert!(parse_zhipu_results(&json).is_none());
        assert!(parse_zhipu_results(&serde_json::json!({})).is_none());
    }

    #[test]
    fn test_parse_tavily_results() {
        let json = serde_json::json!({
            "results": [
                {"title": "T1", "url": "https://t.com", "content": "C1", "score": 0.9}
            ]
        });
        let items = parse_tavily_results(&json).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].link, "https://t.com");
        assert_eq!(items[0].snippet, "C1");
    }

    #[test]
    fn test_split_host_port() {
        // 常见形态：https 带路径 / http 显式端口 / 尾斜杠 / 非法输入
        assert_eq!(split_host_port("https://open.bigmodel.cn/api/paas/v4").unwrap(), ("open.bigmodel.cn".into(), 443));
        assert_eq!(split_host_port("http://localhost:11434").unwrap(), ("localhost".into(), 11434));
        assert_eq!(split_host_port("https://api.x.com/").unwrap(), ("api.x.com".into(), 443));
        assert!(split_host_port("https://").is_err());
        assert!(split_host_port("https://host:notaport").is_err());
    }
}
