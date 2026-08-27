# EP07 AI 篇：宠物会聊天了

> 系列第 7 篇 | Demo：`ep07/demo` | 预计阅读 15 分钟

> 💡 **AI 协作贴士**：本篇是「用 AI 做 AI」。对接云端接口时，
> 「本地优先 + 云端可选 + 失败降级」的**策略必须人定**，
> AI 只负责把每一层的代码写出来——否则它容易只写云端一条路。

本篇目标：给应用装上一张会说话的嘴——一个聊天助手，
**不配置任何东西就能用（本地规则引擎）**，配置了云端 key 就升级到真 AI，
云端挂了还能自动降级。这就是贯穿成品的"本地优先"策略。

## 运行本篇 Demo

```bash
cd tutorials/ep07/demo
npm install
npm run tauri dev
```

直接发消息体验本地模式（试试「帮助」「摘要：一段很长的话」）。
然后展开底部「云端接口配置」，填入任意 OpenAI 兼容接口：

| 提供商 | baseUrl | model 示例 |
|--------|---------|-----------|
| 智谱 | `https://open.bigmodel.cn/api/paas/v4` | `glm-4-flash` |
| DeepSeek | `https://api.deepseek.com/v1` | `deepseek-chat` |
| Ollama（本地） | `http://localhost:11434/v1` | `qwen2.5:7b` |

key 只保存在内存里，关窗即焚——教程版刻意不持久化，成品的持久化
用的是 `tauri-plugin-store`（系统钥匙串级存储）。

## 架构：三级火箭

```
用户消息
   │
   ├─ 配置了云端？──是──► cloudChat（fetch /chat/completions）
   │                        │ 失败
   │                        ▼
   └─────────否──────► localReply（规则引擎，永不失败）
```

### 1. 本地引擎：规则也是引擎

教学版用正则规则演示流程——打招呼、帮助、朴素摘要：

```ts
export function localReply(text: string): string {
  const t = text.trim();
  if (/^(你好|hi|hello|在吗)/i.test(t)) {
    return "你好呀！我是本地模式的小助手，断网也能陪你～";
  }
  if (t.startsWith("摘要：")) {
    const body = t.slice(3).trim();
    const head = body.slice(0, 50);
    return `【本地摘要】${head}…（原文 ${body.length} 字，本地处理未上传）`;
  }
  return `（本地模式·回声）你说："${t}"`;
}
```

效果很笨，但架构位置是对的：**本地层永远存在、永不失败**。

> 成品的本地层是真推理：用 `@huggingface/transformers` 在 WebView 里
> 直接跑量化小模型做摘要、嵌入和重排——文件不出硬盘，还能离线。
> 教学版省略模型下载（几百 MB），思路一致。

### 2. 云端：一个 fetch 吃遍所有兼容接口

OpenAI 的 `/chat/completions` 已是事实标准，一个函数通吃：

```ts
export async function cloudChat(cfg: CloudConfig, history: ChatMessage[]) {
  const resp = await fetch(`${base}/chat/completions`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${cfg.apiKey}`,
    },
    body: JSON.stringify({ model: cfg.model, messages: [systemPrompt, ...history] }),
  });
  if (!resp.ok) throw new Error(`云端请求失败：HTTP ${resp.status}`);
  const data = await resp.json();
  return data.choices[0].message.content;
}
```

注意三点：

- **system prompt** 放在最前面，定义助手的角色边界
- **带上对话历史**（`...history`），才有上下文
- **非 200 必须抛错**——不然降级逻辑接不住

### 3. 路由与降级：永远有答案

```ts
if (cloudReady(cfg)) {
  try {
    reply = await cloudChat(cfg, messages.value);
  } catch (e) {
    reply = `${localReply(text)}\n\n（云端不可用已降级本地：${e}）`;
  }
} else {
  reply = localReply(text);
}
```

用户视角永远有回复，只是质量分层——
**这是本地优先策略的底线：增强可以失败，基础不能失败**。

## 成品的进化方向（导读）

教学版为了轻装把 AI 放在前端；成品 DocMorph 更进一步：

1. **RAG 问答**：文档先分块 → 嵌入 → 向量检索，回答时把相关块塞进
   prompt，让 AI"读过"你的文件再回答
2. **工具调用（function calling）**：AI 自己决定调哪个文档工具，
   比如"把这份合同压缩到 2MB 以内"直接触发压缩命令
3. **实时网页搜索**：智谱网页搜索 / Tavily 二选一，回答带引用
4. **Rust 转发云端请求**：统一管理 key、超时与重试，前端不碰密钥

这些在成品的 `src/ai/` 目录，每一块都建立在本篇的三级火箭骨架上。

## 本篇小结

| 知识点 | 一句话 |
|--------|--------|
| 本地优先 | 本地层永不失败，云端是增强项 |
| 兼容接口 | 一个 `/chat/completions` fetch 通吃主流提供商 |
| 降级策略 | 云端异常自动回本地，用户永远有答案 |

## 下一篇预告

最后一集！[EP08 收尾：多语言、主题与打包送礼](../ep08/article.md) ——
给应用做最后的梳妆：vue-i18n 多语言、明暗主题，
然后打包成安装包，真正送到媳妇手上。
