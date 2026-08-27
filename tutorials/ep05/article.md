# EP05 干正事：Rust 文档处理引擎

> 系列第 5 篇 | Demo：`ep05/demo` | 预计阅读 20 分钟

本篇目标：暂时放下宠物，搭起应用的主窗口，用 Rust + [lopdf](https://github.com/J-F-Liu/lopdf)
实现两个真实功能——**PDF 合并**和 **PDF 压缩**，并掌握
`#[tauri::command]` / `invoke` 这条前后端通信大动脉。

从这里开始，桌宠有了"真本事"。

## 运行本篇 Demo

```bash
cd tutorials/ep05/demo
npm install
npm run tauri dev
```

准备两个小 PDF（随便什么都行），把路径填进文本框，点「开始合并」。
教程版用路径文本输入，成品里是文件对话框 + 拖拽，引擎代码完全一样。

## 核心概念：invoke —— 前端的函数调用，后端干活

Tauri 的 IPC 模型非常直观：

```
前端                                    Rust
invoke("pdf_merge", { inputs, output }) ──► #[tauri::command]
                                            fn pdf_merge(inputs: Vec<String>,
                                                         output: String)
        ◄── Promise<T> ──────────────────────  -> Result<T, String>
```

三条规则：

1. **参数名按名称匹配**：JS 传 `{ inputs, output }`，Rust 形参就得叫
   `inputs`、`output`（多参数时推荐 camelCase，宏会自动转换）
2. **类型自动序列化**：`Vec<String>` ↔ JS 数组、结构体（带 `#[derive(serde::Serialize)]`）
   ↔ JS 对象
3. **错误即 reject**：`Result<T, String>` 里的 `Err` 会让前端
   `await invoke(...)` 抛异常，`String(e)` 就是错误信息

注册命令只要一行：

```rust
tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![pdf_merge, pdf_compress])
    .run(tauri::generate_context!())
    .expect("启动失败");
```

前端调用：

```ts
import { invoke } from "@tauri-apps/api/core";

const msg = await invoke<string>("pdf_merge", { inputs, output });
```

## 架构铁律：命令薄、引擎厚

注意 `main.rs` 里的分层——这是成品 DocMorph（`commands/` + `engine/`）
的简化版：

```rust
// 引擎：纯函数，不知道 Tauri 的存在，可单测、可复用
fn merge_pdfs(inputs: &[String], output: &str) -> Result<usize, String> { ... }

// 命令：薄封装，只做错误格式转换和结果包装
#[tauri::command]
fn pdf_merge(inputs: Vec<String>, output: String) -> Result<String, String> {
    let pages = merge_pdfs(&inputs, &output)?;
    Ok(format!("合并完成：{} 个文件，共 {} 页", inputs.len(), pages))
}
```

为什么这么偏执？因为**命令很难单测（要起整个 Tauri 运行时），
纯函数随便测**。成品的 23 个 Rust 单测，全部打在引擎层。

## PDF 合并：对象搬运 + 重建树

PDF 本质是一棵对象树：`trailer → Catalog → Pages → Page[]`。
合并 = 把多个文档的对象搬到新文档（重新分配编号），再重建根。

难点在“搬”：两个文档都有 `(1,0)` 号对象，直接合并必然冲突。
解法是给源文档的每个对象在目标文档里分配全新编号，并把对象内部的
引用同步改指——这个递归重映射就是核心：

```rust
fn remap_object(obj: &Object, map: &HashMap<ObjectId, ObjectId>) -> Object {
    match obj {
        Object::Reference(id) => map.get(id).copied()
            .map(Object::Reference).unwrap_or_else(|| obj.clone()),
        Object::Array(arr) => Object::Array(arr.iter().map(|o| remap_object(o, map)).collect()),
        Object::Dictionary(dict) => /* 逐个递归 */,
        Object::Stream(s) => /* 流自带字典，也要重映射 */,
        other => other.clone(),
    }
}
```

搬运入口（与成品 `engine/pdf.rs` 的 `import_document` 同构）：

```rust
fn import_document(target: &mut Document, source: &Document) -> Vec<ObjectId> {
    let mut id_map = HashMap::new();
    for id in source.objects.keys() {
        target.max_id += 1;              // 目标文档分配全新编号，天然免冲突
        id_map.insert(*id, (target.max_id, 0));
    }
    for old in source.objects.keys() {
        target.objects.insert(id_map[old], remap_object(&source.objects[old], &id_map));
    }
    source.get_pages().values().map(|id| id_map[id]).collect() // 返回新页 id
}
```

然后重建 Pages 树，两个容易踩的坑就藏在里面：

```rust
let pages_id = result.add_object(pages_dict); // 新 Pages 根节点（Type/Kids/Count）

// 坑 1：每个 Page 必须指向父 Pages 节点，缺失会被部分渲染器拒绝打开
for pid in &page_ids {
    if let Ok(d) = result.get_object_mut(*pid).and_then(|o| o.as_dict_mut()) {
        d.set("Parent", Object::Reference(pages_id));
    }
}

// 坑 2：空文档的 trailer 没有 Root，Catalog 要确保存在再指 Pages
catalog.set("Pages", Object::Reference(pages_id));
```

收尾一步 `result.compress()` 压缩所有内容流，`save()` 落盘。
这套代码直接取自成品且带着 23 个单测的考验，是现成代码里最值钱的部分。

## PDF 压缩：先做对，再做狠

教学版压缩是**无损三件套**：

```rust
doc.delete_zero_length_streams(); // 删空流
doc.compress();                   // 压缩所有内容流
doc.save(output)?;
```

并顺手算出前后体积差返回给前端：

```rust
Ok(before.saturating_sub(after))
```

成品的"强力压缩"在此之上再做**有损**操作：
用 `image` 库把页面里的图片解码、降采样、重编码（质量 75 的 JPEG），
体积能再砍一半以上——思路一样，只是多走一步流的替换，
留给你按这个骨架去扩展。

## 为什么是 Rust + lopdf

- **隐私**：全程本地内存操作，文件不出硬盘
- **速度**：合并 100 个 10 页的 PDF 在百毫秒级
- **零运行时依赖**：不像某些方案要装 Ghostscript，
  lopdf 是纯 Rust，编译完就是单个可执行文件

## 本篇小结

| 知识点 | 一句话 |
|--------|--------|
| invoke | 参数按名匹配、自动序列化、`Err` 变 Promise reject |
| 分层 | 命令薄（只做格式转换）、引擎厚（纯函数可测） |
| 合并 | 对象搬运 + 编号重映射 + 重建 Pages 树（记得 Parent 指针） |
| 压缩 | 先无损（压流），有损（重编码图片）是进阶 |

## 下一篇预告

两条线要合体了！
[EP06 双窗口联动：让宠物汇报进度](../ep06/article.md) ——
主窗口 + 宠物窗口同时跑，用 Tauri 全局事件把任务进度实时投给宠物：
气泡进度条 → 完成庆祝 → 失败安慰。这是整个系列最"有灵魂"的一集。
