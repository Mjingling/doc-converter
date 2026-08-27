# EP08 收尾：多语言、主题与打包送礼

> 系列第 8 篇（最终篇） | Demo：`ep08/demo` | 预计阅读 18 分钟

> 💡 **AI 协作贴士**：打包、签名、公证这类流程，AI 写草稿很快，
> 但平台政策变化快——正式发布前对照 Tauri/Apple 官方文档再核一遍，
> 别让 AI 背旧知识毁了最后一步。

最后一集做三件事：给应用上**多语言**和**明暗主题**的妆，
然后学会**打包分发**，把成品真正送到媳妇的电脑上。

## 运行本篇 Demo

```bash
cd tutorials/ep08/demo
npm install
npm run tauri dev
```

点右上角按钮切换语言和主题；跑个模拟任务，宠物的台词也是双语的。

## 多语言：vue-i18n 三步走

### 1. 文案按段落集中管理

```ts
export const messages = {
  "zh-CN": {
    app: { title: "文档工具箱", runSuccess: "模拟任务（成功）", ... },
    pet: { start: "开工啦！", working: "处理中 {pct}%", ... },
  },
  "en-US": {
    app: { title: "Doc Toolbox", ... },
    pet: { start: "Let's go!", ... },
  },
};
```

约定：`app.*` 主窗口用、`pet.*` 宠物用。
成品的四语言（中/英/日/韩）就是这个结构复制四份，
还写了脚本审计四份文案的 key 完整性——**多语言项目必做**，
漏一个 key 就是一处 `{key}` 裸奔。

### 2. 入口注册，跟随系统语言

```ts
const sysLang = navigator.language ?? "en-US";
const locale = sysLang.toLowerCase().startsWith("zh") ? "zh-CN" : "en-US";

const i18n = createI18n({
  legacy: false, // Composition API 模式，配合 useI18n()
  locale,
  fallbackLocale: "en-US",
  messages,
});
```

`legacy: false` 别忘，否则 `useI18n()` 拿不到响应式的 `locale`。

### 3. 带参数的翻译

宠物进度台词用插值：

```ts
t("pet.working", { pct: Math.round(progress.value) }); // "处理中 42%"
```

切换语言就一行：`locale.value = "en-US"`——
所有 `t()` 调用点响应式刷新，包括宠物窗口（同一份 i18n 定义）。
成品额外做手动切换 + `tauri-plugin-store` 持久化。

## 明暗主题：CSS 变量一招鲜

别给每个组件写两套样式。定义两套变量，组件全引用变量：

```css
.page {
  --bg: #f5f6f8;  --fg: #1a1a1a;  --border: #e3e5e8;  --accent: #2080f0;
}
.page.dark {
  --bg: #17181c;  --fg: #e6e8eb;  --border: #33353a;  --accent: #4d9fff;
}
button { background: var(--card); color: var(--fg); }
```

切换主题 = 换一个 class，`transition` 让过渡柔和。
成品的主题状态存 store，宠物窗口启动时读同一份，两窗口永远同步。

## 打包送礼

教学版各 demo 都设了 `bundle.active: false`（开发省事）。
真发布时打开它：

```json
"bundle": {
  "active": true,
  "targets": "all",
  "category": "Productivity",
  "icon": ["icons/32x32.png", "icons/128x128.png", "icons/icon.icns", "icons/icon.ico"]
}
```

### 1. 生成全套图标

准备一张 1024×1024 的 PNG，一条命令生成所有平台尺寸：

```bash
npx tauri icon path/to/logo.png
```

### 2. 打包

```bash
npm run tauri build
```

产物在 `src-tauri/target/release/bundle/`：

| 平台 | 产物 | 说明 |
|------|------|------|
| macOS | `*.dmg` + `*.app` | 媳妇拖进应用程序文件夹即可 |
| Windows | `*.msi` + `*.exe`(NSIS) | 需要 VS Build Tools |
| Linux | `*.deb` / `*.rpm` / `*.AppImage` | 看发行版 |

### 3. 签名与公证（正式分发必看）

- **macOS**：未签名应用会被 Gatekeeper 拦。自用可以右键打开绕过；
  对外分发需要 Apple Developer 证书 + `notarytool` 公证，
  Tauri 官方文档有完整的 CI 示例
- **Windows**：SmartScreen 会对无签名应用警告，
  可购买代码签名证书配置给 Tauri

> 成品的发布流水线在 `.github/workflows/release.yml`：
> 打 tag → GitHub Actions 三平台矩阵构建 → 自动挂产物到 Release。
> 想抄作业的可以直接参考。

### 4. 版本号

`tauri.conf.json` 的 `version` 是唯一事实源，
每次发布前 +1；成品的自动更新（`version.json` 比对）也基于它。

## 系列完结

回头看这 8 集我们走过的路：

```
EP01 蓝图 → EP02 空壳窗口 → EP03 有了灵魂 → EP04 能互动
→ EP05 能干活 → EP06 双窗口联动 → EP07 会聊天 → EP08 梳妆出门
```

每一集的 demo 都能独立运行，串起来就是一个真实上线的开源项目。
而这个项目从头到尾，是我和大模型结对完成的：
我出需求和验收，它出方案和代码，编译器和测试当裁判。
希望这个系列让你看到：**AI 结对编程不是嗪头，
是一种能交付真实产品的工作方式**——前提是方向盘在你手里。
接下来你可以：

- 把教程里的思路搬回成品仓库，对照 `src/` 与 `src-tauri/` 读源码
- 给宠物加新玩具：托盘图标、开机自启（`tauri-plugin-autostart`）、
  全局快捷键（`tauri-plugin-global-shortcut`）
- 给引擎加新功能：水印、拆分、图片转 PDF——套路都和 EP05 一样

祝你也做出一个让自己喜欢的人用得开心的小工具。
