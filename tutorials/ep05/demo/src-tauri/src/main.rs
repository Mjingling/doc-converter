//! EP05：Rust 文档处理引擎
//!
//! 架构：commands（薄）→ engine（厚）
//! - #[tauri::command] 只做参数/错误格式转换
//! - 真正的合并/压缩逻辑在纯函数里，可以单测、可以脱离 UI 复用
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use lopdf::{Dictionary, Document, Object, ObjectId, Stream};
use std::collections::HashMap;

/// 把对象里的引用按 id_map 重新指向新编号（递归处理数组/字典/流）
fn remap_object(obj: &Object, map: &HashMap<ObjectId, ObjectId>) -> Object {
    match obj {
        Object::Reference(id) => map
            .get(id)
            .copied()
            .map(Object::Reference)
            .unwrap_or_else(|| obj.clone()),
        Object::Array(arr) => Object::Array(arr.iter().map(|o| remap_object(o, map)).collect()),
        Object::Dictionary(dict) => {
            Object::Dictionary(dict.iter().map(|(k, v)| (k.clone(), remap_object(v, map))).collect())
        }
        Object::Stream(s) => {
            let new_dict: Dictionary = s.dict.iter().map(|(k, v)| (k.clone(), remap_object(v, map))).collect();
            Object::Stream(Stream::new(new_dict, s.content.clone()))
        }
        other => other.clone(),
    }
}

/// 把 source 的全部对象搬进 target（重新分配编号），返回源页面在 target 里的新 id。
/// 这段是成品 DocMorph engine/pdf.rs 的简化版。
fn import_document(target: &mut Document, source: &Document) -> Vec<ObjectId> {
    let source_ids: Vec<ObjectId> = source.objects.keys().copied().collect();
    let mut id_map: HashMap<ObjectId, ObjectId> = HashMap::with_capacity(source_ids.len());
    for id in &source_ids {
        target.max_id += 1; // 目标文档分配全新编号，天然避免冲突
        id_map.insert(*id, (target.max_id, 0));
    }
    for old in &source_ids {
        if let Some(obj) = source.objects.get(old) {
            target.objects.insert(id_map[old], remap_object(obj, &id_map));
        }
    }
    source.get_pages().values().map(|id| id_map[id]).collect()
}

/// 合并多个 PDF，返回总页数
fn merge_pdfs(inputs: &[String], output: &str) -> Result<usize, String> {
    if inputs.len() < 2 {
        return Err("至少需要选择 2 个 PDF 文件".to_string());
    }
    let mut result = Document::with_version("1.4");
    let mut page_ids: Vec<ObjectId> = Vec::new();
    for path in inputs {
        let doc = Document::load(path).map_err(|e| format!("打开失败 {path}: {e}"))?;
        page_ids.extend(import_document(&mut result, &doc));
    }

    // 重建根页树，挂载全部页面
    let kids: Vec<Object> = page_ids.iter().map(|id| Object::Reference(*id)).collect();
    let pages = Dictionary::from_iter(vec![
        (b"Type".to_vec(), Object::Name(b"Pages".to_vec())),
        (b"Kids".to_vec(), Object::Array(kids)),
        (b"Count".to_vec(), Object::Integer(page_ids.len() as i64)),
    ]);
    let pages_id = result.add_object(pages);

    // PDF 规范要求每个 Page 必须指向其父 Pages 节点，缺失会被部分渲染器拒绝
    for pid in &page_ids {
        if let Ok(page) = result.get_object_mut(*pid) {
            if let Ok(d) = page.as_dict_mut() {
                d.set("Parent", Object::Reference(pages_id));
            }
        }
    }

    // 确保存在 Catalog（空文档的 trailer 没有 Root，需手动创建）
    let catalog_id = match result.trailer.get(b"Root").and_then(|o| o.as_reference()) {
        Ok(id) => id,
        Err(_) => {
            let catalog = Dictionary::from_iter(vec![(
                b"Type".to_vec(),
                Object::Name(b"Catalog".to_vec()),
            )]);
            let id = result.add_object(Object::Dictionary(catalog));
            result.trailer.set("Root", Object::Reference(id));
            id
        }
    };
    if let Ok(catalog) = result.get_dictionary_mut(catalog_id) {
        catalog.set("Pages", Object::Reference(pages_id));
    }

    result.compress(); // 压缩所有内容流（zlib）
    result.save(output).map_err(|e| format!("保存失败 {output}: {e}"))?;
    Ok(page_ids.len())
}

/// 压缩单个 PDF（无损：压缩内容流 + 清理空流），返回节省的字节数
fn compress_pdf(input: &str, output: &str) -> Result<u64, String> {
    let before = std::fs::metadata(input)
        .map_err(|e| format!("读取失败 {input}: {e}"))?
        .len();
    let mut doc = Document::load(input).map_err(|e| format!("打开失败 {input}: {e}"))?;
    doc.delete_zero_length_streams();
    doc.compress();
    doc.save(output).map_err(|e| format!("保存失败 {output}: {e}"))?;
    let after = std::fs::metadata(output)
        .map_err(|e| format!("读取失败 {output}: {e}"))?
        .len();
    Ok(before.saturating_sub(after))
}

// ── Tauri 命令：薄封装，错误统一转 String 交给前端 ──────────

#[tauri::command]
fn pdf_merge(inputs: Vec<String>, output: String) -> Result<String, String> {
    let pages = merge_pdfs(&inputs, &output)?;
    Ok(format!("合并完成：{} 个文件，共 {} 页 → {}", inputs.len(), pages, output))
}

#[tauri::command]
fn pdf_compress(input: String, output: String) -> Result<String, String> {
    let saved = compress_pdf(&input, &output)?;
    Ok(format!("压缩完成，减小 {:.1} KB → {}", saved as f64 / 1024.0, output))
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![pdf_merge, pdf_compress])
        .run(tauri::generate_context!())
        .expect("启动 Tauri 应用失败");
}
