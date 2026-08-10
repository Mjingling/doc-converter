//! AI 云端命令：通过 ureq 转发 OpenAI 兼容 API 的 chat 和 embedding 请求
//! 支持 OpenAI、DeepSeek、通义千问、Ollama 等兼容服务

use serde::{Deserialize, Serialize};

/// 聊天消息（OpenAI 兼容格式）
#[derive(Deserialize, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// 云端 chat 补全（非流式）；返回 AI 回复文本
/// 异步 + spawn_blocking 避免阻塞主线程；配置显式读写超时防止 UI 永久冻结
#[tauri::command]
pub async fn ai_cloud_chat(
    messages: Vec<ChatMessage>,
    model: String,
    base_url: String,
    api_key: String,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
        let body = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": false,
        });

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

        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| format!("API 响应格式异常: {:?}", json))?
            .to_string();

        Ok(content)
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
        let body = serde_json::json!({
            "model": model,
            "input": texts,
        });

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