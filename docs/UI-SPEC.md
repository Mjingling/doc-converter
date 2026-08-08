# DocMorph UI 规范

> 本文档从现有代码（App.vue 全局变量 + 21 个功能面板）中提炼的事实规范，新增组件时请遵循。
> 定义位置：`src/App.vue`（全局 CSS 变量与字体）、各 `src/components/*Panel.vue`。

## 1. 设计 Token（全局 CSS 变量）

所有颜色一律通过 `var(--xxx)` 引用，禁止硬编码色值（明暗主题由 `html[data-theme]` 自动切换）。

### 文字色（5 级，从强到弱）

| Token | 用途 |
|---|---|
| `--text-main` | 标题、激活态、强调 |
| `--text-body` | 正文 |
| `--text-sub` | 次要文字、表单标签旁说明 |
| `--text-muted` | 辅助信息、占位 |
| `--text-faint` | 最弱（空状态图标、禁用） |

### 背景色（5 级）

| Token | 用途 |
|---|---|
| `--bg-page` | 页面底色 |
| `--bg-panel` | 面板卡片 |
| `--bg-input` | 输入框、引擎卡片 |
| `--bg-hover` | 悬浮态 |
| `--bg-active` | 选中态 |

### 边框（4 级）

| Token | 用途 |
|---|---|
| `--border` | 默认分隔 |
| `--border-strong` | 按钮描边 |
| `--border-soft` | 列表内分隔线 |
| `--border-dash` | 拖放区虚线 |

### 功能色

| Token | 用途 |
|---|---|
| `--accent` / `--accent-soft` | 主操作色（蓝），浅底变体作标签底 |
| `--green` / `--green-soft` | 成功、内置引擎标签 |
| `--red` / `--red-soft` | 失败、删除、PDF 工具图标（`#e6494c`） |
| `--orange` / `--orange-soft` | 警告、需安装提示 |
| `--cta-bg` / `--cta-text` | 主按钮（反色 CTA） |
| `--shadow` | 面板阴影（`0 1px 3px var(--shadow)`） |

## 2. 字体

字体栈（html/body 全局，无需重复声明）：

```
-apple-system, BlinkMacSystemFont, "PingFang SC", "Helvetica Neue", "Microsoft YaHei", sans-serif
```

### 字号层级

| 字号 | 用途 |
|---|---|
| 22px | 面板标题 h2（`font-weight: normal` 默认） |
| 14px | 正文、导航项、表单主标签、文件名 |
| 13px | 副标题/说明、面板头描述、字段 label（`font-weight: 600`） |
| 12px | 分组标题、次要操作、hint 提示 |
| 11px | 引擎描述、engine-tag/need-engine 角标 |
| 10px | 引擎标签角标（`--green`/`--accent` 底 + soft 背景） |

## 3. 圆角

| 元素 | 圆角 |
|---|---|
| 面板卡片 | 14px |
| 拖放区 | 14px |
| 导航项 | 8px |
| 按钮 | 7–8px |
| 角标（tag） | 8px |

## 4. 间距

### 面板结构（所有功能面板统一）

```css
.panel {
  padding: 30px;
  background: var(--bg-panel);
  border-radius: 14px;
  box-shadow: 0 1px 3px var(--shadow);
}
.panel-head h2 { margin: 0; font-size: 22px; color: var(--text-main); }
.panel-head p { margin: 6px 0 0; font-size: 13px; color: var(--text-sub); }
```

所有面板统一使用 `.panel-head` 结构（h2 + p 副标题），无例外。

### 通用间距

| 场景 | 值 |
|---|---|
| 容器 padding | 四边一致：面板 `30px`、主内容区 `.content` `20px`（容器一律不写不对称值） |
| 表单字段间距（纵向 stack） | `gap: 14px`；行内字段 `field-row` 也 `gap: 14px` |
| 行内元素组合 | gap 8px（图标+文字）/ 10px / 12px |
| 拖放区 | `padding: 28px`，虚线边框 2px |
| 导航项 | `padding: 9px 10px`，间距 2px |
| 小按钮 | `padding: 5px 12px` / `6px 18px` |
| 面板头与内容之间 | 20px |
| 标签角标 | `padding: 1px 6px`，`line-height: 16px` |

## 5. 状态与交互

- 拖放区悬浮：边框变 `--accent` + 背景 `--accent-soft`
- 悬浮态：`background: var(--bg-hover)`，过渡 `transition: background 0.15s`
- 激活态：`background: var(--bg-active)`，`font-weight: 600`，文字 `--text-main`
- 链接式按钮（`.link-btn`）：无边框无背景、`--accent` 色、悬浮下划线
- 删除类按钮：默认 `--text-muted`，悬浮 `--red`
- 主操作按钮（`.switch-btn` 等 CTA）：`background: var(--cta-bg)`、`color: var(--cta-text)`，悬浮 opacity 0.85
- 禁用态：`opacity: 0.5` + `cursor: not-allowed`

## 6. 组件结构约定

- 每个功能面板独立组件：`src/components/*Panel.vue`，模板根节点为 `.panel`
- 顶部标题区：标题 + 副标题（文案走 i18n，key 挂 `scenes.xxx` / 各面板 `xxx.title`）
- 文件拖放：`.upload-zone`（可点击打开 + 拖放），空状态 `.zone-empty`（功能图标 34px + `.zone-main` 提示 + `.zone-sub` 补充说明），选中后 `.zone-filled`（文档图标 + 文件名 + `.size-tag` 类型标签 + 清除按钮）
- CTA 区：`.action-row`（顶部分隔线，左侧 `.hint` 说明 + 右侧 `.cta` 主按钮，带功能图标），按钮禁用用 `:disabled`
- 图标：统一 `@vicons/ionicons5`，经 `<NIcon :component="..." />` 渲染；功能图标按语义分组着色（PDF 工具 `#e6494c`、LibreOffice 转换 `#2080f0` 等，见 SideNav.vue groups）
- Naive UI 组件由 `NConfigProvider` 统一主题/语言，组件内不再做主题判断

## 7. 明暗主题

- 主题切换：`html[data-theme="light"|"dark"]`（App.vue 中由 `resolvedDark` 驱动）
- 自定义样式全部走 CSS 变量，天然适配双主题；深色滚动条等交给系统
- 新增颜色必须同时补充浅色（`:root`）与深色（`html[data-theme="dark"]`）两套值
