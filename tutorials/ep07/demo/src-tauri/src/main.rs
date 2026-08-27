//! EP07：AI 助手
//!
//! 教学版把 AI 逻辑放在前端：本地规则引擎零依赖可用，
//! 云端走 fetch 调任意 OpenAI 兼容接口（用户自带 key）。
//!
//! 成品 DocMorph 的做法更进一步：
//! - 本地摘要用 transformers.js（浏览器内跑小模型，真本地推理）
//! - 云端聊天与网页搜索由 Rust 命令转发（统一管理 key 与超时）
//! 架构思想相同：本地优先，云端是增强项，永远可降级。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("启动 Tauri 应用失败");
}
