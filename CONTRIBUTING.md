# 贡献指南

感谢您考虑为 DocMorph 做出贡献！在开始之前，请先阅读以下内容。

## 贡献者许可协议（CLA）

**所有代码贡献在合并前需签署本项目 CLA（Contributor License Agreement）。**

- 首次提交 Pull Request 时，CLA Assistant 机器人会自动在 PR 中评论一个签署链接
- 通过 GitHub 授权点击签署即可，约一分钟完成，签署一次适用于后续所有贡献
- 未签署 CLA 的 PR 将无法合并

### CLA 授权范围（请务必知悉）

签署 CLA 后，您保留贡献的署名权，同时授予项目所有者：

1. 以 Apache License 2.0（或项目当前采用的许可证）使用、复制、修改和分发您的贡献；
2. **以任何其他条款对您的贡献进行再许可（sublicense）与再发布（relicense）的权利**，
   包括在项目的未来版本中采用不同授权方式的权利。

> **English summary**: By signing the CLA, you grant the project owner a
> perpetual, worldwide, royalty-free, irrevocable license to use, reproduce,
> modify, distribute, **sublicense and relicense** your contribution under
> any terms, while retaining your authorship attribution.

此项授权确保项目所有者对代码拥有完整、无争议的处置权。如您不同意，请不要提交代码贡献，仍然欢迎通过 Issue 反馈问题与建议。

### 琐碎贡献豁免

十行以内的笔误修正、错别字、文案微调、纯翻译修正等琐碎贡献（trivial contributions）无需签署 CLA。某项贡献是否属于此类由维护者判定。

## 开发环境

参见 [README](README.md) 的「构建」一节：Node.js 22+、Rust 1.70+，`yarn install` 后执行 `yarn tauri dev`。

## 提交 Pull Request

1. 从 `master` 切出新分支（如 `feat/xxx`、`fix/xxx`）
2. 保持改动聚焦：一个 PR 只做一件事
3. 提交信息使用简洁的祈使句（如 `Fix PDF page range parsing`）
4. 涉及新功能时请同步更新 `src/i18n/locales/` 下四个语言文件
5. 等待 CLA 检查与 CI 构建通过

## 报告问题

提交 Issue 时请附上：系统版本、DocMorph 版本、复现步骤、报错截图或日志。
