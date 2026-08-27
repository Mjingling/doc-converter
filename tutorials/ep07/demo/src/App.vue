<template>
  <main class="page">
    <header>
      <h1>AI 助手 <small>EP07 · 本地优先</small></h1>
      <span class="badge" :class="cloudReady(cfg) ? 'on' : 'off'">
        {{ cloudReady(cfg) ? "云端模式" : "本地模式" }}
      </span>
    </header>

    <!-- 消息列表 -->
    <div class="chat" ref="chatRef">
      <div v-for="(m, i) in messages" :key="i" class="msg" :class="m.role">
        <div class="bubble">{{ m.content }}</div>
      </div>
      <div v-if="thinking" class="msg assistant">
        <div class="bubble dim">正在思考…</div>
      </div>
    </div>

    <!-- 输入区 -->
    <div class="input-row">
      <input
        v-model="draft"
        placeholder="说点什么…（试试「帮助」「摘要：一段话」）"
        :disabled="thinking"
        @keyup.enter="send"
      />
      <button :disabled="thinking || !draft.trim()" @click="send">发送</button>
    </div>

    <!-- 云端配置 -->
    <details class="config">
      <summary>云端接口配置（可选，不填则纯本地）</summary>
      <input v-model="cfg.baseUrl" placeholder="baseUrl，如 https://open.bigmodel.cn/api/paas/v4" />
      <input v-model="cfg.apiKey" type="password" placeholder="apiKey" />
      <input v-model="cfg.model" placeholder="model，如 glm-4-flash" />
      <p class="hint">任何 OpenAI 兼容接口都行（智谱 / DeepSeek / Ollama…）。key 只存在本地内存，不上传。</p>
    </details>
  </main>
</template>

<script setup lang="ts">
import { nextTick, reactive, ref } from "vue";
import {
  cloudChat,
  cloudReady,
  localReply,
  type ChatMessage,
  type CloudConfig,
} from "./aiEngine";

const messages = ref<ChatMessage[]>([
  { role: "assistant", content: "你好！我是兔小胖（教学版）。说「帮助」看看我会什么～" },
]);
const draft = ref("");
const thinking = ref(false);
const chatRef = ref<HTMLElement | null>(null);
const cfg = reactive<CloudConfig>({ baseUrl: "", apiKey: "", model: "" });

function scrollToBottom() {
  void nextTick(() => {
    chatRef.value?.scrollTo({ top: chatRef.value.scrollHeight, behavior: "smooth" });
  });
}

/**
 * 路由策略：配置了云端就优先云端，任何失败都降级本地 ——
 * 用户永远能拿到回复，这是"本地优先"的底线。
 */
async function send() {
  const text = draft.value.trim();
  if (!text || thinking.value) return;
  draft.value = "";
  messages.value.push({ role: "user", content: text });
  scrollToBottom();

  thinking.value = true;
  let reply = "";
  let source = "";
  try {
    if (cloudReady(cfg)) {
      try {
        reply = await cloudChat(cfg, messages.value);
        source = "云端";
      } catch (e) {
        // 云端失败 → 自动降级
        reply = `${localReply(text)}\n\n（云端不可用已降级本地：${e}）`;
        source = "本地（降级）";
      }
    } else {
      // 模拟一点"思考"延迟，让本地模式也有节奏感
      await new Promise((r) => setTimeout(r, 400));
      reply = localReply(text);
      source = "本地";
    }
  } finally {
    thinking.value = false;
  }
  messages.value.push({ role: "assistant", content: reply });
  console.log(`[AI] ${source}`);
  scrollToBottom();
}
</script>

<style scoped>
.page {
  max-width: 640px;
  margin: 0 auto;
  padding: 24px;
  display: flex;
  flex-direction: column;
  height: 100vh;
  box-sizing: border-box;
  font-family: -apple-system, "PingFang SC", "Microsoft YaHei", sans-serif;
  color: #1a1a1a;
}
header { display: flex; align-items: center; gap: 10px; }
h1 { font-size: 18px; margin: 0; }
h1 small { font-size: 12px; color: #8a8f98; font-weight: 400; }
.badge {
  font-size: 11px;
  padding: 3px 10px;
  border-radius: 20px;
}
.badge.on { background: #e8f5e9; color: #2e7d32; }
.badge.off { background: #f0f2f5; color: #8a8f98; }

.chat {
  flex: 1;
  overflow-y: auto;
  margin: 16px 0;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.msg.user { align-self: flex-end; }
.msg.assistant { align-self: flex-start; }
.bubble {
  max-width: 78%;
  padding: 9px 13px;
  border-radius: 12px;
  font-size: 13px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-word;
}
.msg.user .bubble { background: #2080f0; color: #fff; border-bottom-right-radius: 4px; }
.msg.assistant .bubble { background: #f0f2f5; border-bottom-left-radius: 4px; }
.bubble.dim { color: #8a8f98; }

.input-row { display: flex; gap: 8px; }
.input-row input {
  flex: 1;
  padding: 10px 12px;
  border: 1px solid #d5d8dc;
  border-radius: 10px;
  font-size: 13px;
}
.input-row button {
  padding: 10px 20px;
  border: none;
  border-radius: 10px;
  background: #2080f0;
  color: #fff;
  font-size: 13px;
  cursor: pointer;
}
.input-row button:disabled { opacity: 0.5; cursor: not-allowed; }

.config {
  margin-top: 12px;
  font-size: 12px;
  color: #555;
}
.config summary { cursor: pointer; user-select: none; }
.config input {
  display: block;
  width: 100%;
  margin-top: 8px;
  padding: 8px 10px;
  border: 1px solid #d5d8dc;
  border-radius: 8px;
  font-size: 12px;
  box-sizing: border-box;
}
.hint { color: #8a8f98; margin: 8px 0 0; }
</style>
