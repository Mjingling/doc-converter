# EP03 让宠物活起来：状态机与空闲行为

> 系列第 3 篇 | Demo：`ep03/demo` | 预计阅读 15 分钟

> 💡 **AI 协作贴士**：给 AI 描述行为引擎时，用「状态枚举 + 转移规则 +
> 纯函数输出」的结构化说法，比「让它可爱一点」效果好十倍。
> 决策写成纯函数还有个红利：可以直接让 AI 顺手写单测。

本篇目标：EP02 的静态机器人开始"自己动"——随机眨眼、四处张望、偶尔蹦一下、
困了还会打盹冒 Zzz。核心是一个**行为引擎：纯函数决策 + 组件渲染**。

## 运行本篇 Demo

```bash
cd tutorials/ep03/demo
npm install
npm run tauri dev
```

盯着它看 30 秒：会眨眼、会左右张望、可能蹦一下；再过一会儿会闭眼打盹，
头顶飘出 Zzz，9 秒后自己醒。

## 设计：把"灵魂"写成纯函数

新手常见写法是把随机逻辑散落在组件里，结果：没法测、没法调权重、
以后接"AI 状态同步"时全得重写。

本系列的做法（也是成品 DocMorph 的做法）——
**决策层是纯函数，渲染层只管画**：

```
petBehavior.ts（纯函数，不知道 Vue 的存在）
   ├─ pickBehavior(rand)      抽一个行为
   ├─ nextBehaviorDelay(rand) 决定歇多久
   └─ behaviorDuration(b)     行为持续多久
        ▲
        │ 调用时传 Math.random()，测试时传固定值
App.vue（只管渲染 + 定时器）
```

所有随机都以参数传入，这是可测试性的关键：

```ts
export type PetBehavior = "idle" | "lookAround" | "hop" | "doze";

export function pickBehavior(rand: number): PetBehavior {
  if (rand < 0.3) return "lookAround"; // 30%
  if (rand < 0.5) return "hop";        // 20%
  if (rand < 0.65) return "doze";      // 15%
  return "idle";                       // 35%
}

export function nextBehaviorDelay(rand: number): number {
  return 4000 + Math.floor(rand * 6000); // 4~10 秒
}
```

> 成品的 `src/utils/petBehavior.ts` 就是这个结构，后来接任务进度、
> AI 状态时只加了 `resolveDisplayState` 做状态仲裁，一行没动过调度逻辑。

## 调度循环：setTimeout 链

行为引擎的驱动用 `setTimeout` 链而不是 `setInterval`——
因为每次间隔都不一样（随机的），链式写法天然支持：

```ts
function scheduleNext() {
  behaviorTimer = window.setTimeout(() => {
    const next = pickBehavior(Math.random());
    if (next !== "idle") {
      behavior.value = next;
      if (next === "lookAround") lookDir.value = Math.random() < 0.5 ? -1 : 1;
      // 行为到期自动回 idle，再排下一轮
      durationTimer = window.setTimeout(() => {
        behavior.value = "idle";
        scheduleNext();
      }, behaviorDuration(next));
      return;
    }
    scheduleNext();
  }, nextBehaviorDelay(Math.random()));
}
```

状态只有 4 个：`idle / lookAround / hop / doze`，
组件里用 `behavior.value` 一处切换、处处响应。

## 眨眼：一条独立的小循环

眨眼不属于"行为"，它是生理节律——清醒时永远在发生，
所以单独一个调度器，并且打盹时跳过：

```ts
function scheduleBlink() {
  blinkTimer = window.setTimeout(() => {
    if (behavior.value !== "doze") {
      blinking.value = true;
      window.setTimeout(() => (blinking.value = false), 150);
    }
    scheduleBlink();
  }, 2500 + Math.random() * 2500);
}
```

渲染上，闭眼/睁眼是两套画法，`eyesClosed` 计算属性统一裁决：

```ts
const eyesClosed = computed(() => blinking.value || behavior.value === "doze");
```

```html
<template v-if="eyesClosed">
  <!-- 闭眼：两条弯弯的弧线 -->
  <path d="M 43 53 Q 48 57 53 53" ... />
  <path d="M 67 53 Q 72 57 77 53" ... />
</template>
<template v-else>
  <!-- 睁眼：椭圆，张望时整体偏移 eyeOffset -->
  <ellipse :cx="48 + eyeOffset" cy="52" rx="4.5" ry="8" fill="#1a1a1a" />
  <ellipse :cx="72 + eyeOffset" cy="52" rx="4.5" ry="8" fill="#1a1a1a" />
</template>
```

"张望"不画新眼睛，只是给瞳孔加个 ±4px 的偏移——
**用已有元素的属性表达状态，是 SVG 桌宠省力的秘诀**。

## 打盹：全身一起演

打盹不是换张图，是一组细节同时变化：

- 眼睛换成闭眼弧线，嘴变成一字
- 天线指示灯从蓝色 `#2080f0` 变灰 `#9aa0a6`（细节加分项）
- 头顶出现 `Zzz` 气泡，缓慢浮动
- 持续 9 秒（`behaviorDuration("doze")`）后自动醒

```css
.zzz {
  position: absolute;
  top: 18px; right: 18px;
  font-size: 20px; font-weight: 700; color: #2080f0;
  animation: float 2.4s ease-in-out infinite;
}
```

## 别忘了清理定时器

组件卸载时三个 `setTimeout` 都要 `clearTimeout`，否则窗口销毁后
定时器还在跑，引用已卸载的响应式状态：

```ts
onUnmounted(() => {
  clearTimeout(behaviorTimer);
  clearTimeout(durationTimer);
  clearTimeout(blinkTimer);
});
```

## 本篇小结

| 知识点 | 一句话 |
|--------|--------|
| 行为引擎分层 | 决策用纯函数（随机当参数），组件只管渲染 |
| 调度方式 | `setTimeout` 链，间隔可变；眨眼独立循环 |
| 表情实现 | 一套 SVG + 属性偏移/条件画法，别堆图 |

## 下一篇预告

[EP04 交互篇：摸头、戳一戳与右键菜单](../ep04/article.md) ——
宠物不能只是自说自话。下一篇加入鼠标交互：悬停摸头冒爱心、
单击戳一戳随机反应、右键菜单、拖着它满屏跑。
