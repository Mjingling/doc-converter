//! PDF 工具命令：合并 / 拆分 / 压缩 / 水印 / 页码 / 旋转 / 加解密 / 图片转 PDF
use crate::engine::pdf;
use crate::engine::light;
use crate::engine::render;
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

/// 添加平铺文字水印（text 支持中文，opacity 0.05~1.0，color RGB 0~255，font_size 字号）
#[tauri::command]
pub fn pdf_watermark(
    input_path: String,
    out_path: String,
    text: String,
    opacity: f32,
    color: [u8; 3],
    font_size: f32,
) -> Result<String, String> {
    ensure_parent_dir(Path::new(&out_path))?;
    let (r, g, b) = (color[0] as f32 / 255.0, color[1] as f32 / 255.0, color[2] as f32 / 255.0);
    pdf::add_watermark(
        Path::new(&input_path),
        Path::new(&out_path),
        &text,
        opacity,
        (r, g, b),
        font_size,
    )?;
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

/// 压缩图片文件（变小才覆盖原文件；返回 true=已重写，false=已是最优未改动），quality 1~100
#[tauri::command]
pub fn image_compress(path: String, quality: u8) -> Result<bool, String> {
    pdf::image_compress(Path::new(&path), quality)
}

/// 图片格式转换（png/jpg/webp/bmp/gif 互转）；quality 仅对 jpeg 目标生效
#[tauri::command]
pub fn image_convert(input_path: String, out_path: String, quality: u8) -> Result<String, String> {
    ensure_parent_dir(Path::new(&out_path))?;
    let img = image::open(Path::new(&input_path)).map_err(|e| format!("读取图片失败: {}", e))?;
    let ext = Path::new(&out_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let format = image::ImageFormat::from_extension(&ext)
        .ok_or_else(|| format!("不支持的目标格式: {}", ext))?;
    match format {
        // jpeg 不支持透明通道：转 RGB 并按质量编码
        image::ImageFormat::Jpeg => {
            let rgb = img.into_rgb8();
            let file = std::fs::File::create(Path::new(&out_path))
                .map_err(|e| format!("创建输出文件失败: {}", e))?;
            let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(file, quality);
            enc.encode_image(&rgb).map_err(|e| format!("保存图片失败: {}", e))?;
        }
        // bmp 同样不支持透明通道，其余格式保留 alpha
        image::ImageFormat::Bmp => img
            .into_rgb8()
            .save_with_format(Path::new(&out_path), format)
            .map_err(|e| format!("保存图片失败: {}", e))?,
        _ => img
            .save_with_format(Path::new(&out_path), format)
            .map_err(|e| format!("保存图片失败: {}", e))?,
    }
    Ok(out_path)
}

/// 图片缩放：宽或高传 0 表示按另一边长等比缩放；两者都给则精确拉伸
#[tauri::command]
pub fn image_resize(input_path: String, out_path: String, width: u32, height: u32) -> Result<String, String> {
    ensure_parent_dir(Path::new(&out_path))?;
    if width == 0 && height == 0 {
        return Err("宽和高至少需指定一项".to_string());
    }
    let img = image::open(Path::new(&input_path)).map_err(|e| format!("读取图片失败: {}", e))?;
    let resized = match (width, height) {
        (0, h) => img.resize(u32::MAX, h, image::imageops::FilterType::Lanczos3),
        (w, 0) => img.resize(w, u32::MAX, image::imageops::FilterType::Lanczos3),
        (w, h) => img.resize_exact(w, h, image::imageops::FilterType::Lanczos3),
    };
    resized
        .save(Path::new(&out_path))
        .map_err(|e| format!("保存图片失败: {}", e))?;
    Ok(out_path)
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

/// 提取 PDF 文本并保存为 txt 文件（批量提取文本用）
#[tauri::command]
pub fn pdf_extract_text_to_file(input_path: String, out_path: String) -> Result<String, String> {
    let text = pdf::pdf_extract_text(Path::new(&input_path))?;
    ensure_parent_dir(Path::new(&out_path))?;
    std::fs::write(Path::new(&out_path), text).map_err(|e| format!("写入文件失败: {}", e))?;
    Ok(out_path)
}

/// 保存文本内容为文件（AI 翻译结果输出等）
#[tauri::command]
pub fn save_text_file(out_path: String, content: String) -> Result<String, String> {
    ensure_parent_dir(Path::new(&out_path))?;
    std::fs::write(Path::new(&out_path), content).map_err(|e| format!("写入文件失败: {}", e))?;
    Ok(out_path)
}

/// PDF 逐页渲染为图片（Pdfium 内置引擎）；pages 为空时渲染全部页
#[tauri::command]
pub fn pdf_render(
    input_path: String,
    out_dir: String,
    pages: Option<Vec<u32>>,
    format: String,
    dpi: u16,
) -> Result<Vec<String>, String> {
    render::pdf_render_pages(Path::new(&input_path), Path::new(&out_dir), pages, &format, dpi)
}

/// 在 PDF 指定页绘制签名图片；x/y/width 均为页面宽高百分比（0~100），高度按图片比例
#[tauri::command]
pub fn pdf_sign(
    input_path: String,
    out_path: String,
    image_path: String,
    page: u32,
    x: f32,
    y: f32,
    width: f32,
) -> Result<String, String> {
    ensure_parent_dir(Path::new(&out_path))?;
    pdf::sign_pdf(
        Path::new(&input_path),
        Path::new(&out_path),
        Path::new(&image_path),
        page,
        x,
        y,
        width,
    )?;
    Ok(out_path)
}

/// 读取图片宽高（签名面板按图片比例换算高度用）
#[tauri::command]
pub fn image_size(input_path: String) -> Result<(u32, u32), String> {
    let dims = image::ImageReader::open(Path::new(&input_path))
        .map_err(|e| format!("读取图片失败: {}", e))?
        .into_dimensions()
        .map_err(|e| format!("解析图片失败: {}", e))?;
    Ok(dims)
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

#[cfg(test)]
mod tests {
    use super::*;
    use image::GenericImageView;
    use std::fs;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// 每个测试独立的临时目录序号（并行测试避免冲突）
    static TMP_SEQ: AtomicU32 = AtomicU32::new(0);

    fn tmp_dir() -> PathBuf {
        let n = TMP_SEQ.fetch_add(1, Ordering::Relaxed);
        let d = std::env::temp_dir().join(format!("pdf_tools_test_{}_{}", std::process::id(), n));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    /// 生成 N 页空白 A4 PDF（复用 pdf.rs 测试中的模式）
    fn make_blank_pages(path: &Path, n: usize) {
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

    /* ---------- ensure_parent_dir ---------- */

    #[test]
    fn test_ensure_parent_dir_creates_nested_dirs() {
        let d = tmp_dir();
        let out = d.join("a/b/c/output.pdf");
        assert!(ensure_parent_dir(&out).is_ok());
        assert!(d.join("a/b/c").is_dir());
    }

    #[test]
    fn test_ensure_parent_dir_existing_dir_ok() {
        let d = tmp_dir();
        let out = d.join("output.pdf");
        // 目录已存在（tmp_dir 本身），不应报错
        assert!(ensure_parent_dir(&out).is_ok());
    }

    #[test]
    fn test_ensure_parent_dir_no_parent() {
        // 纯文件名无目录部分
        assert!(ensure_parent_dir(Path::new("output.pdf")).is_ok());
    }

    /* ---------- pdf_split ranges 校验 ---------- */

    #[test]
    fn test_pdf_split_ranges_len_not_2() {
        let d = tmp_dir();
        let input = d.join("test.pdf");
        make_blank_pages(&input, 4);
        // ranges 长度为 3 → 报错
        let result = pdf_split(
            input.to_str().unwrap().to_string(),
            vec![vec![1, 2, 3]],
            d.to_str().unwrap().to_string(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("页范围"));
    }

    #[test]
    fn test_pdf_split_ranges_empty_vec() {
        let d = tmp_dir();
        let input = d.join("test.pdf");
        make_blank_pages(&input, 4);
        // ranges 长度为 0 → 报错
        let result = pdf_split(
            input.to_str().unwrap().to_string(),
            vec![vec![]],
            d.to_str().unwrap().to_string(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("页范围"));
    }

    #[test]
    fn test_pdf_split_valid_ranges() {
        let d = tmp_dir();
        let input = d.join("test.pdf");
        make_blank_pages(&input, 4);
        let result = pdf_split(
            input.to_str().unwrap().to_string(),
            vec![vec![1, 2], vec![3, 4]],
            d.to_str().unwrap().to_string(),
        );
        assert!(result.is_ok());
        let outs = result.unwrap();
        assert_eq!(outs.len(), 2);
    }

    /* ---------- pdf_delete_pages ranges 校验 ---------- */

    #[test]
    fn test_pdf_delete_pages_ranges_len_not_2() {
        let d = tmp_dir();
        let input = d.join("test.pdf");
        let out = d.join("out.pdf");
        make_blank_pages(&input, 4);
        let result = pdf_delete_pages(
            input.to_str().unwrap().to_string(),
            out.to_str().unwrap().to_string(),
            vec![vec![1]], // 长度 1
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("页范围"));
    }

    /* ---------- pdf_outline items 校验 ---------- */

    #[test]
    fn test_pdf_outline_items_wrong_length() {
        let d = tmp_dir();
        let input = d.join("test.pdf");
        let out = d.join("out.pdf");
        make_blank_pages(&input, 2);
        // items 长度为 3 → 报错
        let result = pdf_outline(
            input.to_str().unwrap().to_string(),
            out.to_str().unwrap().to_string(),
            vec![vec![
                serde_json::Value::String("标题".to_string()),
                serde_json::json!(1),
                serde_json::json!("extra"),
            ]],
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("书签项"));
    }

    #[test]
    fn test_pdf_outline_title_not_string() {
        let d = tmp_dir();
        let input = d.join("test.pdf");
        let out = d.join("out.pdf");
        make_blank_pages(&input, 2);
        let result = pdf_outline(
            input.to_str().unwrap().to_string(),
            out.to_str().unwrap().to_string(),
            vec![vec![serde_json::json!(123), serde_json::json!(1)]],
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("标题"));
    }

    #[test]
    fn test_pdf_outline_page_not_number() {
        let d = tmp_dir();
        let input = d.join("test.pdf");
        let out = d.join("out.pdf");
        make_blank_pages(&input, 2);
        let result = pdf_outline(
            input.to_str().unwrap().to_string(),
            out.to_str().unwrap().to_string(),
            vec![vec![serde_json::json!("标题"), serde_json::json!("abc")]],
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("页码"));
    }

    #[test]
    fn test_pdf_outline_empty_items_error() {
        let d = tmp_dir();
        let input = d.join("test.pdf");
        let out = d.join("out.pdf");
        make_blank_pages(&input, 2);
        // 空列表引擎层报错：至少需要一个书签项
        let result = pdf_outline(
            input.to_str().unwrap().to_string(),
            out.to_str().unwrap().to_string(),
            vec![],
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("至少需要一个书签项"));
    }

    /* ---------- extract_text 分发逻辑 ---------- */

    #[test]
    fn test_extract_text_unsupported_ext() {
        let result = extract_text("/tmp/test.xyz".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("暂不支持"));
    }

    #[test]
    fn test_extract_text_no_ext() {
        let result = extract_text("/tmp/noextension".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("暂不支持"));
    }

    #[test]
    fn test_extract_text_txt_file() {
        let d = tmp_dir();
        let txt = d.join("hello.txt");
        fs::write(&txt, "Hello World").unwrap();
        let result = extract_text(txt.to_str().unwrap().to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello World");
    }

    #[test]
    fn test_extract_text_md_file() {
        let d = tmp_dir();
        let md = d.join("readme.md");
        fs::write(&md, "# Title").unwrap();
        let result = extract_text(md.to_str().unwrap().to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "# Title");
    }

    /* ---------- P2: docx_extract_text 集成测试 ---------- */

    /// 构造最小 docx zip（word/document.xml）
    fn make_docx(path: &Path, xml: &str) {
        use std::io::Write;
        let f = fs::File::create(path).unwrap();
        let mut w = zip::ZipWriter::new(f);
        let opts = zip::write::SimpleFileOptions::default();
        w.start_file("word/document.xml", opts).unwrap();
        w.write_all(xml.as_bytes()).unwrap();
        w.finish().unwrap();
    }

    #[test]
    fn test_docx_extract_text_basic() {
        let d = tmp_dir();
        let docx = d.join("test.docx");
        let xml = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body><w:p><w:r><w:t>Hello World</w:t></w:r></w:p></w:body>
</w:document>"#;
        make_docx(&docx, xml);
        let result = extract_text(docx.to_str().unwrap().to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello World");
    }

    #[test]
    fn test_docx_extract_text_multi_paragraph() {
        let d = tmp_dir();
        let docx = d.join("test.docx");
        let xml = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>第一段</w:t></w:r></w:p>
    <w:p><w:r><w:t>第二段</w:t></w:r></w:p>
  </w:body>
</w:document>"#;
        make_docx(&docx, xml);
        let result = extract_text(docx.to_str().unwrap().to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "第一段\n第二段");
    }

    #[test]
    fn test_docx_extract_text_tab_and_br() {
        let d = tmp_dir();
        let docx = d.join("test.docx");
        let xml = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>col1</w:t></w:r><w:r><w:tab/></w:r><w:r><w:t>col2</w:t></w:r></w:p>
    <w:p><w:r><w:t>line1</w:t></w:r><w:r><w:br/></w:r><w:r><w:t>line2</w:t></w:r></w:p>
  </w:body>
</w:document>"#;
        make_docx(&docx, xml);
        let result = extract_text(docx.to_str().unwrap().to_string());
        assert!(result.is_ok());
        let text = result.unwrap();
        assert!(text.contains("col1\tcol2"), "tab 应转为制表符: {text}");
        assert!(text.contains("line1\nline2"), "br 应转为换行: {text}");
    }

    #[test]
    fn test_docx_extract_text_empty_doc_error() {
        let d = tmp_dir();
        let docx = d.join("empty.docx");
        let xml = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body></w:body>
</w:document>"#;
        make_docx(&docx, xml);
        let result = extract_text(docx.to_str().unwrap().to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("未提取到文本"));
    }

    #[test]
    fn test_docx_extract_text_multi_runs_merged() {
        let d = tmp_dir();
        let docx = d.join("test.docx");
        let xml = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>片段一</w:t></w:r><w:r><w:t>片段二</w:t></w:r></w:p>
  </w:body>
</w:document>"#;
        make_docx(&docx, xml);
        let result = extract_text(docx.to_str().unwrap().to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "片段一片段二");
    }

    /* ---------- P2: pdf_tools 端到端测试 ---------- */

    #[test]
    fn test_pdf_merge_e2e() {
        let d = tmp_dir();
        let pdf1 = d.join("a.pdf");
        let pdf2 = d.join("b.pdf");
        let out = d.join("merged.pdf");
        make_blank_pages(&pdf1, 2);
        make_blank_pages(&pdf2, 3);
        let result = pdf_merge(
            vec![pdf1.to_str().unwrap().to_string(), pdf2.to_str().unwrap().to_string()],
            out.to_str().unwrap().to_string(),
        );
        assert!(result.is_ok());
        assert!(out.exists());
        // 验证合并后页数
        let doc = lopdf::Document::load(&out).unwrap();
        assert_eq!(doc.get_pages().len(), 5);
    }

    #[test]
    fn test_pdf_compress_e2e() {
        let d = tmp_dir();
        let input = d.join("input.pdf");
        let out = d.join("compressed.pdf");
        make_blank_pages(&input, 4);
        let result = pdf_compress(
            input.to_str().unwrap().to_string(),
            out.to_str().unwrap().to_string(),
        );
        assert!(result.is_ok());
        assert!(out.exists());
        let doc = lopdf::Document::load(&out).unwrap();
        assert_eq!(doc.get_pages().len(), 4);
    }

    #[test]
    fn test_pdf_watermark_e2e() {
        // 精简系统（如 CI 的 Windows Server runner）无中文字体时优雅跳过；
        // macOS / Windows 客户端本机跑测试时正常验证水印全链路
        if crate::engine::font::load_system_font().is_err() {
            eprintln!("跳过：系统无可用中文字体（CI runner 常见）");
            return;
        }
        let d = tmp_dir();
        let input = d.join("input.pdf");
        let out = d.join("watermarked.pdf");
        make_blank_pages(&input, 2);
        let result = pdf_watermark(
            input.to_str().unwrap().to_string(),
            out.to_str().unwrap().to_string(),
            "机密".to_string(),
            0.3,
            [255, 0, 0],
            48.0,
        );
        assert!(result.is_ok());
        assert!(out.exists());
    }

    #[test]
    fn test_pdf_metadata_e2e() {
        let d = tmp_dir();
        let input = d.join("input.pdf");
        let out = d.join("meta.pdf");
        make_blank_pages(&input, 1);
        let result = pdf_metadata(
            input.to_str().unwrap().to_string(),
            out.to_str().unwrap().to_string(),
            Some("测试标题".to_string()),
            Some("测试作者".to_string()),
            None,
            None,
        );
        assert!(result.is_ok());
        assert!(out.exists());
    }

    #[test]
    fn test_pdf_rotate_e2e() {
        let d = tmp_dir();
        let input = d.join("input.pdf");
        let out = d.join("rotated.pdf");
        make_blank_pages(&input, 2);
        let result = pdf_rotate(
            input.to_str().unwrap().to_string(),
            out.to_str().unwrap().to_string(),
            90,
        );
        assert!(result.is_ok());
        assert!(out.exists());
    }

    #[test]
    fn test_pdf_encrypt_decrypt_e2e() {
        let d = tmp_dir();
        let input = d.join("input.pdf");
        let encrypted = d.join("encrypted.pdf");
        let decrypted = d.join("decrypted.pdf");
        make_blank_pages(&input, 2);
        // 加密
        let enc_result = pdf_encrypt(
            input.to_str().unwrap().to_string(),
            encrypted.to_str().unwrap().to_string(),
            "user123".to_string(),
            "owner456".to_string(),
        );
        assert!(enc_result.is_ok());
        assert!(encrypted.exists());
        // 解密
        let dec_result = pdf_decrypt(
            encrypted.to_str().unwrap().to_string(),
            decrypted.to_str().unwrap().to_string(),
            "user123".to_string(),
        );
        assert!(dec_result.is_ok());
        assert!(decrypted.exists());
    }

    #[test]
    fn test_get_pdf_page_count_e2e() {
        let d = tmp_dir();
        let input = d.join("input.pdf");
        make_blank_pages(&input, 7);
        let result = get_pdf_page_count(input.to_str().unwrap().to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 7);
    }

    /* ---------- images_to_pdf / image_compress e2e ---------- */

    /// 生成一张简单的测试图片（PNG）
    fn make_test_image(path: &Path, width: u32, height: u32) {
        let img = image::RgbImage::from_fn(width, height, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, 128u8])
        });
        img.save(path).unwrap();
    }

    #[test]
    fn test_images_to_pdf_e2e() {
        let d = tmp_dir();
        let img1 = d.join("a.png");
        let img2 = d.join("b.png");
        let out = d.join("images.pdf");
        make_test_image(&img1, 100, 80);
        make_test_image(&img2, 120, 90);
        let result = images_to_pdf(
            vec![img1.to_str().unwrap().to_string(), img2.to_str().unwrap().to_string()],
            out.to_str().unwrap().to_string(),
            "auto".to_string(),
        );
        assert!(result.is_ok());
        assert!(out.exists());
        let doc = lopdf::Document::load(&out).unwrap();
        assert_eq!(doc.get_pages().len(), 2);
    }

    #[test]
    fn test_images_to_pdf_a4_mode() {
        let d = tmp_dir();
        let img = d.join("photo.png");
        let out = d.join("a4.pdf");
        make_test_image(&img, 200, 150);
        let result = images_to_pdf(
            vec![img.to_str().unwrap().to_string()],
            out.to_str().unwrap().to_string(),
            "a4".to_string(),
        );
        assert!(result.is_ok());
        assert!(out.exists());
    }

    #[test]
    fn test_image_compress_e2e() {
        let d = tmp_dir();
        let img = d.join("photo.png");
        make_test_image(&img, 200, 200);
        let original_size = fs::metadata(&img).unwrap().len();
        // 压缩质量 50
        let result = image_compress(img.to_str().unwrap().to_string(), 50);
        assert!(result.is_ok());
        // 压缩后文件应存在
        assert!(img.exists());
        let compressed_size = fs::metadata(&img).unwrap().len();
        // 压缩后应不大于原文件（或相等如果已是最优）
        assert!(compressed_size <= original_size);
    }

    #[test]
    fn test_pdf_extract_pages_e2e() {
        let d = tmp_dir();
        let input = d.join("input.pdf");
        let out = d.join("extracted.pdf");
        make_blank_pages(&input, 5);
        // 提取第 1、3、5 页
        let result = pdf_extract_pages(
            input.to_str().unwrap().to_string(),
            out.to_str().unwrap().to_string(),
            vec![1, 3, 5],
        );
        assert!(result.is_ok());
        assert!(out.exists());
        let doc = lopdf::Document::load(&out).unwrap();
        assert_eq!(doc.get_pages().len(), 3);
    }

    #[test]
    fn test_pdf_page_numbers_e2e() {
        let d = tmp_dir();
        let input = d.join("input.pdf");
        let out = d.join("numbered.pdf");
        make_blank_pages(&input, 3);
        let result = pdf_page_numbers(
            input.to_str().unwrap().to_string(),
            out.to_str().unwrap().to_string(),
            "pageOf".to_string(),
        );
        assert!(result.is_ok());
        assert!(out.exists());
    }

    /* ---------- image_convert / image_resize ---------- */

    #[test]
    fn test_image_convert_png_to_jpg() {
        let d = tmp_dir();
        let src = d.join("src.png");
        let out = d.join("src.jpg");
        make_test_image(&src, 60, 40);
        let result = image_convert(
            src.to_str().unwrap().to_string(),
            out.to_str().unwrap().to_string(),
            85,
        );
        assert!(result.is_ok());
        assert!(out.exists());
        // 产物确实是 JPEG
        let converted = image::open(&out).unwrap();
        assert_eq!(converted.dimensions(), (60, 40));
    }

    #[test]
    fn test_image_convert_to_webp_and_bmp() {
        let d = tmp_dir();
        let src = d.join("src.png");
        make_test_image(&src, 30, 30);
        for ext in ["webp", "bmp"] {
            let out = d.join(format!("src.{}", ext));
            let result = image_convert(
                src.to_str().unwrap().to_string(),
                out.to_str().unwrap().to_string(),
                85,
            );
            assert!(result.is_ok());
            assert!(out.exists());
        }
    }

    #[test]
    fn test_image_convert_unsupported_format() {
        let d = tmp_dir();
        let src = d.join("src.png");
        make_test_image(&src, 10, 10);
        let out = d.join("src.tiff");
        let result = image_convert(
            src.to_str().unwrap().to_string(),
            out.to_str().unwrap().to_string(),
            85,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_image_resize_by_width() {
        let d = tmp_dir();
        let src = d.join("src.png");
        let out = d.join("resized.png");
        make_test_image(&src, 200, 100);
        // 只给宽，高传 0 → 等比缩放
        let result = image_resize(
            src.to_str().unwrap().to_string(),
            out.to_str().unwrap().to_string(),
            100,
            0,
        );
        assert!(result.is_ok());
        let resized = image::open(&out).unwrap();
        assert_eq!(resized.dimensions(), (100, 50));
    }

    #[test]
    fn test_image_resize_by_height() {
        let d = tmp_dir();
        let src = d.join("src.png");
        let out = d.join("resized.png");
        make_test_image(&src, 200, 100);
        // 只给高，宽传 0 → 等比缩放
        let result = image_resize(
            src.to_str().unwrap().to_string(),
            out.to_str().unwrap().to_string(),
            0,
            50,
        );
        assert!(result.is_ok());
        let resized = image::open(&out).unwrap();
        assert_eq!(resized.dimensions(), (100, 50));
    }

    #[test]
    fn test_image_resize_exact_and_invalid() {
        let d = tmp_dir();
        let src = d.join("src.png");
        make_test_image(&src, 200, 100);
        // 宽高都给 → 精确拉伸
        let out = d.join("exact.png");
        let result = image_resize(
            src.to_str().unwrap().to_string(),
            out.to_str().unwrap().to_string(),
            80,
            90,
        );
        assert!(result.is_ok());
        assert_eq!(image::open(&out).unwrap().dimensions(), (80, 90));
        // 宽高都为 0 → 报错
        let invalid = image_resize(
            src.to_str().unwrap().to_string(),
            d.join("bad.png").to_str().unwrap().to_string(),
            0,
            0,
        );
        assert!(invalid.is_err());
    }

    #[test]
    fn test_pdf_sign_ok() {
        let d = tmp_dir();
        let src = d.join("doc.pdf");
        make_blank_pages(&src, 2);
        let sig = d.join("sig.png");
        make_test_image(&sig, 120, 40);
        let out = d.join("signed.pdf");
        let result = pdf_sign(
            src.to_str().unwrap().to_string(),
            out.to_str().unwrap().to_string(),
            sig.to_str().unwrap().to_string(),
            1,
            70.0,
            10.0,
            20.0,
        );
        assert!(result.is_ok());
        assert!(out.exists());
        // 签名后页数不变，目标页新增 ImSig 图像资源
        let doc = lopdf::Document::load(&out).unwrap();
        assert_eq!(doc.get_pages().len(), 2);
        let page_id = *doc.get_pages().get(&1).unwrap();
        let page = doc.get_dictionary(page_id).unwrap();
        let xobj = page
            .get(b"Resources")
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"XObject")
            .unwrap()
            .as_dict()
            .unwrap();
        assert!(xobj.get(b"ImSig").is_ok());
    }

    #[test]
    fn test_pdf_sign_invalid_params() {
        let d = tmp_dir();
        let src = d.join("doc.pdf");
        make_blank_pages(&src, 2);
        let sig = d.join("sig.png");
        make_test_image(&sig, 60, 20);
        // 页码越界
        let out_of_range = pdf_sign(
            src.to_str().unwrap().to_string(),
            d.join("o1.pdf").to_str().unwrap().to_string(),
            sig.to_str().unwrap().to_string(),
            5,
            10.0,
            10.0,
            20.0,
        );
        assert!(out_of_range.is_err());
        // 百分比越界
        let bad_pct = pdf_sign(
            src.to_str().unwrap().to_string(),
            d.join("o2.pdf").to_str().unwrap().to_string(),
            sig.to_str().unwrap().to_string(),
            1,
            120.0,
            10.0,
            20.0,
        );
        assert!(bad_pct.is_err());
    }
}
