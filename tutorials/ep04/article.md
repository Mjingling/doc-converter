# EP04 交互篇：摸头、戳一戳与右键菜单

> 系列第 4 篇 | Demo：`ep04/demo` | 预计阅读 18 分钟

> 💡 **AI 协作贴士**：交互反馈的细节最容易漏——本篇开发时 AI 写了「戳一戳可能冒爱心」
> 的分支，却忘了真的冒爱心，是人审代码时抓出来的。
> 结论：交互类代码写完必须**人肉逐一点一遍**，别信 AI 的「已完成」。

本篇目标：宠物从"自己动"升级到"能互动"——

- 鼠标悬停：摸头冒爱心 ♥
- 单击：戳一戳，随机跳一下 / 扭一扭 / 冒爱心，配台词气泡
- 右键：弹出自定义菜单（再戳一下 / 隐藏宠物）
- 按住拖动：把它拖到屏幕任何位置

## 运行本篇 Demo

```bash
cd tutorials/ep04/demo
npm install
npm run tauri dev
```

## 交互一：戳一戳（单击）

决策照旧放纯函数里（`petBehavior.ts` 新增）：

```ts
export type PokeReaction = "hop" | "wiggle" | "hearts";

export function pickPokeReaction(rand: number): PokeReaction {
  if (rand < 0.4) return "hop";     // 40% 跳一下
  if (rand < 0.7) return "wiggle";  // 30% 扭一扭
  return "hearts";                  // 30% 冒爱心
}

export const POKE_LINES = ["哎呀，干嘛呀~", "唔！", "嘿嘿，别戳啦", "我在认真工作呢！"] as const;
```

组件里的 `poke()` 一次完成四件事：**打断当前行为 → 播放反应动画 →
冒台词气泡 → 安排自动结束**：

```ts
function poke() {
  behavior.value = "idle"; // 戳醒打盹中的宠物
  reaction.value = pickPokeReaction(Math.random());
  if (reaction.value === "hearts") {
    lastHeartAt = 0;       // 临时解除爱心节流
    spawnHeart();
    window.setTimeout(spawnHeart, 150);
    window.setTimeout(spawnHeart, 300);
  }
  bubble.value = pickPokeLine(Math.random());
  clearTimeout(bubbleTimer);
  bubbleTimer = window.setTimeout(() => (bubble.value = ""), 2200);
  clearTimeout(reactionTimer);
  reactionTimer = window.setTimeout(() => (reaction.value = null), 700);
}
```

注意两个 `clearTimeout`：连点两下时，气泡和动画计时器要重置，
否则上一次的定时器会提前掐掉新反应——这是交互动画最常见的 bug。

`wiggle`（扭一扭）是个纯 CSS 动画：

```css
@keyframes wiggle {
  0%, 100% { transform: rotate(0); }
  25% { transform: rotate(-8deg); }
  75% { transform: rotate(8deg); }
}
```

## 交互二：摸头爱心（悬停）

悬停用 `mouseenter` + `mousemove` 双触发，**必须节流**，
否则鼠标抖一下冒出一串爱心，廉价感立刻拉满：

```ts
let lastHeartAt = 0;

function spawnHeart() {
  const now = Date.now();
  if (now - lastHeartAt < 400) return; // 400ms 最多一颗
  lastHeartAt = now;
  const h = { id: heartSeq++, x: 30 + Math.random() * 80, y: 20 + Math.random() * 40 };
  hearts.value.push(h);
  // 动画播完就移除，数组不会无限增长
  window.setTimeout(() => {
    hearts.value = hearts.value.filter((x) => x.id !== h.id);
  }, 1200);
}
```

粒子本身是绝对定位的 `♥` 字符 + 上浮淡出动画，比画 SVG 便宜得多：

```css
@keyframes heart-up {
  0% { transform: translateY(0) scale(0.8); opacity: 0; }
  20% { opacity: 1; }
  100% { transform: translateY(-34px) scale(1.25); opacity: 0; }
}
```

## 交互三：拖动窗口

无边框窗口没有标题栏，宠物本体就是"标题栏"。
Tauri 提供了系统级拖拽，一行调用：

```ts
import { getCurrentWindow } from "@tauri-apps/api/window";

async function startDrag() {
  await getCurrentWindow().startDragging();
}
```

两个细节：

1. **权限**：`startDragging` 要在 `capabilities/default.json` 里显式授权，
   本集新增了 `core:window:allow-start-dragging`
2. **事件绑定在容器上，SVG 设 `pointer-events: none`**——
   让所有鼠标事件统一由 `.pet` 容器接收，避免点击和拖拽在
   父子元素间打架：

```css
.robot { pointer-events: none; }
```

一旦进入系统拖拽，本次按下不会再派发 `click`，
所以"拖一下误触发戳一戳"的情况天然不会发生。

## 交互四：右键菜单

透明小窗口用不了系统菜单的样式，直接自己画：

```html
<ul v-if="menu.open" class="menu" :style="{ left: menu.x + 'px', top: menu.y + 'px' }">
  <li @click="pokeFromMenu">再戳一下 👉</li>
  <li @click="hidePet">隐藏宠物 🙈</li>
  <li class="menu-footer">桌宠教程 EP04</li>
</ul>
```

```ts
function openMenu(e: MouseEvent) {
  // 窗口只有 150px 宽，菜单位置要夹住，避免溢出屏幕外
  menu.x = Math.min(e.clientX, 40);
  menu.y = Math.min(e.clientY, 90);
  menu.open = true;
}
```

要点：

- `@contextmenu.prevent` 阻止浏览器默认右键菜单
- 菜单项点击后自己关闭；点击窗口其他区域也要关
  （全局 `click` 监听 + `@click.stop` 防止自触发）
- "隐藏宠物"调用 `getCurrentWindow().hide()`——
  窗口消失但进程还在。成品里对应 `pet_hide` 命令 + 设置开关，
  随时能 `show()` 回来

> 成品 DocMorph 的右键菜单还挂了 4 个快捷功能（压缩/合并/图片转PDF/格式转换），
> 点击后发事件给主窗口切面板——这个联动正是下一集之后的重头戏。

## 本篇小结

| 交互 | 实现 | 坑位 |
|------|------|------|
| 戳一戳 | 单击 → 纯函数抽反应 → 动画+气泡 | 连点要重置计时器 |
| 摸头爱心 | 悬停粒子，400ms 节流 | 粒子用完要删，别泄漏 |
| 拖动 | `startDragging()` + capability 授权 | 事件绑容器，SVG 穿透 |
| 右键菜单 | 自定义 HTML + 位置夹取 | 点击外部关闭 |

## 下一篇预告

宠物线告一段落，它现在可爱但"没用"。
[EP05 干正事：Rust 文档处理引擎](../ep05/article.md) ——
切到引擎线，用 Rust + lopdf 实现 PDF 合并与压缩，
学会 `#[tauri::command]` 和 `invoke`，让桌宠真正能帮媳妇干活。
