//! PDF 工具命令：合并 / 拆分 / 压缩 / 水印 / 页码 / 旋转 / 加解密 / 图片转 PDF
use crate::engine::pdf;
use crate::engine::light;
use std::path::{Path, PathBuf};

/// 确保输出文件所在目录存在（批量处理时输出目录可能尚未创建）
fn ensure_parent_dir(out_path: &Path) -> Result<(), String> {
    if let Some(parent) = out_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建输出目录失败: {}", e))?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn pdf_merge(paths: Vec<String>, out_path: String) -> Result<String, String> {
    ensure_parent_dir(Path::new(&out_path))?;
    let ps: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
    pdf::merge_pdfs(&ps, Path::new(&out_path))?;
    Ok(out_path)
}

#[tauri::command]
pub fn pdf_split(
    input_path: String,
    ranges: Vec<Vec<u32>>,
    out_dir: String,
) -> Result<Vec<String>, String> {
    let ranges: Vec<(u32, u32)> = ranges
        .into_iter()
        .map(|r| {
            if r.len() != 2 {
                return Err("页范围必须是 [起始页, 结束页]".to_string());
            }
            Ok((r[0], r[1]))
        })
        .collect::<Result<_, String>>()?;
    ensure_parent_dir(Path::new(&out_dir))?;
    // 前缀 = 原文件名（不含扩展名），如 合同.pdf → 合同_2-4.pdf
    let stem = Path::new(&input_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("split")
        .to_string();
    let outs = pdf::split_pdf(Path::new(&input_path), &ranges, Path::new(&out_dir), &stem)?;
    Ok(outs
        .into_iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect())
}

#[tauri::command]
pub fn pdf_compress(input_path: String, out_path: String) -> Result<String, String> {
    ensure_parent_dir(Path::new(&out_path))?;
    pdf::compress_pdf(Path::new(&input_path), Path::new(&out_path))?;
    Ok(out_path)
}

#[tauri::command]
pub fn get_pdf_page_count(input_path: String) -> Result<u32, String> {
    let doc = lopdf::Document::load(Path::new(&input_path))
        .map_err(|e| format!("读取 PDF 失败: {}", e))?;
    Ok(doc.get_pages().len() as u32)
}

/// 添加平铺文字水印（text 支持中文，opacity 0.05~1.0）
#[tauri::command]
pub fn pdf_watermark(
    input_path: String,
    out_path: String,
    text: String,
    opacity: f32,
) -> Result<String, String> {
    ensure_parent_dir(Path::new(&out_path))?;
    pdf::add_watermark(Path::new(&input_path), Path::new(&out_path), &text, opacity)?;
    Ok(out_path)
}

/// 添加页码；style: "page" 显示「n」，"pageOf" 显示「n / total」
#[tauri::command]
pub fn pdf_page_numbers(
    input_path: String,
    out_path: String,
    style: String,
) -> Result<String, String> {
    ensure_parent_dir(Path::new(&out_path))?;
    pdf::add_page_numbers(Path::new(&input_path), Path::new(&out_path), &style)?;
    Ok(out_path)
}

/// 旋转所有页面（90 / 180 / 270）
#[tauri::command]
pub fn pdf_rotate(input_path: String, out_path: String, angle: i32) -> Result<String, String> {
    ensure_parent_dir(Path::new(&out_path))?;
    pdf::rotate_pdf(Path::new(&input_path), Path::new(&out_path), angle)?;
    Ok(out_path)
}

/// 加密 PDF（打开密码 + 所有者密码，RC4-128）
#[tauri::command]
pub fn pdf_encrypt(
    input_path: String,
    out_path: String,
    user_pass: String,
    owner_pass: String,
) -> Result<String, String> {
    ensure_parent_dir(Path::new(&out_path))?;
    pdf::encrypt_pdf(
        Path::new(&input_path),
        Path::new(&out_path),
        &user_pass,
        &owner_pass,
    )?;
    Ok(out_path)
}

/// 按指定页序提取页面（可挑页 / 重排；页码 1 起）
#[tauri::command]
pub fn pdf_extract_pages(
    input_path: String,
    out_path: String,
    pages: Vec<u32>,
) -> Result<String, String> {
    ensure_parent_dir(Path::new(&out_path))?;
    pdf::extract_pages(Path::new(&input_path), Path::new(&out_path), &pages)?;
    Ok(out_path)
}

/// 删除指定范围的页面（保留其余页，保持原顺序）
#[tauri::command]
pub fn pdf_delete_pages(
    input_path: String,
    out_path: String,
    ranges: Vec<Vec<u32>>,
) -> Result<String, String> {
    ensure_parent_dir(Path::new(&out_path))?;
    let ranges: Vec<(u32, u32)> = ranges
        .into_iter()
        .map(|r| {
            if r.len() != 2 {
                return Err("页范围必须是 [起始页, 结束页]".to_string());
            }
            Ok((r[0], r[1]))
        })
        .collect::<Result<_, String>>()?;
    pdf::delete_pages_range(Path::new(&input_path), Path::new(&out_path), &ranges)?;
    Ok(out_path)
}

/// 解密 PDF（移除打开密码）
#[tauri::command]
pub fn pdf_decrypt(
    input_path: String,
    out_path: String,
    password: String,
) -> Result<String, String> {
    ensure_parent_dir(Path::new(&out_path))?;
    pdf::decrypt_pdf(Path::new(&input_path), Path::new(&out_path), &password)?;
    Ok(out_path)
}

/// 多张图片合成一个 PDF；page_size: "auto" 跟随图片尺寸 / "a4" A4 居中
#[tauri::command]
pub fn images_to_pdf(
    paths: Vec<String>,
    out_path: String,
    page_size: String,
) -> Result<String, String> {
    ensure_parent_dir(Path::new(&out_path))?;
    let ps: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
    pdf::images_to_pdf(&ps, Path::new(&out_path), &page_size)?;
    Ok(out_path)
}

#[tauri::command]
pub fn open_path(path: String) -> Result<(), String> {
    tauri_plugin_opener::open_path(&path, None::<&str>).map_err(|e| e.to_string())
}

/* ---------- 批次 A：轻量引擎扩展命令 ---------- */

/// 提取 docx 中嵌入的图片到输出目录，返回图片路径列表
#[tauri::command]
pub fn docx_extract_images(input_path: String, out_dir: String) -> Result<Vec<String>, String> {
    light::extract_docx_images(Path::new(&input_path), Path::new(&out_dir))
}

/* ---------- 批次 B：PDF 工具箱扩展 ---------- */

/// 设置 PDF 文档元数据（标题、作者、主题、关键词）
#[tauri::command]
pub fn pdf_metadata(
    input_path: String,
    out_path: String,
    title: Option<String>,
    author: Option<String>,
    subject: Option<String>,
    keywords: Option<String>,
) -> Result<String, String> {
    ensure_parent_dir(Path::new(&out_path))?;
    pdf::pdf_metadata(
        Path::new(&input_path),
        Path::new(&out_path),
        title.as_deref(),
        author.as_deref(),
        subject.as_deref(),
        keywords.as_deref(),
    )?;
    Ok(out_path)
}

/// 裁剪 PDF 页面（统一设置 MediaBox），参数为左、下、右、上（PDF 单位 pt）
#[tauri::command]
pub fn pdf_crop(
    input_path: String,
    out_path: String,
    left: f32,
    bottom: f32,
    right: f32,
    top: f32,
) -> Result<String, String> {
    ensure_parent_dir(Path::new(&out_path))?;
    pdf::pdf_crop(Path::new(&input_path), Path::new(&out_path), left, bottom, right, top)?;
    Ok(out_path)
}

/// 添加 PDF 书签（大纲），items 为 [[title, pageNum], ...] 列表
#[tauri::command]
pub fn pdf_outline(
    input_path: String,
    out_path: String,
    items: Vec<Vec<serde_json::Value>>,
) -> Result<String, String> {
    ensure_parent_dir(Path::new(&out_path))?;
    let parsed: Vec<(String, u32)> = items
        .into_iter()
        .map(|v| {
            if v.len() != 2 {
                return Err("书签项必须是 [title, pageNum]".to_string());
            }
            let title = v[0].as_str().ok_or("书签标题必须是字符串")?.to_string();
            let page = v[1].as_u64().ok_or("页码必须是数字")? as u32;
            Ok((title, page))
        })
        .collect::<Result<_, String>>()?;
    pdf::pdf_outline(Path::new(&input_path), Path::new(&out_path), &parsed)?;
    Ok(out_path)
}

/// 压缩图片文件（覆盖原文件），quality 1~100
#[tauri::command]
pub fn image_compress(path: String, quality: u8) -> Result<(), String> {
    pdf::image_compress(Path::new(&path), quality)
}

/* ---------- 新功能：PDF 提取图片 / 去水印 / 比较 ---------- */

#[tauri::command]
pub fn pdf_extract_images(input_path: String, out_dir: String) -> Result<Vec<String>, String> {
    pdf::extract_pdf_images(Path::new(&input_path), Path::new(&out_dir))
}

#[tauri::command]
pub fn pdf_remove_watermark(input_path: String, out_path: String) -> Result<String, String> {
    ensure_parent_dir(Path::new(&out_path))?;
    pdf::remove_watermark(Path::new(&input_path), Path::new(&out_path))?;
    Ok(out_path)
}

#[tauri::command]
pub fn pdf_compare(input1: String, input2: String) -> Result<Vec<pdf::DiffEntry>, String> {
    pdf::compare_pdfs(Path::new(&input1), Path::new(&input2))
}

#[tauri::command]
pub fn pdf_extract_text(input_path: String) -> Result<String, String> {
    pdf::pdf_extract_text(Path::new(&input_path))
}

/// 提取 docx 的 word/document.xml 纯文本（AI 摘要用）
fn docx_extract_text(input: &Path) -> Result<String, String> {
    use quick_xml::events::Event;
    use quick_xml::Reader;
    use std::io::Read;

    let file = std::fs::File::open(input).map_err(|e| format!("读取文件失败: {}", e))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("无法解析 docx（zip 格式）: {}", e))?;
    let mut doc = archive
        .by_name("word/document.xml")
        .map_err(|_| "docx 中缺少 word/document.xml".to_string())?;
    let mut xml = String::new();
    doc.read_to_string(&mut xml)
        .map_err(|e| format!("读取文档内容失败: {}", e))?;

    let mut reader = Reader::from_str(&xml);
    let mut buf = Vec::new();
    let mut paras: Vec<String> = Vec::new();
    let mut in_text = false;
    let mut current = String::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                if e.name().as_ref() == b"w:t" {
                    in_text = true;
                }
            }
            Ok(Event::Text(t)) => {
                if in_text {
                    if let Ok(s) = t.unescape() {
                        current.push_str(&s);
                    }
                }
            }
            Ok(Event::Empty(e)) => {
                if e.name().as_ref() == b"w:tab" {
                    current.push('\t');
                } else if e.name().as_ref() == b"w:br" {
                    current.push('\n');
                }
            }
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"w:t" => in_text = false,
                b"w:p" => {
                    paras.push(std::mem::take(&mut current));
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    let text = paras
        .into_iter()
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    if text.is_empty() {
        return Err("docx 中未提取到文本内容".to_string());
    }
    Ok(text)
}

/// 提取文档全文文本（AI 摘要 / 语义分析用），支持 pdf / docx / 纯文本类
#[tauri::command]
pub fn extract_text(input_path: String) -> Result<String, String> {
    let p = Path::new(&input_path);
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "pdf" => pdf::pdf_extract_text(p),
        "docx" => docx_extract_text(p),
        "txt" | "md" | "markdown" | "csv" | "json" | "xml" | "html" | "htm" | "log" => {
            std::fs::read_to_string(p).map_err(|e| format!("读取文件失败: {}", e))
        }
        _ => Err(format!("暂不支持提取 {} 格式的文本", ext)),
    }
}
