//! PDF 工具集（基于 lopdf 0.34）：合并 / 拆分 / 压缩 / 水印 / 页码 / 旋转 / 加解密 / 图片转 PDF / 提取图片 / 去水印 / 比较
use crate::engine::font;
use image::ImageEncoder;
use lopdf::content::{Content, Operation};
use lopdf::{Dictionary, Document, Object, ObjectId, Stream, StringFormat};
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// 加载 PDF 并检查是否加密（加密文件需先解密后操作）
fn load_pdf(path: &Path) -> Result<Document, String> {
    let doc = Document::load(path).map_err(|e| format!("读取 PDF 失败: {}", e))?;
    if doc.is_encrypted() {
        return Err("输入的 PDF 已加密，请先使用「解密」功能处理后再操作".to_string());
    }
    Ok(doc)
}

/// 递归重写对象中的引用（导入文档时重新编号）
fn remap_object(obj: &Object, map: &HashMap<ObjectId, ObjectId>) -> Object {
    match obj {
        Object::Reference(id) => {
            Object::Reference(*map.get(id).unwrap_or(id))
        }
        Object::Array(arr) => {
            Object::Array(arr.iter().map(|o| remap_object(o, map)).collect())
        }
        Object::Dictionary(dict) => {
            Object::Dictionary(dict.iter().map(|(k, v)| (k.clone(), remap_object(v, map))).collect())
        }
        Object::Stream(s) => {
            let new_dict: Dictionary = s
                .dict
                .iter()
                .map(|(k, v)| (k.clone(), remap_object(v, map)))
                .collect();
            Object::Stream(Stream::new(new_dict, s.content.clone()))
        }
        other => other.clone(),
    }
}

/// 将 source 文档的所有对象导入 target，返回源页面在 target 中的新对象 id
fn import_document(target: &mut Document, source: &Document) -> Result<Vec<ObjectId>, String> {
    let source_ids: Vec<ObjectId> = source.objects.keys().copied().collect();
    let mut id_map: HashMap<ObjectId, ObjectId> = HashMap::with_capacity(source_ids.len());
    for id in &source_ids {
        target.max_id += 1;
        id_map.insert(*id, (target.max_id, 0));
    }
    for old in &source_ids {
        let obj = source
            .objects
            .get(old)
            .ok_or_else(|| format!("源文档缺少对象 {:?}", old))?;
        target.objects.insert(id_map[old], remap_object(obj, &id_map));
    }
    let page_ids: Vec<ObjectId> = source
        .get_pages()
        .values()
        .map(|id| id_map[id])
        .collect();
    Ok(page_ids)
}

/// 合并多个 PDF 为一个
pub fn merge_pdfs(paths: &[PathBuf], out_path: &Path) -> Result<(), String> {
    if paths.len() < 2 {
        return Err("至少需要选择 2 个 PDF 文件".into());
    }
    let mut result = Document::with_version("1.4");
    let mut page_ids: Vec<ObjectId> = Vec::new();
    for p in paths {
        let doc = load_pdf(p).map_err(|e| format!("读取 {} 失败: {}", p.display(), e))?;
        let ids = import_document(&mut result, &doc)?;
        page_ids.extend(ids);
    }

    // 重建根页树，挂载全部页面
    let kids: Vec<Object> = page_ids.iter().map(|id| Object::Reference(*id)).collect();
    let pages = Dictionary::from_iter(vec![
        (b"Type".to_vec(), Object::Name(b"Pages".to_vec())),
        (b"Kids".to_vec(), Object::Array(kids)),
        (b"Count".to_vec(), Object::Integer(page_ids.len() as i64)),
    ]);
    let pages_id = result.add_object(pages);
    // PDF 规范要求每个 Page 必须指向其父 Pages 节点；缺失会被 Apple 预览等渲染器拒绝
    for pid in &page_ids {
        if let Ok(page) = result.get_object_mut(*pid) {
            if let Ok(d) = page.as_dict_mut() {
                d.set("Parent", Object::Reference(pages_id));
            }
        }
    }

    // 确保存在 catalog（空文档的 trailer 没有 Root，需手动创建）
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

    result.save(out_path).map_err(|e| format!("保存失败: {}", e))?;
    Ok(())
}

/// 按页范围拆分 PDF（每个范围输出一个文件）
pub fn split_pdf(
    path: &Path,
    ranges: &[(u32, u32)],
    out_dir: &Path,
    prefix: &str,
) -> Result<Vec<PathBuf>, String> {
    if ranges.is_empty() {
        return Err("至少需要一个页范围".into());
    }
    std::fs::create_dir_all(out_dir).map_err(|e| format!("创建输出目录失败: {}", e))?;
    let mut doc = load_pdf(path)?;
    let total = doc.get_pages().len() as u32;
    let mut outputs = Vec::new();
    for (start, end) in ranges.iter() {
        if *start < 1 || *start > total || *end < *start {
            return Err(format!("范围 {}~{} 超出文档页数（共 {} 页）", start, end, total));
        }
        let end = (*end).min(total);
        let keep: std::collections::HashSet<u32> = (*start..=end).collect();
        let del: Vec<u32> = (1..=total).filter(|n| !keep.contains(n)).collect();
        doc.delete_pages(&del);

        // 命名：原文件名_页码范围.pdf（单页范围省略连字符，如 report_5.pdf）
        let range = if *start == end {
            format!("{}", *start)
        } else {
            format!("{}-{}", *start, end)
        };
        let out = out_dir.join(format!("{}_{}.pdf", prefix, range));
        doc.save(&out).map_err(|e| format!("保存失败: {}", e))?;
        outputs.push(out);
        // 下一段前重新加载原始文档
        doc = load_pdf(path)?;
    }
    Ok(outputs)
}

/// 为已导入对象的目标文档重建根页树，并确保 catalog 指向它（import_document 后调用）
fn rebuild_pages_tree(result: &mut Document, page_ids: &[ObjectId]) -> Result<(), String> {
    let kids: Vec<Object> = page_ids.iter().map(|id| Object::Reference(*id)).collect();
    let pages = Dictionary::from_iter(vec![
        (b"Type".to_vec(), Object::Name(b"Pages".to_vec())),
        (b"Kids".to_vec(), Object::Array(kids)),
        (b"Count".to_vec(), Object::Integer(page_ids.len() as i64)),
    ]);
    let pages_id = result.add_object(pages);
    // 导入的页面 /Parent 仍指向源文档的 Pages 节点，必须重指向新根页树
    for pid in page_ids {
        if let Ok(page) = result.get_object_mut(*pid) {
            if let Ok(d) = page.as_dict_mut() {
                d.set("Parent", Object::Reference(pages_id));
            }
        }
    }

    // 确保存在 catalog（空文档的 trailer 没有 Root，需手动创建）
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
    Ok(())
}

/// 按指定页序提取页面（可挑页 / 重排；页码 1 起，不允许重复）
pub fn extract_pages(path: &Path, out_path: &Path, pages: &[u32]) -> Result<(), String> {
    if pages.is_empty() {
        return Err("至少指定一个页面".into());
    }
    let doc = load_pdf(path)?;
    let total = doc.get_pages().len() as u32;
    let mut seen = HashSet::with_capacity(pages.len());
    for p in pages {
        if *p < 1 || *p > total {
            return Err(format!("页码 {} 超出文档范围（共 {} 页）", p, total));
        }
        if !seen.insert(*p) {
            return Err(format!("页码 {} 重复指定", p));
        }
    }
    let mut result = Document::with_version("1.4");
    let ids = import_document(&mut result, &doc)?;
    let page_ids: Vec<ObjectId> = pages.iter().map(|p| ids[*p as usize - 1]).collect();
    rebuild_pages_tree(&mut result, &page_ids)?;
    result.save(out_path).map_err(|e| format!("保存失败: {}", e))?;
    Ok(())
}

/// 删除指定范围的页面（保留其余页，保持原顺序；范围 1 起）
pub fn delete_pages_range(path: &Path, out_path: &Path, ranges: &[(u32, u32)]) -> Result<(), String> {
    if ranges.is_empty() {
        return Err("至少指定一个删除范围".into());
    }
    let mut doc = load_pdf(path)?;
    let total = doc.get_pages().len() as u32;
    let mut del: Vec<u32> = Vec::new();
    for (start, end) in ranges {
        if *start < 1 || *end > total || *start > *end {
            return Err(format!("范围 {}~{} 超出文档页数（共 {} 页）", start, end, total));
        }
        del.extend(*start..=*end);
    }
    if del.len() as u32 >= total {
        return Err("不能删除全部页面".into());
    }
    del.sort_unstable();
    del.dedup();
    doc.delete_pages(&del);
    doc.save(out_path).map_err(|e| format!("保存失败: {}", e))?;
    Ok(())
}

/// 基础压缩（对象流压缩；对已压缩的图片流效果有限）
pub fn compress_pdf(path: &Path, out_path: &Path) -> Result<(), String> {
    let mut doc = load_pdf(path)?;
    doc.compress();
    doc.save(out_path).map_err(|e| format!("保存失败: {}", e))?;
    Ok(())
}

// ==================== 辅助函数 ====================

/// 读取页面 MediaBox（支持间接引用），异常时退回 A4
fn get_page_media_box(doc: &Document, page_id: ObjectId) -> Result<[f32; 4], String> {
    let page = doc.get_dictionary(page_id).map_err(|e| format!("读取页面失败: {}", e))?;
    let mb = page
        .get(b"MediaBox")
        .ok()
        .cloned()
        .unwrap_or_else(|| {
            Object::Array(vec![
                Object::Integer(0),
                Object::Integer(0),
                Object::Integer(595),
                Object::Integer(842),
            ])
        });
    let arr = match &mb {
        Object::Reference(id) => doc
            .get_object(*id)
            .ok()
            .and_then(|o| o.as_array().ok())
            .ok_or("页面 MediaBox 引用无效")?,
        other => other.as_array().map_err(|_| "页面 MediaBox 格式异常")?,
    };
    if arr.len() < 4 {
        return Err("页面 MediaBox 不完整".into());
    }
    Ok([
        arr[0].as_f32().unwrap_or(0.0),
        arr[1].as_f32().unwrap_or(0.0),
        arr[2].as_f32().unwrap_or(595.0),
        arr[3].as_f32().unwrap_or(842.0),
    ])
}

/// 向页面 Contents 追加一个内容流（保留原流，仅追加新引用）
fn append_page_content(
    doc: &mut Document,
    page_id: ObjectId,
    stream_id: ObjectId,
) -> Result<(), String> {
    let contents = doc
        .get_dictionary(page_id)
        .ok()
        .and_then(|p| p.get(b"Contents").ok().cloned());
    match contents {
        Some(Object::Reference(_)) | Some(Object::Array(_)) => {
            let page = doc
                .get_dictionary_mut(page_id)
                .map_err(|e| format!("读取页面失败: {}", e))?;
            match page.get_mut(b"Contents").ok() {
                Some(Object::Array(arr)) => arr.push(Object::Reference(stream_id)),
                Some(other @ Object::Reference(_)) => {
                    let old = other.clone();
                    *other = Object::Array(vec![old, Object::Reference(stream_id)]);
                }
                _ => page.set("Contents", Object::Reference(stream_id)),
            }
        }
        _ => {
            if let Ok(page) = doc.get_dictionary_mut(page_id) {
                page.set("Contents", Object::Reference(stream_id));
            }
        }
    }
    Ok(())
}

/// 向页面 Resources 注入资源（Font / ExtGState / XObject 等），支持间接引用，名字冲突自动加序号
fn ensure_page_resource(
    doc: &mut Document,
    page_id: ObjectId,
    key: &[u8],
    name: &[u8],
    obj_id: ObjectId,
) -> Result<(), String> {
    let res_ref = doc.get_dictionary(page_id).ok().and_then(|p| match p.get(b"Resources").ok() {
        Some(Object::Reference(id)) => Some(*id),
        _ => None,
    });
    let mut res: Dictionary = match res_ref {
        Some(id) => doc.get_dictionary(id).ok().cloned().unwrap_or_default(),
        None => doc
            .get_dictionary(page_id)
            .ok()
            .and_then(|p| match p.get(b"Resources").ok() {
                Some(Object::Dictionary(d)) => Some(d.clone()),
                _ => None,
            })
            .unwrap_or_default(),
    };
    let sub_ref = match res.get(key).ok() {
        Some(Object::Reference(id)) => Some(*id),
        _ => None,
    };
    let mut sub: Dictionary = match sub_ref {
        Some(id) => doc.get_dictionary(id).ok().cloned().unwrap_or_default(),
        None => match res.get(key).ok() {
            Some(Object::Dictionary(d)) => d.clone(),
            _ => Dictionary::new(),
        },
    };
    // 同名但指向不同对象时追加序号（避免覆盖已有资源）
    let mut final_name = name.to_vec();
    let mut suffix = 1u8;
    while let Some(Object::Reference(id)) = sub.get(&final_name).ok() {
        if *id == obj_id {
            break;
        }
        final_name = format!("{}{}", String::from_utf8_lossy(name), suffix).into_bytes();
        suffix += 1;
    }
    sub.set(final_name, Object::Reference(obj_id));
    res.set(key.to_vec(), Object::Dictionary(sub));
    if let Ok(page) = doc.get_dictionary_mut(page_id) {
        page.set("Resources", Object::Dictionary(res));
    }
    Ok(())
}

/// 把 Unicode 码点序列转为 Identity-H 编码的十六进制字符串字节（每字符 2 字节大端 CID）
fn cid_bytes(chars: &[char]) -> Vec<u8> {
    let mut v = Vec::with_capacity(chars.len() * 2);
    for &c in chars {
        let b = (c as u32).to_be_bytes();
        v.push(b[2]);
        v.push(b[3]);
    }
    v
}

// ==================== 水印 ====================

/// 为 PDF 每一页添加平铺文字水印（嵌入系统中文字体，支持透明度/颜色/字号）
/// color: RGB 各分量 0~1；角度固定 -45°（左上→右下，Word 水印风格）
pub fn add_watermark(
    path: &Path,
    out_path: &Path,
    text: &str,
    opacity: f32,
    color: (f32, f32, f32),
    font_size: f32,
) -> Result<(), String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("水印文字不能为空".into());
    }
    let chars: Vec<char> = text.chars().collect();
    if chars.iter().any(|c| *c as u32 > 0xFFFF) {
        return Err("水印暂不支持 emoji 等非 BMP 字符".into());
    }
    let opacity = opacity.clamp(0.05, 1.0);
    let font_size = font_size.clamp(8.0, 120.0);
    let (r, g, b) = color;
    let (r, g, b) = (r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0));

    let mut doc = load_pdf(path)?;
    let mut font = font::load_system_font()?;
    let font_id = font.build_cid_font(&mut doc, &chars)?;

    // ExtGState：透明度（ca 控制非描边/描边透明度）
    let gs_id = doc.add_object(Object::Dictionary(Dictionary::from_iter(vec![
        (b"Type".to_vec(), Object::Name(b"ExtGState".to_vec())),
        (b"ca".to_vec(), Object::Real(opacity)),
        (b"CA".to_vec(), Object::Real(opacity)),
    ])));

    // 平铺参数：-45° 斜向（左上→右下）、步长按文本宽度自适应
    let text_w = font.text_width(&chars) as f32 * font_size / 1000.0;
    let step_x = text_w + font_size * 2.4;
    let step_y = font_size * 2.2;
    let angle = (-45.0f32).to_radians();
    let (cos, sin) = (angle.cos(), angle.sin());

    let pages = doc.get_pages();
    for (_no, page_id) in pages.iter() {
        let mb = get_page_media_box(&doc, *page_id)?;
        let pw = mb[2] - mb[0];
        let ph = mb[3] - mb[1];
        let mut ops = vec![
            Operation::new("q", vec![]),
            Operation::new("gs", vec![Object::Name(b"GS1".to_vec())]),
            // 填充色（非描边）：水印文字颜色
            Operation::new(
                "rg",
                vec![Object::Real(r), Object::Real(g), Object::Real(b)],
            ),
            Operation::new("BT", vec![]),
            Operation::new(
                "Tf",
                vec![Object::Name(b"WM1".to_vec()), Object::Real(font_size)],
            ),
        ];
        let cid = Object::String(cid_bytes(&chars), StringFormat::Hexadecimal);
        let mut fy = -step_y;
        while fy <= ph + step_y {
            let mut fx = -step_x;
            while fx <= pw + step_x {
                ops.push(Operation::new(
                    "Tm",
                    vec![
                        Object::Real(cos),
                        Object::Real(sin),
                        Object::Real(-sin),
                        Object::Real(cos),
                        Object::Real(mb[0] + fx),
                        Object::Real(mb[1] + fy),
                    ],
                ));
                ops.push(Operation::new("Tj", vec![cid.clone()]));
                fx += step_x;
            }
            fy += step_y;
        }
        ops.push(Operation::new("ET", vec![]));
        ops.push(Operation::new("Q", vec![]));

        let content = Content { operations: ops }
            .encode()
            .map_err(|e| format!("内容流编码失败: {}", e))?;
        let mut stream = Stream::new(Dictionary::new(), content);
        stream.compress().ok();
        let stream_id = doc.add_object(Object::Stream(stream));
        append_page_content(&mut doc, *page_id, stream_id)?;
        ensure_page_resource(&mut doc, *page_id, b"Font", b"WM1", font_id)?;
        ensure_page_resource(&mut doc, *page_id, b"ExtGState", b"GS1", gs_id)?;
    }
    doc.save(out_path).map_err(|e| format!("保存失败: {}", e))?;
    Ok(())
}

// ==================== 页码 ====================

/// 在每页底部居中添加页码（标准 Helvetica 字体，纯数字，无需嵌入）
/// style: "page" 显示「n」，"pageOf" 显示「n / total」
pub fn add_page_numbers(path: &Path, out_path: &Path, style: &str) -> Result<(), String> {
    let mut doc = load_pdf(path)?;
    let font_id = doc.add_object(Object::Dictionary(Dictionary::from_iter(vec![
        (b"Type".to_vec(), Object::Name(b"Font".to_vec())),
        (b"Subtype".to_vec(), Object::Name(b"Type1".to_vec())),
        (b"BaseFont".to_vec(), Object::Name(b"Helvetica".to_vec())),
    ])));

    let pages = doc.get_pages();
    let total = pages.len();
    let fs = 10.0f32;
    for (no, page_id) in pages.iter() {
        let text = if style == "pageOf" {
            format!("{} / {}", no, total)
        } else {
            format!("{}", no)
        };
        let mb = get_page_media_box(&doc, *page_id)?;
        // Helvetica 数字近似宽度 0.55em
        let tw = text.len() as f32 * fs * 0.55;
        let x = mb[0] + (mb[2] - mb[0] - tw) / 2.0;
        let y = mb[1] + 18.0;
        let ops = vec![
            Operation::new("BT", vec![]),
            Operation::new(
                "Tf",
                vec![Object::Name(b"PN1".to_vec()), Object::Real(fs)],
            ),
            Operation::new("g", vec![Object::Real(0.5)]),
            Operation::new(
                "Tm",
                vec![
                    Object::Real(1.0),
                    Object::Real(0.0),
                    Object::Real(0.0),
                    Object::Real(1.0),
                    Object::Real(x),
                    Object::Real(y),
                ],
            ),
            Operation::new(
                "Tj",
                vec![Object::String(text.into_bytes(), StringFormat::Literal)],
            ),
            Operation::new("ET", vec![]),
        ];
        let content = Content { operations: ops }
            .encode()
            .map_err(|e| format!("内容流编码失败: {}", e))?;
        let mut stream = Stream::new(Dictionary::new(), content);
        stream.compress().ok();
        let stream_id = doc.add_object(Object::Stream(stream));
        append_page_content(&mut doc, *page_id, stream_id)?;
        ensure_page_resource(&mut doc, *page_id, b"Font", b"PN1", font_id)?;
    }
    doc.save(out_path).map_err(|e| format!("保存失败: {}", e))?;
    Ok(())
}

// ==================== 旋转 ====================

/// 旋转所有页面（90/180/270 度，累加到已有 /Rotate）
pub fn rotate_pdf(path: &Path, out_path: &Path, angle: i32) -> Result<(), String> {
    let angle = ((angle % 360) + 360) % 360;
    if angle == 0 || angle % 90 != 0 {
        return Err("旋转角度仅支持 90 / 180 / 270 度".into());
    }
    let mut doc = load_pdf(path)?;
    let pages = doc.get_pages();
    for page_id in pages.values() {
        let cur = doc
            .get_dictionary(*page_id)
            .ok()
            .and_then(|p| p.get(b"Rotate").ok().cloned())
            .and_then(|o| o.as_i64().ok())
            .unwrap_or(0);
        // 负数 /Rotate 同样归一化到 0/90/180/270（个别工具会写出不合规负值）
        let new = ((cur + angle as i64) % 360 + 360) % 360;
        if let Ok(page) = doc.get_dictionary_mut(*page_id) {
            page.set("Rotate", Object::Integer(new));
        }
    }
    doc.save(out_path).map_err(|e| format!("保存失败: {}", e))?;
    Ok(())
}

// ==================== 加密 / 解密 ====================

/// PDF 标准加密填充字节（32 字节固定串）
const ENCRYPT_PAD: [u8; 32] = [
    0x28, 0xBF, 0x4E, 0x5E, 0x4E, 0x75, 0x8A, 0x41, 0x64, 0x00, 0x4E, 0x56, 0xFF, 0xFA, 0x01, 0x08,
    0x2E, 0x2E, 0x00, 0xB6, 0xD0, 0x68, 0x3E, 0x80, 0x2F, 0x0C, 0xA9, 0xFE, 0x64, 0x53, 0x69, 0x7A,
];

/// RC4 流密码（KSA + PRGA）
fn rc4(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut s: [u8; 256] = core::array::from_fn(|i| i as u8);
    let mut j = 0usize;
    for i in 0..256 {
        j = (j + s[i] as usize + key[i % key.len()] as usize) % 256;
        s.swap(i, j);
    }
    let mut i = 0usize;
    let mut j = 0usize;
    let mut out = Vec::with_capacity(data.len());
    for &b in data {
        i = (i + 1) % 256;
        j = (j + s[i] as usize) % 256;
        s.swap(i, j);
        let k = s[(s[i] as usize + s[j] as usize) % 256];
        out.push(b ^ k);
    }
    out
}

/// 密码填充/截断到 32 字节（Algorithm 2 步骤 a）
fn pad32(pass: &str) -> Vec<u8> {
    let bytes = pass.as_bytes();
    let n = bytes.len().min(32);
    let mut v = vec![0u8; 32];
    v[..n].copy_from_slice(&bytes[..n]);
    v[n..].copy_from_slice(&ENCRYPT_PAD[..32 - n]);
    v
}

/// 计算文件加密密钥（Algorithm 2，R=3，n 字节）
fn compute_file_key(
    user_pass: &str,
    o: &[u8],
    perms: u32,
    file_id: &[u8],
    key_len: usize,
) -> Vec<u8> {
    let user_pad = pad32(user_pass);
    let mut key = Vec::with_capacity(64);
    key.extend_from_slice(&user_pad);
    key.extend_from_slice(o);
    key.extend_from_slice(&perms.to_le_bytes());
    key.extend_from_slice(file_id);
    let mut digest = Md5::digest(&key).to_vec();
    for _ in 0..50 {
        digest = Md5::digest(&digest).to_vec();
    }
    digest[..key_len].to_vec()
}

/// 计算 O 值（Algorithm 3.3，R=3）
fn compute_o(user_pass: &str, owner_pass: &str, key_len: usize) -> Vec<u8> {
    let mut owner_key = Md5::digest(pad32(owner_pass)).to_vec();
    for _ in 0..50 {
        owner_key = Md5::digest(&owner_key).to_vec();
    }
    let owner_key = owner_key[..key_len].to_vec();
    let user_pad = pad32(user_pass);
    let mut o = rc4(&owner_key, &user_pad);
    for i in 1..=19u8 {
        let k: Vec<u8> = owner_key.iter().map(|b| b ^ i).collect();
        o = rc4(&k, &o);
    }
    o
}

/// 计算 U 值（Algorithm 4，R=3）
fn compute_u(file_key: &[u8], file_id: &[u8]) -> Vec<u8> {
    let mut hasher = Md5::new();
    hasher.update(ENCRYPT_PAD);
    hasher.update(file_id);
    let mut u = rc4(file_key, &hasher.finalize()[..16]);
    for i in 1..=19u8 {
        let k: Vec<u8> = file_key.iter().map(|b| b ^ i).collect();
        u = rc4(&k, &u);
    }
    u.extend_from_slice(&ENCRYPT_PAD[..16]);
    u
}

/// 使用 RC4-128（V=2 / R=3）加密 PDF，设置打开密码（user_pass）与所有者密码（owner_pass）
pub fn encrypt_pdf(
    path: &Path,
    out_path: &Path,
    user_pass: &str,
    owner_pass: &str,
) -> Result<(), String> {
    if user_pass.is_empty() {
        return Err("打开密码不能为空".into());
    }
    let mut doc = Document::load(path).map_err(|e| format!("读取 PDF 失败: {}", e))?;
    if doc.is_encrypted() {
        return Err("该 PDF 已加密，请先移除密码".into());
    }

    // File ID（缺失或畸形时生成新 ID：时间戳 + 路径哈希）
    let file_id: Vec<u8> = match doc
        .trailer
        .get(b"ID")
        .and_then(Object::as_array)
        .ok()
        .and_then(|arr| arr.first())
        .and_then(|o| o.as_str().ok())
        .map(|s| s.to_vec())
    {
        Some(id) => id,
        None => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let mut id = Vec::with_capacity(16);
            id.extend_from_slice(&now.to_le_bytes());
            id.extend_from_slice(&now.to_le_bytes()[..8]);
            let id_obj = Object::String(id.clone(), StringFormat::Hexadecimal);
            doc.trailer.set("ID", Object::Array(vec![id_obj.clone(), id_obj]));
            id
        }
    };

    // 权限：除保留位外全部开放（含打印）
    let perms: u32 = 0xFFFF_FFFC;
    let key_len = 16; // 128 位
    let o = compute_o(user_pass, owner_pass, key_len);
    let file_key = compute_file_key(user_pass, &o, perms, &file_id, key_len);
    let u = compute_u(&file_key, &file_id);

    // 加密字典自身不参与加密
    let encrypt_id = doc.add_object(Object::Dictionary(Dictionary::from_iter(vec![
        (b"Filter".to_vec(), Object::Name(b"Standard".to_vec())),
        (b"V".to_vec(), Object::Integer(2)),
        (b"R".to_vec(), Object::Integer(3)),
        (b"Length".to_vec(), Object::Integer(128)),
        (b"O".to_vec(), Object::String(o, StringFormat::Hexadecimal)),
        (b"U".to_vec(), Object::String(u, StringFormat::Hexadecimal)),
        (b"P".to_vec(), Object::Integer(perms as i64)),
    ])));
    doc.trailer.set("Encrypt", Object::Reference(encrypt_id));

    // 加密所有字符串与流（Algorithm 1：对象密钥 = MD5(file_key || obj# || gen#)）
    for (id, obj) in doc.objects.iter_mut() {
        if *id == encrypt_id {
            continue;
        }
        let mut hasher = Md5::new();
        hasher.update(&file_key);
        let nb = id.0.to_le_bytes();
        hasher.update(&nb[..3]);
        let gb = id.1.to_le_bytes();
        hasher.update(&gb[..2]);
        let digest = hasher.finalize();
        let n = (file_key.len() + 5).min(16);
        let obj_key = &digest[..n];
        match obj {
            Object::Stream(s) => {
                let enc = rc4(obj_key, &s.content);
                s.set_content(enc);
            }
            Object::String(bytes, _) => {
                *bytes = rc4(obj_key, bytes);
            }
            _ => {}
        }
    }
    doc.save(out_path).map_err(|e| format!("保存失败: {}", e))?;
    Ok(())
}

/// 移除 PDF 打开密码（lopdf 支持 RC4 加密文档的 V=1/2、R=2/3）
pub fn decrypt_pdf(path: &Path, out_path: &Path, password: &str) -> Result<(), String> {
    let mut doc = Document::load(path).map_err(|e| format!("读取 PDF 失败: {}", e))?;
    if !doc.is_encrypted() {
        return Err("该 PDF 未加密，无需解密".into());
    }
    doc.decrypt(password.as_bytes()).map_err(|e| match e {
        lopdf::Error::Decryption(lopdf::encryption::DecryptionError::IncorrectPassword) => {
            "密码错误，无法解密".to_string()
        }
        lopdf::Error::Decryption(lopdf::encryption::DecryptionError::UnsupportedEncryption) => {
            "不支持的加密方式（仅支持 RC4 加密的 PDF）".to_string()
        }
        other => format!("解密失败: {}", other),
    })?;
    doc.save(out_path).map_err(|e| format!("保存失败: {}", e))?;
    Ok(())
}

// ==================== 图片转 PDF ====================

/// 解码后的图片数据（按嵌入方式分类）
enum RawImage {
    /// JPEG 原样嵌入（DCTDecode）
    Jpeg {
        w: u32,
        h: u32,
        data: Vec<u8>,
        gray: bool,
    },
    /// 8 位灰度/RGB 无 alpha 的 PNG 原样嵌入（FlateDecode + PNG Predictor 15）
    Png {
        w: u32,
        h: u32,
        idat: Vec<u8>,
        colors: u32,
    },
    /// 其他格式经 image crate 解码为 RGB8（FlateDecode）
    Rgb { w: u32, h: u32, data: Vec<u8> },
}

impl RawImage {
    fn w(&self) -> u32 {
        match self {
            RawImage::Jpeg { w, .. } | RawImage::Png { w, .. } | RawImage::Rgb { w, .. } => *w,
        }
    }
    fn h(&self) -> u32 {
        match self {
            RawImage::Jpeg { h, .. } | RawImage::Png { h, .. } | RawImage::Rgb { h, .. } => *h,
        }
    }

    /// 构造图像 XObject 流
    fn to_xobject(&self) -> Stream {
        match self {
            RawImage::Jpeg { w, h, data, gray } => Stream::new(
                Dictionary::from_iter(vec![
                    (b"Type".to_vec(), Object::Name(b"XObject".to_vec())),
                    (b"Subtype".to_vec(), Object::Name(b"Image".to_vec())),
                    (b"Width".to_vec(), Object::Integer(*w as i64)),
                    (b"Height".to_vec(), Object::Integer(*h as i64)),
                    (
                        b"ColorSpace".to_vec(),
                Object::Name(if *gray {
                    b"DeviceGray".to_vec()
                } else {
                    b"DeviceRGB".to_vec()
                }),
                    ),
                    (b"BitsPerComponent".to_vec(), Object::Integer(8)),
                    (b"Filter".to_vec(), Object::Name(b"DCTDecode".to_vec())),
                ]),
                data.clone(),
            ),
            RawImage::Png { w, h, idat, colors } => Stream::new(
                Dictionary::from_iter(vec![
                    (b"Type".to_vec(), Object::Name(b"XObject".to_vec())),
                    (b"Subtype".to_vec(), Object::Name(b"Image".to_vec())),
                    (b"Width".to_vec(), Object::Integer(*w as i64)),
                    (b"Height".to_vec(), Object::Integer(*h as i64)),
                    (
                        b"ColorSpace".to_vec(),
                        Object::Name(if *colors == 3 {
                            b"DeviceRGB".to_vec()
                        } else {
                            b"DeviceGray".to_vec()
                        }),
                    ),
                    (b"BitsPerComponent".to_vec(), Object::Integer(8)),
                    (b"Filter".to_vec(), Object::Name(b"FlateDecode".to_vec())),
                    (
                        b"DecodeParms".to_vec(),
                        Object::Dictionary(Dictionary::from_iter(vec![
                            (b"Predictor".to_vec(), Object::Integer(15)),
                            (b"Colors".to_vec(), Object::Integer(*colors as i64)),
                            (b"BitsPerComponent".to_vec(), Object::Integer(8)),
                            (b"Columns".to_vec(), Object::Integer(*w as i64)),
                        ])),
                    ),
                ]),
                idat.clone(),
            ),
            RawImage::Rgb { w, h, data } => {
                let mut s = Stream::new(
                    Dictionary::from_iter(vec![
                        (b"Type".to_vec(), Object::Name(b"XObject".to_vec())),
                        (b"Subtype".to_vec(), Object::Name(b"Image".to_vec())),
                        (b"Width".to_vec(), Object::Integer(*w as i64)),
                        (b"Height".to_vec(), Object::Integer(*h as i64)),
                        (b"ColorSpace".to_vec(), Object::Name(b"DeviceRGB".to_vec())),
                        (b"BitsPerComponent".to_vec(), Object::Integer(8)),
                    ]),
                    data.clone(),
                );
                s.compress().ok();
                s
            }
        }
    }
}

/// 解析 JPEG 文件头（SOF 段）以直接嵌入
fn load_jpeg_raw(bytes: &[u8]) -> Option<RawImage> {
    if bytes.len() < 4 || bytes[0] != 0xFF || bytes[1] != 0xD8 {
        return None;
    }
    let mut pos = 2;
    while pos + 4 <= bytes.len() {
        if bytes[pos] != 0xFF {
            pos += 1;
            continue;
        }
        let marker = bytes[pos + 1];
        // SOF0..SOF15（排除 DHT/DAC 等带数据的非 SOF 段）
        if matches!(marker, 0xC0..=0xCF) && !matches!(marker, 0xC4 | 0xC8 | 0xCC) {
            if pos + 10 > bytes.len() {
                return None;
            }
            let h = u16::from_be_bytes([bytes[pos + 5], bytes[pos + 6]]) as u32;
            let w = u16::from_be_bytes([bytes[pos + 7], bytes[pos + 8]]) as u32;
            let ncomp = bytes[pos + 9];
            if w == 0 || h == 0 {
                return None;
            }
            // 仅 1 分量（灰度）/ 3 分量（RGB）可原样嵌入；CMYK(4) 等格式 DCTDecode + DeviceRGB 会色偏，交由解码路径
            if ncomp != 1 && ncomp != 3 {
                return None;
            }
            return Some(RawImage::Jpeg {
                w,
                h,
                data: bytes.to_vec(),
                gray: ncomp == 1,
            });
        }
        if marker == 0xD9 || marker == 0xDA {
            break;
        }
        if marker == 0x01 || (0xD0..=0xD7).contains(&marker) {
            pos += 2;
            continue;
        }
        let seg_len = u16::from_be_bytes([bytes[pos + 2], bytes[pos + 3]]) as usize;
        if seg_len < 2 {
            break;
        }
        pos += 2 + seg_len;
    }
    None
}

/// 解析 8 位灰度/RGB 无 alpha 的 PNG，直接嵌入（保持无损）
fn load_png_raw(bytes: &[u8]) -> Option<RawImage> {
    if bytes.len() < 33 || &bytes[..8] != &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A] {
        return None;
    }
    if &bytes[12..16] != b"IHDR" {
        return None;
    }
    let w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    let bit_depth = bytes[24];
    let color_type = bytes[25];
    // Adam7 隔行扫描的数据结构与 Predictor 15 逐行滤波不符，嵌入会花屏，交由解码路径
    if bytes[28] != 0 {
        return None;
    }
    // 仅支持 8 位灰度(0)/RGB(2)，其他类型走 image crate 解码
    if w == 0 || h == 0 || bit_depth != 8 || !(color_type == 0 || color_type == 2) {
        return None;
    }
    let mut idat = Vec::new();
    let mut pos = 8;
    while pos + 12 <= bytes.len() {
        let len = u32::from_be_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]])
            as usize;
        if pos + 12 + len > bytes.len() {
            break;
        }
        let t = &bytes[pos + 4..pos + 8];
        if t == b"IEND" {
            break;
        }
        if t == b"IDAT" {
            idat.extend_from_slice(&bytes[pos + 8..pos + 8 + len]);
        }
        pos += 12 + len;
    }
    if idat.is_empty() {
        return None;
    }
    Some(RawImage::Png {
        w,
        h,
        idat,
        colors: if color_type == 2 { 3 } else { 1 },
    })
}

/// 读取图片并分类嵌入方式；不支持的格式交由 image crate 解码为 RGB
fn load_image(path: &Path) -> Result<RawImage, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("读取 {} 失败: {}", path.display(), e))?;
    if let Some(img) = load_jpeg_raw(&bytes) {
        return Ok(img);
    }
    if let Some(img) = load_png_raw(&bytes) {
        return Ok(img);
    }
    let img = image::load_from_memory(&bytes)
        .map_err(|e| format!("无法识别的图片格式 {}: {}", path.display(), e))?;
    // 透明像素与白底做 alpha 混合（PDF 无透明画布概念，直接丢弃 alpha 会显示黑块）
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let mut data = Vec::with_capacity((w as usize) * (h as usize) * 3);
    for p in rgba.pixels() {
        let a = p[3] as u32;
        for c in 0..3 {
            data.push(((p[c] as u32 * a + 255 * (255 - a)) / 255) as u8);
        }
    }
    Ok(RawImage::Rgb { w, h, data })
}

/// 多张图片合成一个 PDF；page_size 为 "a4" 时 A4 居中，否则页面尺寸跟随图片像素
pub fn images_to_pdf(paths: &[PathBuf], out_path: &Path, page_size: &str) -> Result<(), String> {
    if paths.is_empty() {
        return Err("至少需要选择一张图片".into());
    }
    let mut doc = Document::with_version("1.4");
    let mut page_ids: Vec<ObjectId> = Vec::new();
    for p in paths {
        let raw = load_image(p)?;
        let iw = raw.w() as f32;
        let ih = raw.h() as f32;
        // 页面尺寸与缩放：A4 模式等比缩放居中；auto 模式 1px = 1pt，超 14400 时缩放（PDF 上限）
        let (pw, ph, scale) = if page_size == "a4" {
            let (w, h) = if iw > ih { (842.0, 595.0) } else { (595.0, 842.0) };
            (w, h, (w / iw).min(h / ih).min(1.0))
        } else {
            let max = iw.max(ih);
            let s = if max > 14400.0 { 14400.0 / max } else { 1.0 };
            (iw * s, ih * s, s)
        };

        let xobj_id = doc.add_object(Object::Stream(raw.to_xobject()));
        let dw = iw * scale;
        let dh = ih * scale;
        let x = (pw - dw) / 2.0;
        let y = (ph - dh) / 2.0;
        let ops = vec![
            Operation::new("q", vec![]),
            Operation::new(
                "cm",
                vec![
                    Object::Real(dw),
                    Object::Real(0.0),
                    Object::Real(0.0),
                    Object::Real(dh),
                    Object::Real(x),
                    Object::Real(y),
                ],
            ),
            Operation::new("Do", vec![Object::Name(b"Im0".to_vec())]),
            Operation::new("Q", vec![]),
        ];
        let content = Content { operations: ops }
            .encode()
            .map_err(|e| format!("内容流编码失败: {}", e))?;
        let mut stream = Stream::new(Dictionary::new(), content);
        stream.compress().ok();
        let content_id = doc.add_object(Object::Stream(stream));

        let page = Dictionary::from_iter(vec![
            (b"Type".to_vec(), Object::Name(b"Page".to_vec())),
            (
                b"MediaBox".to_vec(),
                Object::Array(vec![
                    Object::Integer(0),
                    Object::Integer(0),
                    Object::Real(pw),
                    Object::Real(ph),
                ]),
            ),
            (
                b"Resources".to_vec(),
                Object::Dictionary(Dictionary::from_iter(vec![(
                    b"XObject".to_vec(),
                    Object::Dictionary(Dictionary::from_iter(vec![(
                        b"Im0".to_vec(),
                        Object::Reference(xobj_id),
                    )])),
                )])),
            ),
            (b"Contents".to_vec(), Object::Reference(content_id)),
        ]);
        page_ids.push(doc.add_object(Object::Dictionary(page)));
    }

    // 重建根页树并挂载全部页面（与 merge_pdfs 相同模式）
    let kids: Vec<Object> = page_ids.iter().map(|id| Object::Reference(*id)).collect();
    let pages = Dictionary::from_iter(vec![
        (b"Type".to_vec(), Object::Name(b"Pages".to_vec())),
        (b"Kids".to_vec(), Object::Array(kids)),
        (b"Count".to_vec(), Object::Integer(page_ids.len() as i64)),
    ]);
    let pages_id = doc.add_object(pages);
    // PDF 规范要求每个 Page 必须有 /Parent，缺失会导致 Apple 预览等渲染器拒绝打开
    for pid in &page_ids {
        if let Ok(page) = doc.get_object_mut(*pid) {
            if let Ok(d) = page.as_dict_mut() {
                d.set("Parent", Object::Reference(pages_id));
            }
        }
    }
    let catalog_id = match doc.trailer.get(b"Root").and_then(|o| o.as_reference()) {
        Ok(id) => id,
        Err(_) => {
            let catalog = Dictionary::from_iter(vec![(
                b"Type".to_vec(),
                Object::Name(b"Catalog".to_vec()),
            )]);
            let id = doc.add_object(Object::Dictionary(catalog));
            doc.trailer.set("Root", Object::Reference(id));
            id
        }
    };
    if let Ok(catalog) = doc.get_dictionary_mut(catalog_id) {
        catalog.set("Pages", Object::Reference(pages_id));
    }
    doc.save(out_path).map_err(|e| format!("保存失败: {}", e))?;
    Ok(())
}

/* ---------- PDF 元数据编辑 ---------- */

/// 设置 PDF 文档元数据（Info 字典条目），None 表示保留原值
pub fn pdf_metadata(
    path: &Path,
    out_path: &Path,
    title: Option<&str>,
    author: Option<&str>,
    subject: Option<&str>,
    keywords: Option<&str>,
) -> Result<(), String> {
    let mut doc = load_pdf(path)?;
    let info_id = match doc.trailer.get(b"Info").and_then(|o| o.as_reference()) {
        Ok(id) => id,
        Err(_) => {
            let info = Dictionary::new();
            let info_ref = doc.add_object(info);
            doc.trailer.set("Info", Object::Reference(info_ref));
            info_ref
        }
    };
    if let Ok(info) = doc.get_dictionary_mut(info_id) {
        if let Some(v) = title { info.set("Title", Object::string_literal(v)); }
        if let Some(v) = author { info.set("Author", Object::string_literal(v)); }
        if let Some(v) = subject { info.set("Subject", Object::string_literal(v)); }
        if let Some(v) = keywords { info.set("Keywords", Object::string_literal(v)); }
        // 更新修改日期
        let now = chrono_now();
        info.set("ModDate", Object::string_literal(now.clone()));
    }
    doc.save(out_path).map_err(|e| format!("保存失败: {}", e))?;
    Ok(())
}

/// 获取当前时间字符串（PDF 日期格式 D:YYYYMMDDHHmmSS），不依赖 chrono
fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let total_secs = d.as_secs();
    let days = total_secs / 86400;
    let time_secs = total_secs % 86400;
    let h = time_secs / 3600;
    let m = (time_secs % 3600) / 60;
    let s = time_secs % 60;
    // 从 1970-01-01 推算年/月/日
    let mut y = 1970i64;
    let mut rem = days as i64;
    loop {
        let diy = if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 { 366 } else { 365 };
        if rem < diy { break; }
        rem -= diy;
        y += 1;
    }
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let month_days: &[i64] = if leap { &[31,29,31,30,31,30,31,31,30,31,30,31] } else { &[31,28,31,30,31,30,31,31,30,31,30,31] };
    let mut mo = 1u32;
    for &md in month_days {
        if rem < md { break; }
        rem -= md;
        mo += 1;
    }
    let day = rem + 1;
    // 计算本地时区偏移（分钟）
    let tz_offset_mins = local_tz_offset();
    let tz_sign = if tz_offset_mins >= 0 { '+' } else { '-' };
    let tz_abs = tz_offset_mins.abs();
    format!("D:{:04}{:02}{:02}{:02}{:02}{:02}{}{:02}'{:02}'", y, mo, day, h, m, s, tz_sign, tz_abs / 60, tz_abs % 60)
}

/// 获取本地时区偏移（分钟），正数表示东
fn local_tz_offset() -> i32 {
    // 在 macOS/Linux 上通过 libc 获取 localtime 的 gmtoff
    #[cfg(target_family = "unix")]
    unsafe {
        let mut now: i64 = 0;
        let mut tm = std::mem::zeroed();
        libc::time(&mut now as *mut i64);
        libc::localtime_r(&now, &mut tm);
        (tm.tm_gmtoff / 60) as i32
    }
    #[cfg(not(target_family = "unix"))]
    0
}

/* ---------- PDF 裁剪页面 ---------- */

/// 裁剪 PDF 页面：所有页面统一设置新的 MediaBox（左、下、右、上，PDF 单位 pt）
pub fn pdf_crop(
    path: &Path,
    out_path: &Path,
    left: f32,
    bottom: f32,
    right: f32,
    top: f32,
) -> Result<(), String> {
    let mut doc = load_pdf(path)?;
    // 参数校验：右边界必须大于左边界、上边界必须大于下边界，否则生成无效 MediaBox
    if right <= left || top <= bottom {
        return Err("裁剪参数无效：右边界必须大于左边界，上边界必须大于下边界".to_string());
    }
    let pages = doc.get_pages();
    let page_ids: Vec<ObjectId> = pages.values().copied().collect();
    for id in &page_ids {
        if let Ok(page) = doc.get_dictionary_mut(*id) {
            page.set("MediaBox", Object::Array(vec![
                Object::Real(left),
                Object::Real(bottom),
                Object::Real(right),
                Object::Real(top),
            ]));
        }
    }
    doc.save(out_path).map_err(|e| format!("保存失败: {}", e))?;
    Ok(())
}

/* ---------- PDF 书签/大纲 ---------- */

/// 添加 PDF 书签（大纲），items 为 (标题, 目标页码) 列表，页码 1 起
pub fn pdf_outline(
    path: &Path,
    out_path: &Path,
    items: &[(String, u32)],
) -> Result<(), String> {
    if items.is_empty() {
        return Err("至少需要一个书签项".to_string());
    }
    let mut doc = load_pdf(path)?;
    let pages = doc.get_pages();
    let total = pages.len() as u32;
    let page_ids: Vec<ObjectId> = pages.values().copied().collect();

    // 构建书签条目（Outline item）
    let mut outline_items: Vec<ObjectId> = Vec::with_capacity(items.len());
    for (_i, (title, page_num)) in items.iter().enumerate() {
        let pn = (*page_num).max(1).min(total) as usize - 1;
        let target_page = page_ids.get(pn).copied().unwrap_or(page_ids[0]);
        let mut entry = Dictionary::from_iter(vec![
            (b"Title".to_vec(), Object::string_literal(title.clone())),
            (b"Parent".to_vec(), Object::Null),        // 稍后填充
            (b"Prev".to_vec(), Object::Null),
            (b"Next".to_vec(), Object::Null),
        ]);
        // 设置目的页：/Dest [page /XYZ left top null]
        entry.set("Dest", Object::Array(vec![
            Object::Reference(target_page),
            Object::Name(b"XYZ".to_vec()),
            Object::Null,
            Object::Null,
            Object::Null,
        ]));
        let id = doc.add_object(Object::Dictionary(entry));
        outline_items.push(id);
    }

    // 链接 Prev/Next
    for i in 0..outline_items.len() {
        if let Ok(e) = doc.get_dictionary_mut(outline_items[i]) {
            if i > 0 { e.set("Prev", Object::Reference(outline_items[i - 1])); }
            if i + 1 < outline_items.len() { e.set("Next", Object::Reference(outline_items[i + 1])); }
        }
    }

    // 创建顶级 Outline 字典
    let first = outline_items.first().copied().unwrap();
    let last = outline_items.last().copied().unwrap();
    let outline = Dictionary::from_iter(vec![
        (b"Type".to_vec(), Object::Name(b"Outlines".to_vec())),
        (b"First".to_vec(), Object::Reference(first)),
        (b"Last".to_vec(), Object::Reference(last)),
        (b"Count".to_vec(), Object::Integer(outline_items.len() as i64)),
    ]);
    let outline_id = doc.add_object(Object::Dictionary(outline));

    // 顶层条目 Parent 指向 Outlines 字典（PDF 规范要求）
    for id in &outline_items {
        if let Ok(e) = doc.get_dictionary_mut(*id) {
            e.set("Parent", Object::Reference(outline_id));
        }
    }

    // 更新 Catalog
    let catalog_id = match doc.trailer.get(b"Root").and_then(|o| o.as_reference()) {
        Ok(id) => id,
        Err(_) => {
            let cat = Dictionary::from_iter(vec![(b"Type".to_vec(), Object::Name(b"Catalog".to_vec()))]);
            let id = doc.add_object(Object::Dictionary(cat));
            doc.trailer.set("Root", Object::Reference(id));
            id
        }
    };
    if let Ok(catalog) = doc.get_dictionary_mut(catalog_id) {
        catalog.set("Outlines", Object::Reference(outline_id));
    }

    doc.save(out_path).map_err(|e| format!("保存失败: {}", e))?;
    Ok(())
}

/* ---------- 图片压缩（配合图片转 PDF） ---------- */

/// 压缩图片文件（变小才覆盖原文件）；quality 1~100，越高画质越好。
/// 返回 true 表示已重写原文件，false 表示重压无收益、保留原文件。
pub fn image_compress(path: &Path, quality: u8) -> Result<bool, String> {
    let q = quality.max(1).min(100);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    let img = image::open(path).map_err(|e| format!("读取图片失败: {}", e))?;
    // 先写临时文件，成功后再重命名覆盖原文件，避免编码失败时原文件被截断
    let tmp_path = path.with_extension(format!("tmp-{}", std::process::id()));
    let encode = |f: &mut std::fs::File| -> Result<(), String> {
        match ext.as_str() {
            "jpg" | "jpeg" => {
                // 使用 JPEG 编码器指定 quality
                let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(f, q);
                let rgb = img.to_rgb8();
                let (w, h) = rgb.dimensions();
                encoder
                    .write_image(&rgb, w, h, image::ExtendedColorType::Rgb8)
                    .map_err(|e| format!("编码 JPEG 失败: {}", e))
            }
            "png" => {
                // PNG 使用最高压缩率 + 自适应滤波；无 alpha 时转 RGB 减少通道数
                let encoder = image::codecs::png::PngEncoder::new_with_quality(
                    f,
                    image::codecs::png::CompressionType::Best,
                    image::codecs::png::FilterType::Adaptive,
                );
                let (w, h, px, color) = if img.color().has_alpha() {
                    let rgba = img.to_rgba8();
                    let (w, h) = rgba.dimensions();
                    (w, h, rgba.into_raw(), image::ExtendedColorType::Rgba8)
                } else {
                    let rgb = img.to_rgb8();
                    let (w, h) = rgb.dimensions();
                    (w, h, rgb.into_raw(), image::ExtendedColorType::Rgb8)
                };
                encoder
                    .write_image(&px, w, h, color)
                    .map_err(|e| format!("编码 PNG 失败: {}", e))
            }
            _ => Err(format!("不支持的图片格式: {}", ext)),
        }
    };
    {
        let mut f = std::fs::File::create(&tmp_path).map_err(|e| format!("创建文件失败: {}", e))?;
        if let Err(e) = encode(&mut f) {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(e);
        }
    } // f 在此 drop，确保写入完成
    // 无损重压未必能变小（PNG 已压缩过）：变小才覆盖，否则保留原文件
    let tmp_len = std::fs::metadata(&tmp_path).map(|m| m.len()).unwrap_or(0);
    let orig_len = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    if tmp_len < orig_len {
        std::fs::rename(&tmp_path, path).map_err(|e| {
            let _ = std::fs::remove_file(&tmp_path);
            format!("保存失败: {}", e)
        })?;
        Ok(true)
    } else {
        let _ = std::fs::remove_file(&tmp_path);
        Ok(false)
    }
}

/* ---------- 提取 PDF 嵌入图片 ---------- */

/// 解码图像流数据（lopdf 的 decompressed_content 会拒绝 Subtype=Image 流，需手动解码）
fn decode_image_stream(stream: &Stream) -> Result<Vec<u8>, String> {
    let filter = stream
        .dict
        .get(b"Filter")
        .ok()
        .and_then(|o| o.as_name().ok());
    match filter {
        Some(b"FlateDecode") => {
            use flate2::read::ZlibDecoder;
            use std::io::Read;
            let mut out = Vec::new();
            let mut dec = ZlibDecoder::new(&stream.content[..]);
            dec.read_to_end(&mut out)
                .map_err(|e| format!("zlib 解码失败: {}", e))?;
            // Predictor 10-15（PNG 逐行滤波）：还原真实像素行
            if let Some(params) = stream
                .dict
                .get(b"DecodeParms")
                .ok()
                .and_then(|o| o.as_dict().ok())
            {
                let predictor = params
                    .get(b"Predictor")
                    .and_then(|o| o.as_i64())
                    .unwrap_or(1);
                if (10..=15).contains(&predictor) {
                    let cols = params
                        .get(b"Columns")
                        .and_then(|o| o.as_i64())
                        .unwrap_or(1) as usize;
                    let colors = params
                        .get(b"Colors")
                        .and_then(|o| o.as_i64())
                        .unwrap_or(1) as usize;
                    let bits = params
                        .get(b"BitsPerComponent")
                        .and_then(|o| o.as_i64())
                        .unwrap_or(8) as usize;
                    // bpp 向上取整：BitsPerComponent=4、Colors=1 时 4/8 截断为 0 会解码异常
                    out = lopdf::filters::png::decode_frame(&out, (colors * bits + 7) / 8, cols)
                        .map_err(|e| format!("PNG 滤波解码失败: {}", e))?;
                }
            }
            Ok(out)
        }
        None => Ok(stream.content.clone()),
        _ => Err("不支持的图片过滤器".into()),
    }
}

/// 提取 PDF 中嵌入的图片到输出目录，返回图片路径列表
pub fn extract_pdf_images(path: &Path, out_dir: &Path) -> Result<Vec<String>, String> {
    let doc = load_pdf(path)?;
    std::fs::create_dir_all(out_dir)
        .map_err(|e| format!("创建输出目录失败: {}", e))?;

    let mut outputs = Vec::new();
    let mut img_idx = 0u32;

    for (_id, obj) in &doc.objects {
        let stream = match obj {
            Object::Stream(s) => s,
            _ => continue,
        };
        let is_image = stream
            .dict
            .get(b"Subtype").ok()
            .and_then(|o| o.as_name().ok())
            .map(|n| n == b"Image")
            .unwrap_or(false);
        if !is_image {
            continue;
        }

        let filter = stream.dict.get(b"Filter").ok().and_then(|o| o.as_name().ok());

        let (ext, data) = match filter {
            Some(b"DCTDecode") => {
                // JPEG 图片：直接使用原始流数据
                ("jpg", stream.content.clone())
            }
            Some(b"FlateDecode") | None => {
                // FlateDecode 压缩或未压缩：手动解码后编码为 PNG（lopdf 拒绝解码 Image 流）
                let decoded = decode_image_stream(stream)?;
                let w = stream
                    .dict
                    .get(b"Width")
                    .and_then(|o| o.as_i64())
                    .unwrap_or(0) as u32;
                let h = stream
                    .dict
                    .get(b"Height")
                    .and_then(|o| o.as_i64())
                    .unwrap_or(0) as u32;
                if w == 0 || h == 0 {
                    continue;
                }
                let cs = stream.dict.get(b"ColorSpace").ok().and_then(|o| o.as_name().ok());

                let mut buf = Vec::new();
                match cs {
                    Some(b"DeviceRGB") => {
                        let img = image::RgbImage::from_raw(w, h, decoded)
                            .ok_or("无法解析 RGB 图片数据")?;
                        let encoder = image::codecs::png::PngEncoder::new(&mut buf);
                        encoder
                            .write_image(&img, w, h, image::ExtendedColorType::Rgb8)
                            .map_err(|e| format!("编码 PNG 失败: {}", e))?;
                    }
                    Some(b"DeviceGray") => {
                        let img = image::GrayImage::from_raw(w, h, decoded)
                            .ok_or("无法解析灰度图片数据")?;
                        let encoder = image::codecs::png::PngEncoder::new(&mut buf);
                        encoder
                            .write_image(&img, w, h, image::ExtendedColorType::L8)
                            .map_err(|e| format!("编码 PNG 失败: {}", e))?;
                    }
                    Some(b"DeviceCMYK") => {
                        let rgb: Vec<u8> = decoded
                            .chunks(4)
                            .flat_map(|cmyk: &[u8]| {
                                if cmyk.len() < 4 {
                                    return vec![0, 0, 0];
                                }
                                let c = cmyk[0] as f32 / 255.0;
                                let m = cmyk[1] as f32 / 255.0;
                                let y = cmyk[2] as f32 / 255.0;
                                let k = cmyk[3] as f32 / 255.0;
                                vec![
                                    (255.0 * (1.0 - c) * (1.0 - k)) as u8,
                                    (255.0 * (1.0 - m) * (1.0 - k)) as u8,
                                    (255.0 * (1.0 - y) * (1.0 - k)) as u8,
                                ]
                            })
                            .collect();
                        let img = image::RgbImage::from_raw(w, h, rgb)
                            .ok_or("无法解析 CMYK 图片数据")?;
                        let encoder = image::codecs::png::PngEncoder::new(&mut buf);
                        encoder
                            .write_image(&img, w, h, image::ExtendedColorType::Rgb8)
                            .map_err(|e| format!("编码 PNG 失败: {}", e))?;
                    }
                    _ => continue,
                }
                ("png", buf)
            }
            _ => continue,
        };

        img_idx += 1;
        let fname = format!("img_{:03}.{}", img_idx, ext);
        let out_path = out_dir.join(&fname);
        std::fs::write(&out_path, &data)
            .map_err(|e| format!("保存图片失败: {}", e))?;
        outputs.push(out_path.to_string_lossy().to_string());
    }

    if outputs.is_empty() {
        return Err("PDF 中未找到嵌入图片".to_string());
    }
    Ok(outputs)
}

/* ---------- 移除 PDF 水印（Stamp 注解） ---------- */

/// 移除 PDF 中的水印（Stamp 类型注解）
pub fn remove_watermark(path: &Path, out_path: &Path) -> Result<(), String> {
    let mut doc = load_pdf(path)?;
    let pages = doc.get_pages();
    let mut removed = 0u32;

    for (_page_no, page_id) in &pages {
        let page_obj = doc.get_object(*page_id)
            .map_err(|e| format!("获取页面对象失败: {}", e))?;
        let dict = match page_obj {
            Object::Dictionary(d) => d,
            _ => continue,
        };

        if let Some(annots_obj) = dict.get(b"Annots").ok() {
            let annots = resolve_pdf_array(&doc, annots_obj);
            let mut kept = Vec::new();

            for annot_ref in &annots {
                let is_stamp = match annot_ref {
                    Object::Reference(id) => doc
                        .get_object(*id)
                        .ok()
                        .and_then(|o| o.as_dict().ok())
                        .and_then(|d| d.get(b"Subtype").ok().and_then(|s| s.as_name().ok()))
                        .map(|n| n == b"Stamp")
                        .unwrap_or(false),
                    _ => false,
                };

                if is_stamp {
                    removed += 1;
                } else {
                    kept.push(annot_ref.clone());
                }
            }

            if removed > 0 {
                if let Ok(page_obj) = doc.get_object_mut(*page_id) {
                    if let Ok(dict) = page_obj.as_dict_mut() {
                        if kept.is_empty() {
                            dict.remove(b"Annots");
                        } else {
                            dict.set("Annots", Object::Array(kept));
                        }
                    }
                }
            }
        }
    }

    if removed == 0 {
        return Err("未检测到水印注解".to_string());
    }

    doc.save(out_path)
        .map_err(|e| format!("保存 PDF 失败: {}", e))?;
    Ok(())
}

/// 解析 PDF 数组（跟踪间接引用）
fn resolve_pdf_array(doc: &Document, obj: &Object) -> Vec<Object> {
    match obj {
        Object::Array(arr) => arr.clone(),
        Object::Reference(id) => doc
            .get_object(*id)
            .ok()
            .map(|o| resolve_pdf_array(doc, o))
            .unwrap_or_default(),
        _ => vec![],
    }
}

/* ---------- PDF 文本比较 ---------- */

/// PDF 文本比较结果条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffEntry {
    pub status: String, // "added" | "removed" | "unchanged"
    pub line: String,
    pub line_a: u32, // 第一个文件中的行号
    pub line_b: u32, // 第二个文件中的行号
}

/// 比较两个 PDF 的内容差异（基于文本提取）
pub fn compare_pdfs(path1: &Path, path2: &Path) -> Result<Vec<DiffEntry>, String> {
    let text1 = pdf_extract_text(path1)?;
    let text2 = pdf_extract_text(path2)?;

    let lines1: Vec<&str> = text1.lines().collect();
    let lines2: Vec<&str> = text2.lines().collect();

    let mut result = Vec::new();
    let max = lines1.len().max(lines2.len());

    for i in 0..max {
        let l1 = lines1.get(i).copied().unwrap_or("");
        let l2 = lines2.get(i).copied().unwrap_or("");

        if l1 == l2 && !l1.is_empty() {
            result.push(DiffEntry {
                status: "unchanged".to_string(),
                line: l1.to_string(),
                line_a: (i + 1) as u32,
                line_b: (i + 1) as u32,
            });
        } else {
            if !l1.is_empty() {
                result.push(DiffEntry {
                    status: "removed".to_string(),
                    line: l1.to_string(),
                    line_a: (i + 1) as u32,
                    line_b: 0,
                });
            }
            if !l2.is_empty() {
                result.push(DiffEntry {
                    status: "added".to_string(),
                    line: l2.to_string(),
                    line_a: 0,
                    line_b: (i + 1) as u32,
                });
            }
        }
    }

    if result.is_empty() {
        result.push(DiffEntry {
            status: "unchanged".to_string(),
            line: "(空文档)".to_string(),
            line_a: 0,
            line_b: 0,
        });
    }

    Ok(result)
}

/// 从 PDF 页面内容流中提取文本（基于 lopdf Content 解析器）
pub fn pdf_extract_text(path: &Path) -> Result<String, String> {
    let doc = load_pdf(path)?;
    let pages = doc.get_pages();
    let mut full_text = String::new();

    for (_page_no, page_id) in &pages {
        let page_obj = doc.get_object(*page_id)
            .map_err(|e| format!("获取页面对象失败: {}", e))?;
        let dict = match page_obj {
            Object::Dictionary(d) => d,
            _ => continue,
        };

        let content_data = get_page_content(&doc, dict)?;
        if content_data.is_empty() {
            continue;
        }

        if let Ok(content) = Content::decode(&content_data) {
            for op in &content.operations {
                match op.operator.as_str() {
                    "Tj" => {
                        if let Some(Object::String(s, _)) = op.operands.first() {
                            full_text.push_str(&String::from_utf8_lossy(s));
                        }
                    }
                    "TJ" => {
                        for operand in &op.operands {
                            if let Object::Array(arr) = operand {
                                for item in arr {
                                    if let Object::String(s, _) = item {
                                        full_text.push_str(
                                            &String::from_utf8_lossy(s),
                                        );
                                    }
                                }
                            }
                        }
                    }
                    "'" | "\"" => {
                        if let Some(Object::String(s, _)) = op.operands.last() {
                            full_text.push('\n');
                            full_text.push_str(&String::from_utf8_lossy(s));
                        }
                    }
                    _ => {}
                }
            }
        }
        full_text.push('\n');
    }

    Ok(full_text)
}

/// 获取页面内容流数据（处理间接引用和数组）
fn get_page_content(doc: &Document, dict: &Dictionary) -> Result<Vec<u8>, String> {
    if let Some(contents) = dict.get(b"Contents").ok() {
        match contents {
            Object::Stream(stream) => decode_content_stream(stream),
            Object::Reference(id) => {
                if let Ok(obj) = doc.get_object(*id) {
                    if let Object::Stream(stream) = obj {
                        return decode_content_stream(stream);
                    }
                }
                Ok(vec![])
            }
            Object::Array(arr) => {
                let mut all = Vec::new();
                for item in arr {
                    if let Object::Reference(id) = item {
                        if let Ok(obj) = doc.get_object(*id) {
                            if let Object::Stream(stream) = obj {
                                if let Ok(d) = decode_content_stream(stream) {
                                    all.extend_from_slice(&d);
                                }
                            }
                        }
                    }
                }
                Ok(all)
            }
            _ => Ok(vec![]),
        }
    } else {
        Ok(vec![])
    }
}

/// 解码内容流（兼容无 Filter 的明文流与 FlateDecode 压缩流；
/// lopdf 的 decompressed_content 对无 Filter 键的流会报 "required dictionary key"）
fn decode_content_stream(stream: &Stream) -> Result<Vec<u8>, String> {
    let filter = stream.dict.get(b"Filter").ok();
    let names = match filter {
        None => return Ok(stream.content.clone()),
        Some(f) => {
            if let Ok(n) = f.as_name() {
                vec![n.to_vec()]
            } else if let Ok(arr) = f.as_array() {
                arr.iter().filter_map(|o| o.as_name().ok()).map(|n| n.to_vec()).collect()
            } else {
                return Ok(stream.content.clone());
            }
        }
    };
    let mut out = stream.content.clone();
    for name in names {
        if name == b"FlateDecode" {
            use flate2::read::ZlibDecoder;
            use std::io::Read;
            let mut buf = Vec::new();
            let mut dec = ZlibDecoder::new(&out[..]);
            dec.read_to_end(&mut buf)
                .map_err(|e| format!("解码内容流失败: {}", e))?;
            out = buf;
        } else {
            // 未知过滤器（如 DCTDecode 内容流）：保持原样返回
            return Ok(out);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// 生成 N 页空白 A4 PDF 作为测试样例（消除对原作者本机样例文件的依赖）
    fn make_blank_pages(path: &std::path::Path, n: usize) {
        let mut kids = String::new();
        let mut offsets: Vec<usize> = vec![0; n * 2 + 3];
        let mut body = String::new();
        let header = "%PDF-1.4\n";
        let add = |body: &mut String, offsets: &mut Vec<usize>, id: usize, content: String| {
            offsets[id] = header.len() + body.len();
            body.push_str(&content);
        };
        for i in 0..n {
            kids.push_str(&format!("{} 0 R ", i * 2 + 3));
        }
        add(&mut body, &mut offsets, 1, format!(
            "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n"
        ));
        add(&mut body, &mut offsets, 2, format!(
            "2 0 obj\n<< /Type /Pages /Kids [{}] /Count {} >>\nendobj\n", kids.trim_end(), n
        ));
        for i in 0..n {
            add(&mut body, &mut offsets, i * 2 + 3, format!(
                "{id} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] >>\nendobj\n", id = i * 2 + 3
            ));
        }
        let xref_off = header.len() + body.len();
        let mut xref = format!("xref\n0 {}\n0000000000 65535 f \n", n * 2 + 3);
        for id in 1..n * 2 + 3 {
            xref.push_str(&format!("{:010} 00000 n \n", offsets[id]));
        }
        let content = format!(
            "{}{}{}\ntrailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            header, body, xref, n * 2 + 3, xref_off
        );
        fs::write(path, content).unwrap();
    }

    /** 测试样例：运行时生成 6 页空白 PDF（原硬编码 /Users/yjz1/Downloads/... 依赖原作者本机文件，不可移植） */
    fn sample_pdf_path(out_dir: &std::path::Path) -> std::path::PathBuf {
        let p = out_dir.join("sample_source.pdf");
        make_blank_pages(&p, 6);
        p
    }

    #[test]
    fn test_split_and_compress() {
        let out_dir = PathBuf::from("/tmp/pdf_tool_test");
        let _ = fs::remove_dir_all(&out_dir);
        fs::create_dir_all(&out_dir).unwrap();
        let sample = sample_pdf_path(&out_dir);

        // 原文件页数
        let src = Document::load(&sample).unwrap();
        let total = src.get_pages().len() as u32;
        assert!(total >= 4, "样例 PDF 应至少 4 页，实际 {}", total);

        // 拆分 [1,2] [3,4]
        let outs = split_pdf(Path::new(&sample), &[(1, 2), (3, 4)], &out_dir, "split").unwrap();
        assert_eq!(outs.len(), 2);
        assert_eq!(outs[0].file_name().unwrap().to_str().unwrap(), "split_1-2.pdf");
        assert_eq!(outs[1].file_name().unwrap().to_str().unwrap(), "split_3-4.pdf");
        for o in outs.iter() {
            let d = Document::load(o).unwrap();
            assert_eq!(d.get_pages().len(), 2usize);
        }

        // 合并前两个拆分结果
        let merged = out_dir.join("merged.pdf");
        merge_pdfs(&outs, &merged).unwrap();
        let dm = Document::load(&merged).unwrap();
        assert_eq!(dm.get_pages().len(), 4usize, "合并后应为 4 页");

        // 压缩
        let comp = out_dir.join("compressed.pdf");
        compress_pdf(Path::new(&sample), &comp).unwrap();
        assert!(comp.exists());
        let dc = Document::load(&comp).unwrap();
        assert_eq!(dc.get_pages().len(), total as usize, "压缩后页数不变");

        println!("✅ 拆分/合并/压缩全部通过（原文件 {} 页）", total);
    }

    #[test]
    fn test_extended_pdf_tools() {
        let out_dir = PathBuf::from("/tmp/pdf_tool_test_ext");
        let _ = fs::remove_dir_all(&out_dir);
        fs::create_dir_all(&out_dir).unwrap();
        let sample = sample_pdf_path(&out_dir);
        let total = Document::load(&sample).unwrap().get_pages().len() as u32;

        // 旋转 90°：/Rotate 属性应为 90，页数不变
        let rotated = out_dir.join("rotated.pdf");
        rotate_pdf(Path::new(&sample), &rotated, 90).unwrap();
        let dr = Document::load(&rotated).unwrap();
        assert_eq!(dr.get_pages().len() as u32, total, "旋转后页数不变");
        let first = dr.get_pages().values().next().copied().unwrap();
        let rot = dr
            .get_dictionary(first)
            .unwrap()
            .get(b"Rotate")
            .unwrap()
            .as_i64()
            .unwrap();
        assert_eq!(rot, 90, "/Rotate 应为 90");

        // 页码：页数不变，页面 Resources 应包含 PN1 字体
        let numbered = out_dir.join("numbered.pdf");
        add_page_numbers(Path::new(&sample), &numbered, "pageOf").unwrap();
        let dn = Document::load(&numbered).unwrap();
        assert_eq!(dn.get_pages().len() as u32, total, "页码后页数不变");

        // 水印：依赖系统字体；失败（如无字体）时跳过而非报错
        let watermarked = out_dir.join("watermarked.pdf");
        match add_watermark(Path::new(&sample), &watermarked, "DocMorph 内部资料", 0.2, (0.5, 0.5, 0.5), 26.0) {
            Ok(()) => {
                let dw = Document::load(&watermarked).unwrap();
                assert_eq!(dw.get_pages().len() as u32, total, "水印后页数不变");
                let first = dw.get_pages().values().next().copied().unwrap();
                let res = dw.get_dictionary(first).unwrap();
                assert!(
                    res.get(b"Resources").ok().is_some(),
                    "水印后页面应注入 Resources"
                );
                println!("✅ 水印生成成功");
            }
            Err(e) => eprintln!("⚠️ 跳过水印测试: {}", e),
        }

        // 加密：正确密码可打开，错误密码失败，解密产物无加密标记
        let enc = out_dir.join("encrypted.pdf");
        encrypt_pdf(Path::new(&sample), &enc, "123456", "owner888").unwrap();
        let mut de = Document::load(&enc).unwrap();
        de.decrypt(b"123456").unwrap();
        assert_eq!(de.get_pages().len() as u32, total, "加密后正确密码可打开");
        let mut de2 = Document::load(&enc).unwrap();
        assert!(
            de2.decrypt(b"wrong-pass").is_err(),
            "错误密码应解密失败"
        );
        let dec = out_dir.join("decrypted.pdf");
        decrypt_pdf(&enc, &dec, "123456").unwrap();
        let dd = Document::load(&dec).unwrap();
        assert!(!dd.is_encrypted(), "解密产物不应有加密标记");
        assert_eq!(dd.get_pages().len() as u32, total, "解密后页数不变");
        // 对未加密文档解密应报错
        assert!(decrypt_pdf(Path::new(&sample), &dec, "x").is_err());
        println!("✅ 加密/解密全部通过（RC4-128）");

        // 图片转 PDF：生成 32x32 PNG 与 JPEG 各一张
        let png_path = out_dir.join("sample.png");
        let jpg_path = out_dir.join("sample.jpg");
        {
            use image::{ImageBuffer, Rgb};
            let buf: ImageBuffer<Rgb<u8>, Vec<u8>> =
                ImageBuffer::from_pixel(32, 32, Rgb([255, 0, 0]));
            buf.save(&png_path).unwrap();
            buf.save(&jpg_path).unwrap();
        }
        let imgs = out_dir.join("images.pdf");
        images_to_pdf(&[png_path.clone(), jpg_path], &imgs, "auto").unwrap();
        let di = Document::load(&imgs).unwrap();
        assert_eq!(di.get_pages().len(), 2, "两张图应生成两页");

        // A4 模式
        let imgs_a4 = out_dir.join("images_a4.pdf");
        images_to_pdf(&[png_path], &imgs_a4, "a4").unwrap();
        let dia = Document::load(&imgs_a4).unwrap();
        assert_eq!(dia.get_pages().len(), 1);
        println!("✅ 图片转 PDF 全部通过");
    }

    // ---------- 图片转 PDF 回归测试（不依赖外部样例文件） ----------

    /// 测试用 CRC32（PNG chunk 校验）
    fn crc32(data: &[u8]) -> u32 {
        let mut c = 0xFFFFFFFFu32;
        for &b in data {
            c ^= b as u32;
            for _ in 0..8 {
                c = (c >> 1) ^ (0xEDB88320 & 0u32.wrapping_sub(c & 1));
            }
        }
        c ^ 0xFFFFFFFF
    }

    fn png_chunk(tag: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut v = (body.len() as u32).to_be_bytes().to_vec();
        v.extend_from_slice(tag);
        v.extend_from_slice(body);
        let mut cb = Vec::with_capacity(4 + body.len());
        cb.extend_from_slice(tag);
        cb.extend_from_slice(body);
        v.extend_from_slice(&crc32(&cb).to_be_bytes());
        v
    }

    /// 手工编码 Adam7 隔行扫描 PNG（image crate 不支持生成，用于回归测试）
    fn encode_adam7_png(w: u32, h: u32, pixels: &[u8]) -> Vec<u8> {
        let mut raw = Vec::new();
        // Adam7 七个 pass 的（起始 x、起始 y、步进 x、步进 y）
        let passes: [(usize, usize, usize, usize); 7] = [
            (0, 0, 8, 8),
            (4, 0, 8, 8),
            (0, 4, 4, 8),
            (2, 0, 4, 4),
            (0, 2, 2, 4),
            (1, 0, 2, 2),
            (0, 1, 1, 2),
        ];
        for &(sx, sy, dx, dy) in &passes {
            let mut y = sy;
            while y < h as usize {
                raw.push(0u8); // filter None
                let mut x = sx;
                while x < w as usize {
                    let o = (y * w as usize + x) * 3;
                    raw.extend_from_slice(&pixels[o..o + 3]);
                    x += dx;
                }
                y += dy;
            }
        }
        let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&w.to_be_bytes());
        ihdr.extend_from_slice(&h.to_be_bytes());
        ihdr.extend_from_slice(&[8, 2, 0, 0, 1]); // 8 位 RGB，interlace=Adam7
        out.extend(png_chunk(b"IHDR", &ihdr));
        let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        std::io::Write::write_all(&mut enc, &raw).unwrap();
        out.extend(png_chunk(b"IDAT", &enc.finish().unwrap()));
        out.extend(png_chunk(b"IEND", &[]));
        out
    }

    /// 从 PDF 中取出图像 XObject 的解码后原始数据与颜色空间
    fn extract_pdf_image(doc: &Document) -> (Vec<u8>, Vec<u8>) {
        for (_id, obj) in &doc.objects {
            if let Object::Stream(s) = obj {
                let is_img = s.dict.get(b"Subtype").ok()
                    .and_then(|o| o.as_name().ok())
                    .map(|n| n == b"Image")
                    .unwrap_or(false);
                if is_img {
                    let cs = s.dict.get(b"ColorSpace").ok()
                        .and_then(|o| o.as_name().ok())
                        .unwrap_or(b"Unknown")
                        .to_vec();
                    // zlib 头（0x78）说明仍是压缩数据，手动解码
                    let data = if s.content.first() == Some(&0x78) {
                        let mut dec = flate2::read::ZlibDecoder::new(&s.content[..]);
                        let mut out = Vec::new();
                        std::io::Read::read_to_end(&mut dec, &mut out).unwrap();
                        out
                    } else {
                        s.content.clone()
                    };
                    return (data, cs);
                }
            }
        }
        panic!("PDF 中未找到图像 XObject");
    }

    #[test]
    fn test_images_to_pdf_adam7_and_alpha() {
        let out_dir = PathBuf::from("/tmp/pdf_tool_test_img");
        let _ = fs::remove_dir_all(&out_dir);
        fs::create_dir_all(&out_dir).unwrap();

        // 1️⃣ Adam7 隔行 PNG：像素必须与原图一致（修复前 Predictor 15 与隔行数据不符会花屏）
        let (w, h) = (8u32, 8u32);
        let mut px = vec![0u8; (w * h * 3) as usize];
        for y in 0..h {
            for x in 0..w {
                let o = ((y * w + x) * 3) as usize;
                px[o] = (x * 32) as u8;
                px[o + 1] = (y * 32) as u8;
                px[o + 2] = 128;
            }
        }
        let png = encode_adam7_png(w, h, &px);
        assert!(
            image::load_from_memory(&png).is_ok(),
            "手工编码的 Adam7 PNG 应可被 image crate 解码（测试前置自检）"
        );
        let a7 = out_dir.join("adam7.png");
        fs::write(&a7, &png).unwrap();
        let pdf = out_dir.join("adam7.pdf");
        images_to_pdf(&[a7], &pdf, "auto").unwrap();
        let doc = Document::load(&pdf).unwrap();
        let (data, cs) = extract_pdf_image(&doc);
        assert_eq!(cs, b"DeviceRGB", "Adam7 PNG 应按 RGB 嵌入");
        assert_eq!(data, px, "Adam7 隔行 PNG 嵌入后像素应与原图一致");

        // 2️⃣ 透明 PNG：透明区域应与白底混合而非黑块
        let rgba = image::RgbaImage::from_fn(2, 1, |x, _| {
            if x == 0 {
                image::Rgba([0, 0, 0, 0]) // 全透明黑 → 应渲染为白
            } else {
                image::Rgba([255, 0, 0, 255]) // 不透明红
            }
        });
        let alpha_png = out_dir.join("alpha.png");
        rgba.save(&alpha_png).unwrap();
        let pdf2 = out_dir.join("alpha.pdf");
        images_to_pdf(&[alpha_png], &pdf2, "auto").unwrap();
        let doc2 = Document::load(&pdf2).unwrap();
        let (data2, _) = extract_pdf_image(&doc2);
        assert_eq!(&data2[..3], &[255, 255, 255], "透明像素应与白底混合为白色");
        assert_eq!(&data2[3..6], &[255, 0, 0], "不透明红色应保持原色");

        println!("✅ 图片转 PDF 回归测试通过（Adam7 隔行 + 透明混合）");
    }

    #[test]
    fn test_malformed_wallpaper() {
        // 畸形 PNG：IHDR 声明 RGBA + interlace=1，实际数据为非隔行 RGBA（壁纸包常见）
        let p = PathBuf::from("tests/assets/malformed_wallpaper.png");
        assert!(p.exists(), "测试素材缺失: {}", p.display());
        let raw = load_image(&p).expect("畸形 PNG 应可解码");
        println!("load_image 输出: w={} h={}", raw.w(), raw.h());
        let pdf = PathBuf::from("/tmp/wall_test.pdf");
        images_to_pdf(&[p], &pdf, "auto").unwrap();
        let doc = Document::load(&pdf).unwrap();
        let page = doc.get_pages().values().next().copied().unwrap();
        let dict = doc.get_dictionary(page).unwrap();
        let mb = dict.get(b"MediaBox").unwrap();
        println!("MediaBox: {:?}", mb);
    }

    /// 普通 8 位 RGB 非隔行 PNG（直嵌 Predictor 15 路径）与 JPEG（直嵌 DCTDecode 路径）回归测试
    /// 覆盖用户常见“照片/截图转 PDF”场景；此前两条路径均无像素级断言
    #[test]
    fn test_images_to_pdf_plain_png_and_jpeg() {
        use image::{ImageBuffer, Rgb};
        let out_dir = PathBuf::from("/tmp/pdf_img_plain_test");
        let _ = fs::remove_dir_all(&out_dir);
        fs::create_dir_all(&out_dir).unwrap();

        // 生成 64x48 渐变图（每像素不同，能抓住任何错位/花屏）
        let w = 64u32;
        let h = 48u32;
        let px: Vec<u8> = (0..h)
            .flat_map(|y| (0..w).flat_map(move |x| vec![(x * 4) as u8, (y * 5) as u8, ((x + y) * 3) as u8]))
            .collect();
        let buf: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_raw(w, h, px.clone()).unwrap();

        // 1️⃣ 普通 RGB PNG（非隔行、无 alpha）→ 应走 Png 直嵌路径
        let png_path = out_dir.join("grad.png");
        buf.save(&png_path).unwrap();
        let pdf1 = out_dir.join("grad.pdf");
        images_to_pdf(&[png_path.clone()], &pdf1, "auto").unwrap();
        let doc = Document::load(&pdf1).unwrap();
        let (data, cs) = extract_pdf_image(&doc);
        assert_eq!(cs, b"DeviceRGB", "普通 RGB PNG 应按 DeviceRGB 嵌入");
        // 解 zlib 后仍是 PNG 滤波流（每行前 1 字节 filter），与源图 IDAT 解压后必须逐字节一致
        let src_raw = {
            let b = fs::read(&png_path).unwrap();
            let mut idat = Vec::new();
            let mut pos = 8usize;
            while pos + 12 <= b.len() {
                let len = u32::from_be_bytes([b[pos], b[pos + 1], b[pos + 2], b[pos + 3]]) as usize;
                let t = &b[pos + 4..pos + 8];
                if t == b"IEND" {
                    break;
                }
                if t == b"IDAT" {
                    idat.extend_from_slice(&b[pos + 8..pos + 8 + len]);
                }
                pos += 12 + len;
            }
            let mut dec = flate2::read::ZlibDecoder::new(&idat[..]);
            let mut out = Vec::new();
            std::io::Read::read_to_end(&mut dec, &mut out).unwrap();
            out
        };
        assert_eq!(data, src_raw, "PNG 直嵌流与源图滤波流应一致");
        // 再做 Predictor 15 (PNG) 解码，像素级对比
        let stride = (w * 3) as usize;
        let mut dec = Vec::with_capacity(stride * h as usize);
        let mut p = 0usize;
        let mut prev = vec![0u8; stride];
        for _y in 0..h {
            let ft = data[p];
            p += 1;
            let mut row = data[p..p + stride].to_vec();
            p += stride;
            match ft {
                0 => {}
                1 => for i in 3..stride {
                    row[i] = row[i].wrapping_add(row[i - 3]);
                },
                2 => for i in 0..stride {
                    row[i] = row[i].wrapping_add(prev[i]);
                },
                3 => for i in 0..stride {
                    let a = if i >= 3 { row[i - 3] as u16 } else { 0 };
                    row[i] = (row[i] as u16 + ((a + prev[i] as u16) >> 1)) as u8;
                },
                4 => for i in 0..stride {
                    let a = if i >= 3 { row[i - 3] } else { 0 };
                    let b = prev[i];
                    let c = if i >= 3 { prev[i - 3] } else { 0 };
                    let pa = (b as i16 - c as i16).abs();
                    let pb = (a as i16 - c as i16).abs();
                    let pc = (a as i16 + b as i16 - 2 * c as i16).abs();
                    let pr = if pa <= pb && pa <= pc {
                        a
                    } else if pb <= pc {
                        b
                    } else {
                        c
                    };
                    row[i] = row[i].wrapping_add(pr);
                },
                _ => panic!("未知 PNG 滤波类型 {}", ft),
            }
            dec.extend_from_slice(&row);
            prev = row;
        }
        assert_eq!(dec, px, "Predictor 15 解码后像素应与源图一致");

        // 2️⃣ JPEG → 应走 DCTDecode 直嵌路径，流字节与源 JPEG 完全一致
        let jpg_path = out_dir.join("grad.jpg");
        buf.save(&jpg_path).unwrap();
        let pdf2 = out_dir.join("grad_jpg.pdf");
        images_to_pdf(&[jpg_path.clone()], &pdf2, "auto").unwrap();
        let doc = Document::load(&pdf2).unwrap();
        let (data, cs) = extract_pdf_image(&doc);
        assert_eq!(cs, b"DeviceRGB", "RGB JPEG 应按 DeviceRGB 嵌入");
        let src_jpg = fs::read(&jpg_path).unwrap();
        assert_eq!(data, src_jpg, "JPEG 直嵌流应与源文件字节一致");

        println!("✅ 普通 PNG / JPEG 直嵌路径像素级验证通过");
    }

    /// 生成带文本页的 PDF（每页一个 Tj 文本），用于文本提取 / 对比测试
    fn make_text_pdf(path: &std::path::Path, texts: &[&str]) {
        let header = "%PDF-1.4\n";
        let n = texts.len();
        let total = 3 + 2 * n;
        let mut offsets = vec![0usize; total];
        let mut body = String::new();
        let add = |body: &mut String, offsets: &mut Vec<usize>, id: usize, content: String| {
            offsets[id] = header.len() + body.len();
            body.push_str(&content);
        };
        let mut kids = String::new();
        for i in 0..n {
            kids.push_str(&format!("{} 0 R ", 3 + i));
        }
        add(&mut body, &mut offsets, 1, "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".into());
        add(&mut body, &mut offsets, 2, format!("2 0 obj\n<< /Type /Pages /Kids [{}] /Count {} >>\nendobj\n", kids.trim_end(), n));
        for (i, t) in texts.iter().enumerate() {
            let stream_id = 3 + n + i;
            let content = format!("BT /F1 12 Tf 72 720 Td ({}) Tj ET", t);
            add(&mut body, &mut offsets, 3 + i, format!(
                "{id} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents {sid} 0 R >>\nendobj\n",
                id = 3 + i,
                sid = stream_id
            ));
            add(&mut body, &mut offsets, stream_id, format!(
                "{id} 0 obj\n<< /Length {len} >>\nstream\n{content}\nendstream\nendobj\n",
                id = stream_id,
                len = content.len()
            ));
        }
        let xref_off = header.len() + body.len();
        let mut xref = format!("xref\n0 {}\n0000000000 65535 f \n", total);
        for id in 1..total {
            xref.push_str(&format!("{:010} 00000 n \n", offsets[id]));
        }
        let content = format!(
            "{}{}{}\ntrailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            header, body, xref, total, xref_off
        );
        fs::write(path, content).unwrap();
    }

    /// 页面整理（提取/删除页）、元数据、裁剪、书签 回归测试
    #[test]
    fn test_page_organize_metadata_crop_outline() {
        let out_dir = PathBuf::from("/tmp/pdf_tool_test_organize");
        let _ = fs::remove_dir_all(&out_dir);
        fs::create_dir_all(&out_dir).unwrap();
        let sample = sample_pdf_path(&out_dir);

        // 1) 提取页：1,3 → 2 页
        let ex = out_dir.join("extracted.pdf");
        extract_pages(&sample, &ex, &[1, 3]).unwrap();
        let de = Document::load(&ex).unwrap();
        assert_eq!(de.get_pages().len(), 2, "提取 1,3 页应得到 2 页");
        assert!(extract_pages(&sample, &ex, &[0]).is_err(), "页码 0 应报错");
        assert!(extract_pages(&sample, &ex, &[7]).is_err(), "越界页码应报错");
        assert!(extract_pages(&sample, &ex, &[1, 1]).is_err(), "重复页码应报错");

        // 2) 删除页：删 2-3 → 4 页
        let dl = out_dir.join("deleted.pdf");
        delete_pages_range(&sample, &dl, &[(2, 3)]).unwrap();
        let dd = Document::load(&dl).unwrap();
        assert_eq!(dd.get_pages().len(), 4, "删除 2-3 页后应剩 4 页");
        assert!(delete_pages_range(&sample, &dl, &[(1, 6)]).is_err(), "删除全部页应报错");
        assert!(delete_pages_range(&sample, &dl, &[(3, 2)]).is_err(), "范围颠倒应报错");

        // 3) 元数据：写入后可读回
        let md = out_dir.join("meta.pdf");
        pdf_metadata(&sample, &md, Some("标题"), Some("作者"), Some("主题"), Some("关键词")).unwrap();
        let dm = Document::load(&md).unwrap();
        let info_id = dm.trailer.get(b"Info").unwrap().as_reference().unwrap();
        let info = dm.get_dictionary(info_id).unwrap();
        assert_eq!(info.get(b"Title").unwrap().as_string().unwrap(), "标题");
        assert_eq!(info.get(b"Author").unwrap().as_string().unwrap(), "作者");
        assert_eq!(info.get(b"Keywords").unwrap().as_string().unwrap(), "关键词");

        // 4) 裁剪：MediaBox 变为指定边界；无效参数报错
        let cr = out_dir.join("cropped.pdf");
        pdf_crop(&sample, &cr, 10.0, 20.0, 500.0, 700.0).unwrap();
        let dc = Document::load(&cr).unwrap();
        let page = dc.get_pages().values().next().copied().unwrap();
        let dict = dc.get_dictionary(page).unwrap();
        let mb = dict.get(b"MediaBox").unwrap().as_array().unwrap();
        assert_eq!(mb[2].as_float().unwrap(), 500.0);
        assert_eq!(mb[3].as_float().unwrap(), 700.0);
        assert!(pdf_crop(&sample, &cr, 500.0, 20.0, 10.0, 700.0).is_err(), "右小于左应报错");

        // 5) 书签：目录应出现 Outlines；空书签报错
        let ol = out_dir.join("outline.pdf");
        pdf_outline(&sample, &ol, &[("第一章".into(), 1), ("第二章".into(), 2)]).unwrap();
        let dout = Document::load(&ol).unwrap();
        let catalog_id = dout.trailer.get(b"Root").unwrap().as_reference().unwrap();
        let catalog = dout.get_dictionary(catalog_id).unwrap();
        assert!(catalog.get(b"Outlines").is_ok(), "书签后目录应有 Outlines");
        assert!(pdf_outline(&sample, &ol, &[]).is_err(), "空书签应报错");

        println!("✅ 页面整理 / 元数据 / 裁剪 / 书签 全部通过");
    }

    /// 去水印、提取 PDF 图片、文本提取、PDF 对比 回归测试
    #[test]
    fn test_watermark_remove_extract_images_compare_text() {
        let out_dir = PathBuf::from("/tmp/pdf_tool_test_wm");
        let _ = fs::remove_dir_all(&out_dir);
        fs::create_dir_all(&out_dir).unwrap();
        let sample = sample_pdf_path(&out_dir);

        // 1) 去水印：给第一页注入 Stamp 注释后移除
        let mut doc = Document::load(&sample).unwrap();
        let page_id = doc.get_pages().values().next().copied().unwrap();
        let stamp_id = doc.add_object(Object::Dictionary(Dictionary::from_iter(vec![
            (b"Type".to_vec(), Object::Name(b"Annot".to_vec())),
            (b"Subtype".to_vec(), Object::Name(b"Stamp".to_vec())),
            (b"Rect".to_vec(), Object::Array(vec![
                Object::Integer(0), Object::Integer(0), Object::Integer(100), Object::Integer(100),
            ])),
        ])));
        if let Ok(page) = doc.get_dictionary_mut(page_id) {
            page.set("Annots", Object::Array(vec![Object::Reference(stamp_id)]));
        }
        let with_wm = out_dir.join("with_wm.pdf");
        doc.save(&with_wm).unwrap();
        let no_wm = out_dir.join("no_wm.pdf");
        remove_watermark(&with_wm, &no_wm).unwrap();
        let dn = Document::load(&no_wm).unwrap();
        let page = dn.get_pages().values().next().copied().unwrap();
        let dict = dn.get_dictionary(page).unwrap();
        assert!(dict.get(b"Annots").is_err(), "水印注释应被移除");
        assert!(remove_watermark(&sample, &no_wm).is_err(), "无水印文档应报错");

        // 2) 提取 PDF 图片：先合成含图 PDF 再提取
        let png_path = out_dir.join("img.png");
        {
            use image::{ImageBuffer, Rgb};
            let buf: ImageBuffer<Rgb<u8>, Vec<u8>> =
                ImageBuffer::from_pixel(16, 16, Rgb([10, 200, 30]));
            buf.save(&png_path).unwrap();
        }
        let img_pdf = out_dir.join("img.pdf");
        images_to_pdf(&[png_path], &img_pdf, "auto").unwrap();
        let imgs_out = out_dir.join("imgs");
        let files = extract_pdf_images(&img_pdf, &imgs_out).unwrap();
        assert_eq!(files.len(), 1, "应提取到 1 张图片");
        assert!(!fs::read(&files[0]).unwrap().is_empty(), "提取的图片不应为空");
        assert!(extract_pdf_images(&sample, &imgs_out).is_err(), "无图 PDF 应报错");

        // 3) 文本提取：Tj 内容流可提取；空白页无文本
        let tp = out_dir.join("text.pdf");
        make_text_pdf(&tp, &["Hello PDF", "Second page"]);
        let text = pdf_extract_text(&tp).unwrap();
        assert!(text.contains("Hello PDF"), "应提取到 Hello PDF，实际: {}", text);
        assert!(text.contains("Second page"), "应提取到 Second page");
        assert_eq!(pdf_extract_text(&sample).unwrap().trim(), "", "空白页应无文本");

        // 4) 对比：相同文档无差异，不同文档有差异
        let diff = compare_pdfs(&tp, &tp).unwrap();
        assert!(diff.iter().all(|d| d.status == "unchanged"), "相同文档应无差异");
        let tp2 = out_dir.join("text2.pdf");
        make_text_pdf(&tp2, &["Hello PDF", "Different page"]);
        let diff2 = compare_pdfs(&tp, &tp2).unwrap();
        assert!(
            diff2.iter().any(|d| d.status == "removed" || d.status == "added"),
            "不同文档应有差异: {:?}",
            diff2.iter().map(|d| d.status.as_str()).collect::<Vec<_>>()
        );

        println!("✅ 去水印 / 提取图片 / 文本提取 / PDF 对比 全部通过");
    }

    /// 图片压缩回归测试（原地覆盖写回）
    #[test]
    fn test_image_compress() {
        use image::{ImageBuffer, Rgb};
        let out_dir = PathBuf::from("/tmp/pdf_img_compress_test");
        let _ = fs::remove_dir_all(&out_dir);
        fs::create_dir_all(&out_dir).unwrap();

        // JPEG：低质量应显著变小且仍可解码
        let jpg = out_dir.join("big.jpg");
        {
            let buf: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(800, 600, |x, y| {
                Rgb([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8])
            });
            buf.save(&jpg).unwrap();
        }
        let before = fs::metadata(&jpg).unwrap().len();
        assert!(image_compress(&jpg, 30).unwrap(), "JPEG 重压应变小并覆盖");
        let after = fs::metadata(&jpg).unwrap().len();
        assert!(after < before, "JPEG 压缩后应变小: {} -> {}", before, after);
        assert!(image::open(&jpg).is_ok(), "压缩后应仍可解码");

        // PNG：原地覆盖后仍是合法 PNG，且大小不增大
        let png = out_dir.join("p.png");
        {
            let buf: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(400, 300, |x, y| {
                Rgb([(x * 7 % 256) as u8, (y * 11 % 256) as u8, 128])
            });
            buf.save(&png).unwrap();
        }
        let before = fs::metadata(&png).unwrap().len();
        image_compress(&png, 90).unwrap();
        let after = fs::metadata(&png).unwrap().len();
        assert!(after <= before, "PNG 压缩后不应增大: {} -> {}", before, after);
        assert!(image::open(&png).is_ok(), "PNG 压缩后应仍可解码");

        // quality 越界应 clamp 而非报错
        image_compress(&jpg, 0).unwrap();
        image_compress(&jpg, 200).unwrap();

        println!("✅ 图片压缩全部通过");
    }

    /// 校验页面树合法性：每个 Page 必须有 /Parent 指向根 Pages 节点。
    /// 缺失 /Parent 会被 Apple Preview / QuickLook 渲染器拒绝（表现为整份 PDF 无法打开）
    fn assert_valid_page_tree(path: &std::path::Path, expect_pages: usize) {
        let doc = Document::load(path).unwrap();
        assert_eq!(doc.get_pages().len(), expect_pages, "页数不符: {}", path.display());
        let catalog_id = doc.trailer.get(b"Root").unwrap().as_reference().unwrap();
        let catalog = doc.get_dictionary(catalog_id).unwrap();
        let pages_ref = catalog.get(b"Pages").unwrap().as_reference().unwrap();
        let pages_obj = doc.get_dictionary(pages_ref).unwrap();
        assert_eq!(
            pages_obj.get(b"Type").unwrap().as_name().unwrap(),
            b"Pages",
            "根对象必须是 Pages 类型: {}",
            path.display()
        );
        for (_no, page_id) in doc.get_pages() {
            let page = doc.get_dictionary(page_id).unwrap();
            let parent = page.get(b"Parent").unwrap().as_reference().unwrap();
            assert_eq!(
                parent, pages_ref,
                "页面 {} 的 /Parent 应指向根 Pages: {}",
                _no, path.display()
            );
        }
    }

    /// 回归：所有生成路径（图片转 PDF / HTML 转 PDF / 合并 / 提取页）产出的 PDF
    /// 页面树必须合法（含 /Parent），否则 Apple 预览等渲染器无法打开
    #[test]
    fn test_generated_pdfs_page_tree_valid() {
        use crate::engine::light;
        let out_dir = PathBuf::from("/tmp/pdf_page_tree_test");
        let _ = fs::remove_dir_all(&out_dir);
        fs::create_dir_all(&out_dir).unwrap();

        // 图片转 PDF（PNG 直嵌 + JPEG 直嵌）
        let png = out_dir.join("a.png");
        let jpg = out_dir.join("b.jpg");
        {
            use image::{ImageBuffer, Rgb};
            let buf: ImageBuffer<Rgb<u8>, Vec<u8>> =
                ImageBuffer::from_pixel(32, 32, Rgb([9, 8, 7]));
            buf.save(&png).unwrap();
            buf.save(&jpg).unwrap();
        }
        let imgs = out_dir.join("imgs.pdf");
        images_to_pdf(&[png, jpg], &imgs, "auto").unwrap();
        assert_valid_page_tree(&imgs, 2);

        // HTML → PDF
        let html = out_dir.join("t.html");
        fs::write(&html, "<h1>Title</h1><p>Body</p>").unwrap();
        let hpdf = out_dir.join("t.pdf");
        light::html_to_pdf(&html, &hpdf).unwrap();
        assert_valid_page_tree(&hpdf, 1);

        // 合并（跨文件导入的页面 /Parent 必须重指向新根）
        let merged = out_dir.join("m.pdf");
        merge_pdfs(&[imgs.clone(), hpdf.clone()], &merged).unwrap();
        assert_valid_page_tree(&merged, 3);

        // 提取页
        let ex = out_dir.join("ex.pdf");
        extract_pages(&imgs, &ex, &[1]).unwrap();
        assert_valid_page_tree(&ex, 1);

        println!("✅ 页面树 /Parent 回归测试通过（Preview 可打开）");
    }
}
