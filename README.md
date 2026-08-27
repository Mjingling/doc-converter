<div align="center">
  <img src="src-tauri/icons/128x128@2x.png" width="96" height="96" alt="DocMorph logo" />
  <h1>DocMorph</h1>
  <p>办公文档转换与 PDF 工具箱 — 轻量、离线、跨平台</p>
  <p>
    <a href="#功能">功能</a> · <a href="#安装">安装</a> · <a href="#使用">使用</a> · <a href="#构建">构建</a> · <a href="#国际化">国际化</a>
  </p>
  <p>
    <img src="https://img.shields.io/badge/Tauri_2-000?logo=tauri" />
    <img src="https://img.shields.io/badge/Vue_3.4-4FC08D?logo=vuedotjs" />
    <img src="https://img.shields.io/badge/Rust-2021-000?logo=rust" />
    <img src="https://img.shields.io/badge/license-Apache_2.0-blue" />
  </p>
</div>

---

## 简介

**DocMorph** 是一款桌面端办公文档转换与 PDF 处理工具，基于 Tauri 2 构建。提供 30+ 项 PDF 操作、文档格式互转与本地 AI 能力（摘要 / 问答 / 翻译），所有处理在本地完成，无需上传文件，保护隐私安全。

支持中、英、日、韩四种语言，适配浅色 / 深色主题。

## 功能

### PDF 处理
| 功能 | 说明 |
|------|------|
| 合并 PDF | 将多个 PDF 按顺序合并为一个文件 |
| 拆分 PDF | 按页范围将 PDF 拆分为多个独立文件 |
| 压缩 PDF | 减小 PDF 文件体积，便于存储与分享 |
| 提取 / 删除页面 | 按页码提取页面生成新 PDF，或删除指定页面 |
| 添加 / 去除水印 | 给 PDF 每一页添加平铺文字水印，或移除已有水印 |
| 旋转 / 页码 | 旋转 PDF 页面方向，或为每一页添加页码 |
| 加密 / 解密 | 为 PDF 设置打开密码，或移除已有密码 |
| 图片转 PDF | 将多张图片按顺序合成为一个 PDF 文件 |
| 批量处理 | 对文件夹中的全部 PDF 批量执行同一操作（合并 / 压缩 / 水印 / 解密 / 提取文本 / 提取图片等） |
| PDF 对比 | 逐页对比两个 PDF 的差异 |

### PDF 转图片与签名
| 功能 | 说明 |
|------|------|
| PDF 转图片（内置） | 内置 Pdfium 引擎逐页渲染为 PNG / JPG，可选页码范围与 DPI（72 / 150 / 300），无需 LibreOffice |
| 电子签名 | 在 PDF 指定页添加签名图片，支持四角 / 居中位置预设与宽度调节 |

### PDF 扩展工具
| 功能 | 说明 |
|------|------|
| PDF 元数据 | 编辑文档的标题、作者、主题和关键词 |
| 裁剪 PDF | 统一裁剪所有页面的边距 |
| PDF 书签 | 为文档添加书签（大纲），方便导航 |
| 提取 PDF 图片 | 从 PDF 中提取所有嵌入的图片 |
| 提取 DOCX 图片 | 从 Word 文档中提取所有嵌入的图片 |
| 图片压缩 | 压缩 JPEG / PNG 图片文件 |
| 图片格式转换 | PNG / JPG / WebP / BMP / GIF 格式互转，可选等比缩放尺寸 |
| 批量重命名 | 按规则批量重命名文件 |
| 网页转 PDF | 将网页内容抓取并转换为 PDF |

### AI 智能（本地 + 可选云端）
| 功能 | 说明 |
|------|------|
| AI 助手（兔小胖） | 对话式助手，支持文件上下文、跨轮对话记忆与实时网页搜索（智谱 / Tavily，回答附来源链接） |
| AI 文档摘要 | 提取文档全文生成摘要，支持要点列表 / 待办提取 / 会议纪要 / 脑图大纲等多种格式 |
| 文档问答（RAG） | 多文档向量化索引，基于文档内容问答并展示引用来源 |
| AI 翻译 | 文档分块翻译为中 / 英 / 日 / 韩 / 法 / 德六语种，输出双语 Markdown |
| 桌面宠物 | 桌面右下角常驻机器人：悬停摸头、单击戳一戳、右键菜单直达常用功能；同步 AI 状态、任务进度与小贴士 |

### 文档转换
| 功能 | 说明 |
|------|------|
| PDF → Word / Excel / 图片 | 将 PDF 转换为可编辑的 Office 文档或图片 |
| Word / Excel / PPT → PDF | 将 Office 文档转换为 PDF 格式 |
| 通用格式转换 | 支持 EPUB、HTML、Markdown、TXT、CSV 等格式互转 |

### 其他特性
- **命令面板** — `Cmd / Ctrl + K` 快速搜索并切换全部功能
- **拖拽直达** — 文件拖到任意位置自动弹出快捷操作（一键转换 / 压缩 / 合成 PDF / AI 摘要）
- **完成通知** — 长任务完成后推送系统通知
- **文件夹监控** — 自动监控指定文件夹，新文件到达后按规则自动转换
- **历史记录** — 自动记录操作历史，可快速打开输出文件或定位目录
- **任务队列** — 批量文档转换时的顺序执行与进度跟踪
- **Finder 集成** — 在访达中右键文件直接通过 DocMorph 打开（macOS）
- **托盘图标** — 常驻系统托盘，后台运行，全局快捷键唤出
- **明暗主题** — 跟随系统或手动切换浅色 / 深色主题
- **四语言界面** — 中文、英文、日文、韩文

## 安装

### 系统要求
- **macOS** 11.0+（Intel 或 Apple Silicon）
- **Node.js** 22+（开发构建用）
- **Rust** 1.70+（开发构建用）

### 下载
从 [Releases](https://gitee.com/speed_turbo/doc-converter/releases) 页面下载最新版本的 `.dmg` 安装包，拖入 Applications 文件夹即可。

## 使用

### 首次启动
1. 打开 DocMorph，进入主界面
2. 在左侧导航栏选择需要操作的功能
3. 拖拽或点击选择文件，按提示完成操作

### 引擎切换
DocMorph 支持两种转换引擎：
- **内置引擎** — 轻量快速，支持 TXT / Markdown / HTML → PDF 转换，无需额外依赖
- **LibreOffice 引擎** — 完整支持 Office 文档互转，需安装 [LibreOffice](https://www.libreoffice.org/)

在侧边栏底部可切换引擎并查看运行状态。

### 文件夹监控
1. 点击设置 → 文件夹监控
2. 选择监控目录，配置转换规则（源文件扩展名 → 目标格式）
3. 启动监控后，新文件到达会自动按规则转换

### Finder 集成（macOS）
安装 Finder 服务后，在访达中右键任意 PDF 或文档文件，选择"用 DocMorph 打开"即可快速导入。

```bash
# 安装 Finder 服务
./scripts/install-finder-service.sh
```

## 国际化

DocMorph 内置四种语言界面，可在设置中切换：

| 语言 | 代码 | 说明 |
|------|------|------|
| 简体中文 | `zh-CN` | 默认 |
| English | `en-US` | 英文 |
| 日本語 | `ja-JP` | 日文 |
| 한국어 | `ko-KR` | 韩文 |

主题跟随系统或手动切换浅色 / 深色模式。

## 构建

### 前置依赖
- Node.js 22+
- Rust 1.70+
- macOS：Xcode Command Line Tools

### 克隆与构建

```bash
git clone https://gitee.com/speed_turbo/doc-converter.git
cd doc-converter

# 安装前端依赖
yarn install

# 下载 Pdfium 动态库（PDF 转图片功能需要，首次构建前执行一次）
./scripts/download_pdfium.sh

# 开发模式
yarn tauri dev

# 生产构建
yarn tauri build
```

构建产物位于 `src-tauri/target/release/bundle/` 目录下。

### 测试

```bash
# 前端单测（Vitest）
yarn test

# 类型检查
npx vue-tsc --noEmit

# Rust 单测（PDF 渲染冒烟测试需本地 Pdfium 库，默认 #[ignore]）
cd src-tauri && cargo test
```

### 技术栈
| 层级 | 技术 |
|------|------|
| 桌面框架 | Tauri 2 |
| 前端框架 | Vue 3.4 + TypeScript |
| UI 组件库 | Naive UI 2.43 |
| 状态管理 | Pinia |
| 国际化 | vue-i18n |
| 构建工具 | Vite 5 |
| 前端测试 | Vitest + Vue Test Utils |
| 后端语言 | Rust (edition 2021) |
| PDF 处理 | lopdf 0.34 |
| PDF 渲染 | pdfium-render 0.8（动态库随包分发） |
| 图片处理 | image 0.25 |
| 本地 AI | @huggingface/transformers（向量嵌入），可选 OpenAI 兼容云端 API 与网页搜索（智谱 / Tavily） |
| 文件监控 | notify 6 |

## 项目结构

```
doc-converter/
├── src/                    # 前端代码
│   ├── components/         # 功能面板组件（30+ 个面板）
│   ├── views/              # 页面视图
│   ├── stores/             # Pinia 状态管理
│   ├── ai/                 # AI 路由与 RAG 检索
│   ├── utils/              # 工具函数（通知 / 分块 / 页码解析等）
│   ├── i18n/               # 国际化（4 语言）
│   └── api/                # Tauri 命令封装
├── src-tauri/              # Rust 后端
│   ├── src/commands/       # Tauri 命令实现
│   ├── src/engine/         # 引擎模块（PDF / LibreOffice / Pdfium 渲染）
│   ├── resources/pdfium/   # Pdfium 动态库（脚本下载）
│   └── icons/              # 应用图标
├── docs/                   # 文档
│   └── UI-SPEC.md          # UI 规范
├── finder/                 # macOS Finder 集成
└── scripts/                # 辅助脚本（含 download_pdfium.sh）
```

## 许可证

本项目代码以 [Apache License 2.0](LICENSE) 授权。

**商标保留**：「DocMorph」名称、Logo 及本项目的其他品牌标识归项目所有者所有，不在 Apache 2.0 授权范围内。未经书面许可，不得将上述名称或标识用于衍生产品、fork 版本的命名或对外推广中。