# EP06 双窗口联动：让宠物汇报进度

> 系列第 6 篇 | Demo：`ep06/demo` | 预计阅读 20 分钟

本篇是整个系列的"灵魂一集"：主窗口干活的进度，**实时**出现在
右下角宠物的气泡里——进度条一点点涨满，完成后它蹦起来撒彩带，
失败了它耷拉着脑袋安慰你。

学完本篇你将掌握：

1. 一份前端代码喂饱两个窗口（query 分流）
2. 运行时动态创建窗口（`WebviewWindowBuilder`）
3. Tauri 全局事件总线（`emit` / `listen`）

## 运行本篇 Demo

```bash
cd tutorials/ep06/demo
npm install
npm run tauri dev
```

点「模拟任务（成功）」：宠物气泡出现"开工啦！"→ 进度条涨到 100% →
它开心地蹦两下、撒彩带。再点「失败」那个，看它怎么安慰你。

## 架构：一份代码，两个窗口，事件总线

```
┌─────────────────┐         emit("pet-progress")        ┌─────────────────┐
│   主窗口         │ ──────────────────────────────────► │   宠物窗口       │
│ index.html      │      （Tauri 全局事件，广播到         │ index.html      │
│ → MainWindow.vue│        所有窗口，谁监听谁处理）        │ ?window=pet     │
└─────────────────┘                                      │ → PetWindow.vue │
                                                         └─────────────────┘
```

### 1. query 分流：一份前端代码

两个窗口加载同一个 `index.html`，区别只在 URL。`main.ts` 里一句分流：

```ts
const isPet = new URLSearchParams(location.search).get("window") === "pet";
createApp(isPet ? PetWindow : MainWindow).mount("#app");
```

好处：**宠物窗口自动继承全部的公共代码**——成品的国际化、主题、
字体配置就是这样零成本同步到宠物窗口的。

### 2. 运行时创建宠物窗口

前几集窗口写在 `tauri.conf.json` 里，这一集改为运行时创建——
因为成品里宠物是"可开关"的，窗口要按需生灭：

```rust
let win = WebviewWindowBuilder::new(
    app, "pet",
    WebviewUrl::App("index.html?window=pet".into()), // ← 带上分流 query
)
.decorations(false)
.transparent(true)
.always_on_top(true)
.skip_taskbar(true)
.shadow(false)
.build()?;
```

创建完接着用 EP02 的公式把它摆到右下角——代码一字未改，
这就是把定位写成纯函数的回报。

### 3. 事件协议：只广播事实

发送端（`petProgress.ts`，与成品同名同构）：

```ts
export type PetProgressPhase = "start" | "tick" | "done" | "error";
export interface PetProgressPayload {
  phase: PetProgressPhase;
  progress?: number; // tick 阶段携带 0~100
  name?: string;     // done/error 携带任务名
}

export async function emitPetProgress(payload: PetProgressPayload) {
  try {
    await emit("pet-progress", payload);
  } catch { /* 非 Tauri 环境静默忽略 */ }
}
```

三个设计决策值得记住：

- **四个阶段，不传对象**：`start/tick/done/error` 覆盖所有任务的
  生命周期，宠物不用理解"合并"和"压缩"的区别
- **事件名全局常量化**：收发两端共用 `PET_PROGRESS_EVENT`，
  改名字只改一处
- **try/catch 兜底**：前端也能在纯浏览器里跑（方便调试），
  不会因为 emit 失败崩掉主流程

发送侧在任务节点打卡，以模拟任务为例：

```ts
void emitPetProgress({ phase: "start" });
// 每次进度变化：
void emitPetProgress({ phase: "tick", progress: progress.value });
// 结束：
void emitPetProgress({ phase: "done", name: "模拟合并" });
```

换成真实任务也一样——成品的 `usePanelTask` 就是在这三个位置打的卡，
所以 30+ 个功能面板的进度全都能投给宠物。

### 4. 接收端：一个 listen 撑起全部反应

```ts
unlisten = await listen<PetProgressPayload>(PET_PROGRESS_EVENT, (e) => {
  switch (e.payload.phase) {
    case "start": mood.value = "working"; say("开工啦！"); break;
    case "tick":  progress.value = e.payload.progress ?? 0; break;
    case "done":  mood.value = "happy"; celebrate(); say("搞定！✨"); break;
    case "error": mood.value = "sad"; say("呜…出错了，抱抱你"); break;
  }
});
```

注意 `listen` 返回 `unlisten` 函数，组件卸载时调用，防止重复监听。

宠物用 `mood`（idle/working/happy/sad）驱动三套表情和动画：
开心时弯弯眼 + 双跳 + 彩带粒子，难过时下垂眼 + 微微低头摇晃。
这些在 EP03/EP04 都练过，此处只是换个触发源。

## 为什么用事件而不是共享状态

常见疑问：为什么不把进度放一个全局 store 里？

- 两个窗口是**两个独立的 JS 运行时**，Pinia 之类的状态不跨窗口
- 跨窗口共享状态要自己造同步机制，而 Tauri 的 `emit` 就是为此设计的：
  **广播事实，各窗口自治**
- 宠物窗口关掉时事件自然没人接收，零副作用——
  不用写任何"宠物是否开着"的判断

## 本篇小结

| 知识点 | 一句话 |
|--------|--------|
| query 分流 | 一份前端代码，`?window=pet` 挂不同根组件 |
| 动态窗口 | `WebviewWindowBuilder` 运行时建，成品宠宠开关的基础 |
| 事件总线 | `emit`/`listen` 广播事实，四阶段协议，卸载记得 unlisten |

## 下一篇预告

宠物已经能干活、会汇报了，还差最后一块拼图：一张嘴。
[EP07 AI 篇：宠物会聊天了](../ep07/article.md) ——
给它加上对话框：本地规则引擎打底、可选接入任意 OpenAI 兼容云端，
体验"本地优先"的 AI 设计。
