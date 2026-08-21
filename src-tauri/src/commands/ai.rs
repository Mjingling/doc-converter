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