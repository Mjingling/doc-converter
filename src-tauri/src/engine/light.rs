//! 轻量内置转换引擎：零外部依赖，直接解析 Office 文档内部 XML
//!
//! 支持（仅文本/数据提取，不做版式渲染，与 LibreOffice 引擎互补）：
//! - docx → txt / html / md（段落 + Heading 标题识别）
//! - xlsx → csv（第一个工作表，支持共享字符串）
//! - pptx → txt（逐页提取文本）

use quick_xml::events::Event;
use quick_xml::Reader;
use lopdf::content::{Content, Operation};
use lopdf::{Dictionary, Document, Object, ObjectId, Stream};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

/// 轻量转换入口：按输入扩展名与目标扩展名分发，返回输出文件路径
pub fn convert_light(input: &Path, target_ext: &str, out_dir: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(out_dir).map_err(|e| format!("创建输出目录失败: {}", e))?;
    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "无效的文件名".to_string())?;
    let out = out_dir.join(format!("{}.{}", stem, target_ext));
    match (ext.as_str(), target_ext) {
        ("docx", "txt") => extract_docx(input, &out, DocxTarget::Txt)?,
        ("docx", "html") => extract_docx(input, &out, DocxTarget::Html)?,
        ("docx", "md") => extract_docx(input, &out, DocxTarget::Md)?,
        ("xlsx", "csv") => extract_xlsx(input, &out)?,
        ("pptx", "txt") => extract_pptx(input, &out)?,
        ("epub", "txt") => extract_epub_book(input, &out, "txt")?,
        ("epub", "html") => extract_epub_book(input, &out, "html")?,
        ("epub", "md") => extract_epub_book(input, &out, "md")?,
        ("txt", "pdf") => txt_to_pdf(input, &out)?,
        ("md", "pdf") => md_to_pdf(input, &out)?,
        ("html", "pdf") => html_to_pdf(input, &out)?,
        _ => {
            return Err(format!(
                "内置引擎暂不支持 {} → {}，请切换到 LibreOffice 引擎",
                ext, target_ext
            ))
        }
    }
    Ok(out)
}

/* ---------- docx → txt / html / md ---------- */

enum DocxTarget {
    Txt,
    Html,
    Md,
}

/// 段落：level > 0 表示 Heading 级别（0 = 普通段落）
struct Para {
    level: u8,
    text: String,
}

fn extract_docx(input: &Path, out: &Path, target: DocxTarget) -> Result<(), String> {
    let paras = read_docx_paras(input)?;
    let rendered = match target {
        DocxTarget::Txt => paras
            .iter()
            .filter(|p| !p.text.is_empty())
            .map(|p| p.text.trim().to_string())
            .collect::<Vec<_>>()
            .join("\n\n"),
        DocxTarget::Md => paras
            .iter()
            .filter(|p| !p.text.is_empty())
            .map(|p| {
                if p.level > 0 {
                    format!("{} {}", "#".repeat(p.level as usize), p.text.trim())
                } else {
                    p.text.trim().to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n\n"),
        DocxTarget::Html => paras
            .iter()
            .filter(|p| !p.text.is_empty())
            .map(|p| {
                if p.level > 0 {
                    format!("<h{}>{}</h{}>", p.level, escape_html(p.text.trim()), p.level)
                } else {
                    format!("<p>{}</p>", escape_html(p.text.trim()))
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
    };
    fs::write(out, rendered).map_err(|e| format!("保存失败: {}", e))
}

/// 读取 docx 的 word/document.xml，提取段落与标题级别
fn read_docx_paras(input: &Path) -> Result<Vec<Para>, String> {
    let file = File::open(input).map_err(|e| format!("读取文件失败: {}", e))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("无法解析 docx（zip 格式）: {}", e))?;
    let mut doc = archive
        .by_name("word/document.xml")
        .map_err(|_| "docx 中缺少 word/document.xml".to_string())?;
    let mut xml = String::new();
    doc.read_to_string(&mut xml)
        .map_err(|e| format!("读取文档内容失败: {}", e))?;

    let mut reader = Reader::from_str(&xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut paras: Vec<Para> = Vec::new();
    let mut in_text = false;
    let mut level = 0u8;
    let mut current = String::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"w:p" => {
                    level = 0;
                    current.clear();
                }
                // 标题级别：w:pStyle w:val="HeadingN"
                b"w:pStyle" => {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"w:val" {
                            if let Ok(v) = attr.unescape_value() {
                                if let Some(rest) = v.strip_prefix("Heading") {
                                    level = rest
                                        .chars()
                                        .next()
                                        .and_then(|c| c.to_digit(10))
                                        .unwrap_or(0) as u8;
                                }
                            }
                        }
                    }
                }
                b"w:t" => in_text = true,
                _ => {}
            },
            Ok(Event::Empty(e)) => match e.name().as_ref() {
                // 自闭合标签（如 <w:pStyle …/>）同样需要读取属性
                b"w:pStyle" => {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"w:val" {
                            if let Ok(v) = attr.unescape_value() {
                                if let Some(rest) = v.strip_prefix("Heading") {
                                    level = rest
                                        .chars()
                                        .next()
                                        .and_then(|c| c.to_digit(10))
                                        .unwrap_or(0) as u8;
                                }
                            }
                        }
                    }
                }
                _ => {}
            },
            Ok(Event::Text(t)) => {
                if in_text {
                    if let Ok(s) = t.unescape() {
                        current.push_str(&s);
                    }
                }
            }
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"w:t" => in_text = false,
                b"w:p" => {
                    paras.push(Para {
                        level,
                        text: std::mem::take(&mut current),
                    });
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(paras)
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/* ---------- xlsx → csv ---------- */

fn extract_xlsx(input: &Path, out: &Path) -> Result<(), String> {
    let file = File::open(input).map_err(|e| format!("读取文件失败: {}", e))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("无法解析 xlsx（zip 格式）: {}", e))?;
    let shared = read_shared_strings(&mut archive)?;
    let rows = read_sheet1(&mut archive, &shared)?;
    let mut csv = String::new();
    for row in &rows {
        let line: Vec<String> = row.iter().map(|f| csv_field(f)).collect();
        csv.push_str(&line.join(","));
        csv.push_str("\r\n");
    }
    fs::write(out, csv).map_err(|e| format!("保存失败: {}", e))
}

/// 共享字符串表：xl/sharedStrings.xml 中每个 <si> 的文本
fn read_shared_strings(archive: &mut zip::ZipArchive<File>) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    let mut xml = String::new();
    // 纯数字表格可能没有共享字符串表
    match archive.by_name("xl/sharedStrings.xml") {
        Ok(mut f) => {
            f.read_to_string(&mut xml)
                .map_err(|e| format!("读取共享字符串失败: {}", e))?;
        }
        Err(_) => return Ok(out),
    }
    let mut reader = Reader::from_str(&xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut in_si = false;
    let mut in_t = false;
    let mut current = String::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"si" => {
                    in_si = true;
                    current.clear();
                }
                b"t" if in_si => in_t = true,
                _ => {}
            },
            Ok(Event::Text(t)) => {
                if in_t {
                    if let Ok(s) = t.unescape() {
                        current.push_str(&s);
                    }
                }
            }
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"t" => in_t = false,
                b"si" => {
                    out.push(std::mem::take(&mut current));
                    in_si = false;
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(out)
}

/// 读取第一个工作表 xl/worksheets/sheet1.xml，输出行数据
fn read_sheet1(
    archive: &mut zip::ZipArchive<File>,
    shared: &[String],
) -> Result<Vec<Vec<String>>, String> {
    let mut xml = String::new();
    match archive.by_name("xl/worksheets/sheet1.xml") {
        Ok(mut f) => {
            f.read_to_string(&mut xml)
                .map_err(|e| format!("读取工作表失败: {}", e))?;
        }
        Err(_) => return Err("xlsx 中缺少工作表 sheet1".to_string()),
    }

    let mut reader = Reader::from_str(&xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut rows: Vec<Vec<String>> = Vec::new();
    // 当前行单元格（按列索引存放）
    let mut cells: Vec<Option<String>> = Vec::new();
    let mut is_shared = false;
    let mut is_inline = false;
    let mut in_inline = false;
    let mut in_t = false;
    let mut in_v = false;
    let mut col = 0usize;
    let mut current = String::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"row" => cells.clear(),
                b"c" => {
                    is_shared = false;
                    is_inline = false;
                    col = 0;
                    for attr in e.attributes().flatten() {
                        match attr.key.as_ref() {
                            // 单元格引用如 "B2"：字母部分转列索引
                            b"r" => {
                                if let Ok(v) = attr.unescape_value() {
                                    col = col_index(&v);
                                }
                            }
                            b"t" => {
                                if let Ok(v) = attr.unescape_value() {
                                    is_shared = v == "s";
                                    is_inline = v == "inlineStr";
                                }
                            }
                            _ => {}
                        }
                    }
                }
                // 内联字符串：<c t="inlineStr"><is><t>…</t></is></c>
                b"is" if is_inline => {
                    in_inline = true;
                    current.clear();
                }
                b"t" if in_inline && !in_v => in_t = true,
                b"v" => {
                    in_v = true;
                    current.clear();
                }
                _ => {}
            },
            Ok(Event::Text(t)) => {
                if in_v || (in_inline && in_t) {
                    if let Ok(s) = t.unescape() {
                        current.push_str(&s);
                    }
                }
            }
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"t" => in_t = false,
                b"is" => {
                    in_inline = false;
                    set_cell(&mut cells, col, std::mem::take(&mut current));
                }
                b"v" => {
                    in_v = false;
                    let value = if is_shared {
                        current
                            .parse::<usize>()
                            .ok()
                            .and_then(|i| shared.get(i))
                            .cloned()
                            .unwrap_or_default()
                    } else {
                        current.clone()
                    };
                    set_cell(&mut cells, col, value);
                }
                b"row" => {
                    rows.push(
                        cells
                            .iter()
                            .map(|c| c.clone().unwrap_or_default())
                            .collect(),
                    );
                    cells.clear();
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(rows)
}

/// 写入单元格（列索引超出时补空）
fn set_cell(cells: &mut Vec<Option<String>>, col: usize, value: String) {
    while cells.len() <= col {
        cells.push(None);
    }
    cells[col] = Some(value);
}

/// 单元格引用 "B2" 的字母部分 → 列索引（A=0, B=1, …）
fn col_index(reference: &str) -> usize {
    reference
        .bytes()
        .take_while(|b| b.is_ascii_alphabetic())
        .fold(0usize, |acc, b| {
            acc * 26 + (b.to_ascii_uppercase() - b'A') as usize + 1
        })
        .saturating_sub(1)
}

fn csv_field(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') || field.contains('\r') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

/* ---------- pptx → txt ---------- */

fn extract_pptx(input: &Path, out: &Path) -> Result<(), String> {
    let file = File::open(input).map_err(|e| format!("读取文件失败: {}", e))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("无法解析 pptx（zip 格式）: {}", e))?;
    // 收集幻灯片文件并按页号排序（slide10 需排在 slide2 之后）
    let mut slides: Vec<(u32, String)> = Vec::new();
    for name in archive.file_names() {
        if let Some(rest) = name.strip_prefix("ppt/slides/slide") {
            if let Some(num_str) = rest.strip_suffix(".xml") {
                if let Ok(n) = num_str.parse::<u32>() {
                    slides.push((n, name.to_string()));
                }
            }
        }
    }
    slides.sort_by_key(|(n, _)| *n);
    let mut result = String::new();
    for (_, name) in slides {
        let mut xml = String::new();
        match archive.by_name(&name) {
            Ok(mut f) => {
                f.read_to_string(&mut xml)
                    .map_err(|e| format!("读取幻灯片失败: {}", e))?;
            }
            Err(_) => return Err(format!("读取幻灯片失败: {}", name)),
        }
        let page = extract_pptx_page(&xml);
        if !page.trim().is_empty() {
            result.push_str(page.trim());
            result.push('\n');
        }
    }
    fs::write(out, result).map_err(|e| format!("保存失败: {}", e))
}

/// 提取单页幻灯片的文本（a:p 段落以换行分隔）
fn extract_pptx_page(xml: &str) -> String {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut in_p = false;
    let mut in_t = false;
    let mut current = String::new();
    let mut page = String::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"a:p" => {
                    in_p = true;
                    current.clear();
                }
                b"a:t" if in_p => in_t = true,
                _ => {}
            },
            Ok(Event::Text(t)) => {
                if in_t {
                    if let Ok(s) = t.unescape() {
                        current.push_str(&s);
                    }
                }
            }
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"a:t" => in_t = false,
                b"a:p" => {
                    page.push_str(current.trim());
                    page.push('\n');
                    in_p = false;
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    page
}

/* ---------- EPUB → txt / html / md ---------- */

/// 提取 EPUB 正文（OPF 中书脊顺序的 XHTML 内容），输出 txt / html / md
fn extract_epub_book(input: &Path, out: &Path, target: &str) -> Result<(), String> {
    let file = File::open(input).map_err(|e| format!("读取文件失败: {}", e))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("无法解析 epub（zip 格式）: {}", e))?;

    // 查找 container.xml 获取 OPF 路径
    let mut container_xml = String::new();
    archive
        .by_name("META-INF/container.xml")
        .map_err(|_| "epub 缺少 META-INF/container.xml".to_string())?
        .read_to_string(&mut container_xml)
        .map_err(|e| format!("读取 container.xml 失败: {}", e))?;
    let opf_path = parse_container(&container_xml)?;

    // 读取 OPF 获取 spine 顺序
    let mut opf_xml = String::new();
    let opf_path = url_decode(&opf_path);
    archive
        .by_name(&opf_path)
        .map_err(|_| format!("epub 中缺少 {}", opf_path))?
        .read_to_string(&mut opf_xml)
        .map_err(|e| format!("读取 OPF 失败: {}", e))?;
    let opf_dir = opf_path.rsplit_once('/').map(|(d, _)| format!("{}/", d)).unwrap_or_default();
    let spine = parse_opf(&opf_xml, &opf_dir)?;

    // 读取每个 spine 条目并拼接
    let mut rendered = String::new();
    for href in &spine {
        let mut xml = String::new();
        if let Ok(mut f) = archive.by_name(href) {
            f.read_to_string(&mut xml).map_err(|e| format!("读取 {} 失败: {}", href, e))?;
        } else {
            continue;
        }
        let text = extract_xhtml_text(&xml);
        if !text.is_empty() {
            match target {
                "html" => rendered.push_str(&format!("<section>\n{}</section>\n\n", text)),
                "md" => rendered.push_str(&format!("\n{}\n\n", text)),
                _ => {
                    rendered.push_str(&text);
                    rendered.push('\n');
                }
            }
        }
    }
    fs::write(out, rendered).map_err(|e| format!("保存失败: {}", e))
}

/// 从 container.xml 解析 OPF 路径
fn parse_container(xml: &str) -> Result<String, String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(e)) | Ok(Event::Start(e)) if e.name().as_ref() == b"rootfile" => {
                for attr in e.attributes().flatten() {
                    if attr.key.as_ref() == b"full-path" {
                        if let Ok(v) = attr.unescape_value() {
                            return Ok(v.to_string());
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    Err("container.xml 中未找到 rootfile 元素".to_string())
}

/// 从 OPF 解析书脊顺序（spine itemref → manifest item href）
fn parse_opf(xml: &str, opf_dir: &str) -> Result<Vec<String>, String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    // manifest: id → href
    let mut manifest: Vec<(String, String)> = Vec::new();
    // spine: 按顺序的 idref
    let mut spine_ids: Vec<String> = Vec::new();
    let mut in_manifest = false;
    let mut in_spine = false;
    loop {
        match reader.read_event_into(&mut buf) {
            // 自闭合标签（<item .../>）产生 Empty 事件，与 Start 一样处理
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                match e.name().as_ref() {
                    b"manifest" => in_manifest = true,
                    b"spine" => in_spine = true,
                    b"item" if in_manifest => {
                        let mut id = String::new();
                        let mut href = String::new();
                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"id" => id = attr.unescape_value().unwrap_or_default().to_string(),
                                // href 可能含 URL 编码（如 %20 空格），解码后拼接
                                b"href" => href = url_decode(&attr.unescape_value().unwrap_or_default().to_string()),
                                _ => {}
                            }
                        }
                        if !id.is_empty() && !href.is_empty() {
                            manifest.push((id, format!("{}{}", opf_dir, href)));
                        }
                    }
                    b"itemref" if in_spine => {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"idref" {
                                if let Ok(v) = attr.unescape_value() {
                                    spine_ids.push(v.to_string());
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                match e.name().as_ref() {
                    b"manifest" => in_manifest = false,
                    b"spine" => in_spine = false,
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    // 按 spine 顺序查 manifest 获取 href
    let mut result: Vec<String> = Vec::new();
    for id in &spine_ids {
        if let Some((_, href)) = manifest.iter().find(|(i, _)| i == id) {
            result.push(href.clone());
        }
    }
    if result.is_empty() {
        return Err("OPF 中未找到有效书脊条目".to_string());
    }
    Ok(result)
}

/// URL 百分号解码（%20 → 空格等），OPF 的 href 可能对路径进行编码
fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// 从 XHTML 提取纯文本（去除标签）
fn extract_xhtml_text(xml: &str) -> String {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut text = String::new();
    let mut skip = 0u32; // 跳过 style/script 内容
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let name = e.name();
                let tag = name.as_ref();
                if tag == b"style" || tag == b"script" { skip += 1; }
                if tag == b"br" || tag == b"p" || tag == b"div" || tag == b"h1" || tag == b"h2"
                    || tag == b"h3" || tag == b"h4" || tag == b"h5" || tag == b"h6"
                    || tag == b"li" || tag == b"tr"
                {
                    if !text.ends_with('\n') { text.push('\n'); }
                }
                if tag == b"td" || tag == b"th" { text.push('\t'); }
            }
            Ok(Event::End(e)) => {
                let name = e.name();
                let tag = name.as_ref();
                if tag == b"style" || tag == b"script" { skip = skip.saturating_sub(1); }
                if tag == b"p" || tag == b"div" || tag == b"h1" || tag == b"h2" || tag == b"h3"
                    || tag == b"h4" || tag == b"h5" || tag == b"h6"
                    || tag == b"li" || tag == b"th" || tag == b"td"
                {
                    if !text.ends_with('\n') { text.push('\n'); }
                }
            }
            Ok(Event::Text(t)) => {
                if skip == 0 {
                    if let Ok(s) = t.unescape() {
                        text.push_str(&s);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    text
}

/* ---------- TXT / HTML / MD → PDF ---------- */

const PAGE_W: f32 = 595.0;
const PAGE_H: f32 = 842.0;
const MARGIN: f32 = 56.0;
const LEADING: f32 = 14.0;
/// Courier 在 10pt 时每字符约 6pt（600/1000 字宽）
const COURIER_W: f32 = 6.0;
const HELV_W: f32 = 5.5; // 近似

/// 将文本块列表渲染为 PDF 并保存
fn render_blocks_to_pdf(blocks: &[Block], out: &Path) -> Result<(), String> {
    let mut doc = Document::with_version("1.4");
    let mut page_ids: Vec<ObjectId> = Vec::new();
    let mut y = PAGE_H - MARGIN;
    let mut page_ops: Vec<Operation> = Vec::new();
    let mut font_name = "Courier";
    let mut font_size = 10.0;

    fn commit_page(doc: &mut Document, page_ids: &mut Vec<ObjectId>, ops: &mut Vec<Operation>, fname: &str, fsize: f32) {
        if ops.is_empty() { return; }
        let mut all = Vec::new();
        all.push(Operation::new("BT", vec![]));
        all.push(Operation::new("Tf", vec![
            Object::Name(fname.as_bytes().to_vec()),
            Object::Real(fsize),
        ]));
        all.extend(ops.drain(..));
        all.push(Operation::new("ET", vec![]));
        let content = Content { operations: all }.encode().unwrap_or_default();
        let mut stream = Stream::new(Dictionary::new(), content);
        stream.compress().ok();
        let content_id = doc.add_object(Object::Stream(stream));
        let font_dict = Dictionary::from_iter(vec![
            (b"F1".to_vec(), Object::Dictionary(Dictionary::from_iter(vec![
                (b"Type".to_vec(), Object::Name(b"Font".to_vec())),
                (b"Subtype".to_vec(), Object::Name(b"Type1".to_vec())),
                (b"BaseFont".to_vec(), Object::Name(fname.as_bytes().to_vec())),
            ]))),
        ]);
        let page = Dictionary::from_iter(vec![
            (b"Type".to_vec(), Object::Name(b"Page".to_vec())),
            (b"MediaBox".to_vec(), Object::Array(vec![
                Object::Integer(0), Object::Integer(0),
                Object::Real(PAGE_W), Object::Real(PAGE_H),
            ])),
            (b"Resources".to_vec(), Object::Dictionary(Dictionary::from_iter(vec![
                (b"Font".to_vec(), Object::Dictionary(font_dict)),
            ]))),
            (b"Contents".to_vec(), Object::Reference(content_id)),
        ]);
        page_ids.push(doc.add_object(Object::Dictionary(page)));
    }

    for block in blocks {
        match block {
            Block::Paragraph(text) => {
                font_name = "Courier";
                font_size = 10.0;
                let cw = COURIER_W;
                let max = ((PAGE_W - 2.0 * MARGIN) / cw).floor() as usize;
                let lines = wrap_text(text, max);
                for line in &lines {
                    if y - LEADING < MARGIN {
                        commit_page(&mut doc, &mut page_ids, &mut page_ops, font_name, font_size);
                        y = PAGE_H - MARGIN;
                    }
                    page_ops.push(Operation::new("Td", vec![
                        Object::Real(MARGIN),
                        Object::Real(y - font_size),
                    ]));
                    page_ops.push(Operation::new("Tj", vec![
                        Object::String(line.as_bytes().to_vec(), lopdf::StringFormat::Literal),
                    ]));
                    y -= LEADING;
                }
            }
            Block::Heading(level, text) => {
                font_name = "Helvetica-Bold";
                font_size = (18.0 - (level.saturating_sub(1) as f32) * 2.0).max(12.0);
                let cw = 5.5; // Helvetica approx
                let max = ((PAGE_W - 2.0 * MARGIN) / cw).floor() as usize;
                if y - font_size - 4.0 < MARGIN {
                    commit_page(&mut doc, &mut page_ids, &mut page_ops, font_name, font_size);
                    y = PAGE_H - MARGIN;
                }
                y -= 4.0;
                page_ops.push(Operation::new("Td", vec![
                    Object::Real(MARGIN),
                    Object::Real(y - font_size),
                ]));
                let display = if text.len() > max { &text[..max] } else { text.as_str() };
                page_ops.push(Operation::new("Tj", vec![
                    Object::String(display.as_bytes().to_vec(), lopdf::StringFormat::Literal),
                ]));
                y -= font_size + 4.0;
            }
            Block::Code(text) => {
                font_name = "Courier";
                font_size = 9.0;
                let cw = 5.4;
                let max = ((PAGE_W - 2.0 * MARGIN) / cw).floor() as usize;
                let lines = wrap_text(text, max);
                for line in &lines {
                    if y - LEADING < MARGIN {
                        commit_page(&mut doc, &mut page_ids, &mut page_ops, font_name, font_size);
                        y = PAGE_H - MARGIN;
                    }
                    page_ops.push(Operation::new("Td", vec![
                        Object::Real(MARGIN + 8.0),
                        Object::Real(y - font_size),
                    ]));
                    page_ops.push(Operation::new("Tj", vec![
                        Object::String(line.as_bytes().to_vec(), lopdf::StringFormat::Literal),
                    ]));
                    y -= LEADING;
                }
            }
        }
    }
    commit_page(&mut doc, &mut page_ids, &mut page_ops, font_name, font_size);

    // 重建根页树
    let kids: Vec<Object> = page_ids.iter().map(|id| Object::Reference(*id)).collect();
    let pages = Dictionary::from_iter(vec![
        (b"Type".to_vec(), Object::Name(b"Pages".to_vec())),
        (b"Kids".to_vec(), Object::Array(kids)),
        (b"Count".to_vec(), Object::Integer(page_ids.len() as i64)),
    ]);
    let pages_id = doc.add_object(pages);
    let catalog_id = match doc.trailer.get(b"Root").and_then(|o| o.as_reference()) {
        Ok(id) => id,
        Err(_) => {
            let catalog = Dictionary::from_iter(vec![
                (b"Type".to_vec(), Object::Name(b"Catalog".to_vec())),
            ]);
            let id = doc.add_object(Object::Dictionary(catalog));
            doc.trailer.set("Root", Object::Reference(id));
            id
        }
    };
    if let Ok(catalog) = doc.get_dictionary_mut(catalog_id) {
        catalog.set("Pages", Object::Reference(pages_id));
    }
    doc.save(out).map_err(|e| format!("保存 PDF 失败: {}", e))?;
    Ok(())
}

enum Block {
    Paragraph(String),
    Heading(u8, String),
    Code(String),
}

/// 简单按字符数截断换行（等宽字体）
fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    if max_chars == 0 { return vec![text.to_string()]; }
    let mut lines = Vec::new();
    for line in text.lines() {
        if line.len() <= max_chars {
            lines.push(line.to_string());
        } else {
            let mut start = 0;
            while start < line.len() {
                let end = (start + max_chars).min(line.len());
                // 尽量在空格处断行
                if end < line.len() {
                    if let Some(space) = line[start..end].rfind(' ') {
                        lines.push(line[start..start + space + 1].trim_end().to_string());
                        start = start + space + 1;
                        continue;
                    }
                }
                lines.push(line[start..end].to_string());
                start = end;
            }
        }
    }
    lines
}

/// 简单 Markdown 解析器
fn parse_md(text: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut in_code = false;
    let mut code_buf = String::new();
    for line in text.lines() {
        if line.starts_with("```") {
            if in_code {
                blocks.push(Block::Code(std::mem::take(&mut code_buf)));
                in_code = false;
            } else {
                in_code = true;
                code_buf.clear();
            }
            continue;
        }
        if in_code {
            code_buf.push_str(line);
            code_buf.push('\n');
            continue;
        }
        if let Some(rest) = line.strip_prefix("# ") {
            blocks.push(Block::Heading(1, rest.to_string()));
        } else if let Some(rest) = line.strip_prefix("## ") {
            blocks.push(Block::Heading(2, rest.to_string()));
        } else if let Some(rest) = line.strip_prefix("### ") {
            blocks.push(Block::Heading(3, rest.to_string()));
        } else if let Some(rest) = line.strip_prefix("#### ") {
            blocks.push(Block::Heading(4, rest.to_string()));
        } else if let Some(rest) = line.strip_prefix("##### ") {
            blocks.push(Block::Heading(5, rest.to_string()));
        } else if let Some(rest) = line.strip_prefix("###### ") {
            blocks.push(Block::Heading(6, rest.to_string()));
        } else if line.trim().is_empty() {
            // 空行分隔段落，但不需要额外操作
        } else if let Some(prev) = blocks.last_mut() {
            if let Block::Paragraph(ref mut p) = prev {
                p.push(' ');
                p.push_str(line);
            } else {
                blocks.push(Block::Paragraph(line.to_string()));
            }
        } else {
            blocks.push(Block::Paragraph(line.to_string()));
        }
    }
    if in_code && !code_buf.is_empty() {
        blocks.push(Block::Code(code_buf));
    }
    blocks
}

/// 简单 HTML 解析器
fn parse_html(xml: &str) -> Vec<Block> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut blocks = Vec::new();
    let mut text_buf = String::new();
    let mut in_code = false;
    let mut code_buf = String::new();
    let mut heading_level = 0u8;
    let mut _in_heading = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let name = e.name();
                let tag = name.as_ref();
                match tag {
                    b if b.len() == 2 && b[0] == b'h' && b[1] >= b'1' && b[1] <= b'6' => {
                        flush_text(&mut blocks, &mut text_buf, heading_level);
                        heading_level = tag[1] - b'0';
                        _in_heading = true;
                    }
                    b"p" | b"div" | b"li" | b"body" => {
                        flush_text(&mut blocks, &mut text_buf, heading_level);
                        heading_level = 0;
                    }
                    b"pre" | b"code" => {
                        flush_text(&mut blocks, &mut text_buf, heading_level);
                        in_code = true;
                        code_buf.clear();
                    }
                    b"br" => text_buf.push('\n'),
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let name = e.name();
                let tag = name.as_ref();
                match tag {
                    b if b.len() == 2 && b[0] == b'h' && b[1] >= b'1' && b[1] <= b'6' => {
                        flush_text(&mut blocks, &mut text_buf, heading_level);
                        heading_level = 0;
                        _in_heading = false;
                    }
                    b"p" | b"div" | b"li" => {
                        flush_text(&mut blocks, &mut text_buf, heading_level);
                    }
                    b"pre" | b"code" if in_code => {
                        blocks.push(Block::Code(std::mem::take(&mut code_buf)));
                        in_code = false;
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(t)) => {
                if let Ok(s) = t.unescape() {
                    if in_code {
                        code_buf.push_str(&s);
                    } else {
                        text_buf.push_str(&s);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    flush_text(&mut blocks, &mut text_buf, heading_level);
    blocks
}

fn flush_text(blocks: &mut Vec<Block>, buf: &mut String, level: u8) {
    let s = buf.trim().to_string();
    buf.clear();
    if s.is_empty() { return; }
    if level > 0 {
        blocks.push(Block::Heading(level, s));
    } else {
        blocks.push(Block::Paragraph(s));
    }
}

/// TXT → PDF（每行视为段落）
pub fn txt_to_pdf(input: &Path, out: &Path) -> Result<(), String> {
    let text = fs::read_to_string(input).map_err(|e| format!("读取文件失败: {}", e))?;
    let blocks: Vec<Block> = text.lines()
        .map(|l| Block::Paragraph(l.to_string()))
        .collect();
    render_blocks_to_pdf(&blocks, out)
}

/// MD → PDF（解析标题、段落、代码块）
pub fn md_to_pdf(input: &Path, out: &Path) -> Result<(), String> {
    let text = fs::read_to_string(input).map_err(|e| format!("读取文件失败: {}", e))?;
    let blocks = parse_md(&text);
    render_blocks_to_pdf(&blocks, out)
}

/// HTML → PDF（解析 h1~h6、p、pre/code）
pub fn html_to_pdf(input: &Path, out: &Path) -> Result<(), String> {
    let text = fs::read_to_string(input).map_err(|e| format!("读取文件失败: {}", e))?;
    let blocks = parse_html(&text);
    render_blocks_to_pdf(&blocks, out)
}

/* ---------- docx 提取图片 ---------- */

/// 提取 docx 中嵌入的图片到输出目录，返回图片路径列表
pub fn extract_docx_images(input: &Path, out_dir: &Path) -> Result<Vec<String>, String> {
    fs::create_dir_all(out_dir).map_err(|e| format!("创建输出目录失败: {}", e))?;
    let file = File::open(input).map_err(|e| format!("读取文件失败: {}", e))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("无法解析 docx（zip 格式）: {}", e))?;
    let mut outputs = Vec::new();
    let names: Vec<String> = archive.file_names().map(|s| s.to_string()).collect();
    for name in &names {
        // docx 的媒体文件通常在 word/media/ 下
        if !name.starts_with("word/media/") && !name.starts_with("media/") { continue; }
        let mut entry = archive.by_name(name).map_err(|e| format!("读取 {} 失败: {}", name, e))?;
        let fname = name.rsplit_once('/').map(|(_, f)| f).unwrap_or(name);
        let out_path = out_dir.join(fname);
        let mut data = Vec::new();
        entry.read_to_end(&mut data).map_err(|e| format!("读取 {} 失败: {}", name, e))?;
        fs::write(&out_path, &data).map_err(|e| format!("保存 {} 失败: {}", fname, e))?;
        outputs.push(out_path.to_string_lossy().to_string());
    }
    if outputs.is_empty() {
        return Err("文档中未找到嵌入图片".to_string());
    }
    Ok(outputs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// 每个测试独立的临时目录序号（并行测试避免互相删除文件）
    static TMP_SEQ: AtomicU32 = AtomicU32::new(0);

    /// 构造一个最小 zip（OOXML 结构），entries 为 (路径, 内容)
    fn make_zip(path: &Path, entries: &[(&str, &str)]) {
        let f = File::create(path).unwrap();
        let mut w = zip::ZipWriter::new(f);
        let opts = zip::write::SimpleFileOptions::default();
        for (name, content) in entries {
            w.start_file(*name, opts).unwrap();
            w.write_all(content.as_bytes()).unwrap();
        }
        w.finish().unwrap();
    }

    const DOCX_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>标题一</w:t></w:r></w:p>
    <w:p><w:r><w:t>第一段，包含</w:t></w:r><w:r><w:t>两个片段</w:t></w:r></w:p>
    <w:p><w:r><w:t>第二段</w:t></w:r></w:p>
  </w:body>
</w:document>"#;

    const SHARED_XML: &str = r#"<?xml version="1.0"?>
<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <si><t>姓名</t></si>
  <si><t>张三, 测试</t></si>
  <si><t>年龄</t></si>
</sst>"#;

    const SHEET_XML: &str = r#"<?xml version="1.0"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData>
    <row r="1"><c r="A1" t="s"><v>0</v></c><c r="B1" t="s"><v>2</v></c></row>
    <row r="2"><c r="A2" t="s"><v>1</v></c><c r="B2"><v>28</v></c></row>
  </sheetData>
</worksheet>"#;

    const SLIDE_XML: &str = r#"<?xml version="1.0"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:sp><p:txBody><a:p><a:r><a:t>第一页标题</a:t></a:r></a:p><a:p><a:r><a:t>第一页正文</a:t></a:r></a:p></p:txBody></p:sp>
  </p:spTree></p:cSld>
</p:sld>"#;

    fn tmp_dir() -> PathBuf {
        let n = TMP_SEQ.fetch_add(1, Ordering::Relaxed);
        let d = std::env::temp_dir().join(format!("light_test_{}_{}", std::process::id(), n));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn test_docx_to_md() {
        let d = tmp_dir();
        let src = d.join("sample.docx");
        make_zip(&src, &[("word/document.xml", DOCX_XML)]);
        let out = convert_light(&src, "md", &d).unwrap();
        let text = fs::read_to_string(&out).unwrap();
        assert!(text.contains("# 标题一"), "标题应转为 markdown: {text}");
        assert!(text.contains("第一段，包含两个片段"), "片段应合并: {text}");
        assert!(text.contains("第二段"));
    }

    #[test]
    fn test_docx_to_html() {
        let d = tmp_dir();
        let src = d.join("sample.docx");
        make_zip(&src, &[("word/document.xml", DOCX_XML)]);
        let out = convert_light(&src, "html", &d).unwrap();
        let text = fs::read_to_string(&out).unwrap();
        assert!(text.contains("<h1>标题一</h1>"), "标题应转为 h1: {text}");
        assert!(text.contains("<p>第一段，包含两个片段</p>"));
    }

    #[test]
    fn test_xlsx_to_csv() {
        let d = tmp_dir();
        let src = d.join("sample.xlsx");
        make_zip(
            &src,
            &[
                ("xl/sharedStrings.xml", SHARED_XML),
                ("xl/worksheets/sheet1.xml", SHEET_XML),
            ],
        );
        let out = convert_light(&src, "csv", &d).unwrap();
        let csv = fs::read_to_string(&out).unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[0], "姓名,年龄", "首行为表头: {csv}");
        assert_eq!(lines[1], "\"张三, 测试\",28", "含逗号字段应转义: {csv}");
    }

    #[test]
    fn test_pptx_to_txt() {
        let d = tmp_dir();
        let src = d.join("sample.pptx");
        make_zip(&src, &[("ppt/slides/slide1.xml", SLIDE_XML)]);
        let out = convert_light(&src, "txt", &d).unwrap();
        let text = fs::read_to_string(&out).unwrap();
        assert!(text.contains("第一页标题"), "应提取幻灯片文本: {text}");
        assert!(text.contains("第一页正文"));
    }

    #[test]
    fn test_xlsx_inline_str() {
        let d = tmp_dir();
        let src = d.join("inline.xlsx");
        make_zip(
            &src,
            &[(
                "xl/worksheets/sheet1.xml",
                r#"<?xml version="1.0"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData>
    <row r="1"><c r="A1" t="inlineStr"><is><t>内联文本</t></is></c><c r="B1"><v>42</v></c></row>
  </sheetData>
</worksheet>"#,
            )],
        );
        let out = convert_light(&src, "csv", &d).unwrap();
        let csv = fs::read_to_string(&out).unwrap();
        assert_eq!(csv.trim(), "内联文本,42", "inlineStr 应被提取: {csv}");
    }

    #[test]
    fn test_unsupported_combination() {
        let d = tmp_dir();
        let src = d.join("sample.pdf");
        fs::write(&src, "not a pdf").unwrap();
        let err = convert_light(&src, "txt", &d).unwrap_err();
        assert!(err.contains("LibreOffice"), "应提示切换到 LibreOffice: {err}");
    }
}
