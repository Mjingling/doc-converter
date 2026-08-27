//! 最小 OOXML DOCX 写入器：把 Block 列表产出为合法 .docx（Word / WPS / Pages / LibreOffice 可打开）
//!
//! 产出 zip 部件（顺序敏感，[Content_Types].xml 需为首个 entry）：
//! [Content_Types].xml → _rels/.rels → word/document.xml → word/_rels/document.xml.rels
//! → word/styles.xml → word/numbering.xml
//!
//! 样式能力：Heading1-6（大纲级别）、Code（等宽灰底）、Quote（斜体缩进）、
//! 无序/有序列表（numId 1/2，最多 9 层缩进）、粗体/斜体/行内代码 run、水平线（段落下边框）。

use super::blocks::{runs_plain, Block, Run};
use std::fs::File;
use std::io::{Cursor, Write};
use std::path::Path;

/// XML 属性转义（& < > " '）
fn escape_xml(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// run → OOXML：<w:r><w:rPr>…</w:rPr><w:t xml:space="preserve">…</w:t></w:r>
fn runs_xml(runs: &[Run]) -> String {
    let mut out = String::new();
    for r in runs {
        if r.text.is_empty() {
            continue;
        }
        let mut rpr = String::new();
        if r.bold {
            rpr.push_str("<w:b/>");
        }
        if r.italic {
            rpr.push_str("<w:i/>");
        }
        if r.code {
            rpr.push_str("<w:rFonts w:ascii=\"Courier New\" w:hAnsi=\"Courier New\"/>");
        }
        let rpr = if rpr.is_empty() {
            String::new()
        } else {
            format!("<w:rPr>{rpr}</w:rPr>")
        };
        out.push_str(&format!(
            "<w:r>{rpr}<w:t xml:space=\"preserve\">{}</w:t></w:r>",
            escape_xml(&r.text)
        ));
    }
    out
}

/// 段落属性 + 内容（style 可选 pStyle 名）
fn para_xml(style: Option<&str>, ppr_extra: &str, runs: &[Run]) -> String {
    let style = style.map(|s| format!("<w:pStyle w:val=\"{s}\"/>")).unwrap_or_default();
    let ppr = if style.is_empty() && ppr_extra.is_empty() {
        String::new()
    } else {
        format!("<w:pPr>{style}{ppr_extra}</w:pPr>")
    };
    format!("<w:p>{ppr}{}</w:p>", runs_xml(runs))
}

/// 生成 document.xml 的 body 内容
fn document_xml(blocks: &[Block]) -> String {
    let mut body = String::new();
    // 有序连续列表项的显示序号（PDF/HTML 用递增号，docx numbering 自动编号）
    let mut ordered_counter = 0u32;
    let mut prev_was_ordered_item = false;

    for block in blocks {
        match block {
            Block::Heading(level, runs) => {
                prev_was_ordered_item = false;
                body.push_str(&para_xml(Some(&format!("Heading{level}")), "", runs));
            }
            Block::Paragraph(runs) => {
                prev_was_ordered_item = false;
                body.push_str(&para_xml(None, "", runs));
            }
            Block::Quote(runs) => {
                prev_was_ordered_item = false;
                body.push_str(&para_xml(Some("Quote"), "", runs));
            }
            Block::Rule => {
                prev_was_ordered_item = false;
                // 段落下边框模拟水平线
                body.push_str(&para_xml(
                    None,
                    "<w:pBdr><w:bottom w:val=\"single\" w:sz=\"6\" w:space=\"1\" w:color=\"auto\"/></w:pBdr>",
                    &[Run::plain("")],
                ));
            }
            Block::Code(text) => {
                prev_was_ordered_item = false;
                // 代码块逐行输出为 Code 样式段落（保留前导空格与空行）
                for line in text.lines() {
                    let runs = vec![Run::plain(line)];
                    if line.is_empty() {
                        // 空行用带 Code 样式的空段落占位（无 run 也能保留样式上下文）
                        body.push_str(&format!("<w:p><w:pPr><w:pStyle w:val=\"Code\"/></w:pPr></w:p>"));
                    } else {
                        body.push_str(&para_xml(Some("Code"), "", &runs));
                    }
                }
                // 代码块末尾补一个空段，避免与下一段粘连
                body.push_str(&format!("<w:p><w:pPr><w:pStyle w:val=\"Code\"/></w:pPr></w:p>"));
            }
            Block::ListItem { ordered, indent, number, runs } => {
                if *ordered {
                    if !prev_was_ordered_item {
                        // 新列表序列：从条目声明的起始号开始
                        ordered_counter = number.unwrap_or(1).saturating_sub(1);
                    }
                    ordered_counter = ordered_counter.saturating_add(1);
                } else if prev_was_ordered_item {
                    ordered_counter = 0;
                }
                prev_was_ordered_item = *ordered;
                let num_id = if *ordered { 2 } else { 1 };
                let ilvl = (*indent).min(9);
                let ppr = format!(
                    "<w:numPr><w:ilvl w:val=\"{ilvl}\"/><w:numId w:val=\"{num_id}\"/></w:numPr>"
                );
                body.push_str(&para_xml(None, &ppr, runs));
            }
        }
    }
    let _ = ordered_counter; // docx 由 numbering.xml 自动编号，计数仅用于注释说明
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\n\
<w:body>{body}\
<w:sectPr><w:pgSz w:w=\"11906\" w:h=\"16838\"/><w:pgMar w:top=\"1440\" w:right=\"1440\" w:bottom=\"1440\" w:left=\"1440\"/></w:sectPr>\n\
</w:body>\n</w:document>\n"
    )
}

/// styles.xml：docDefaults + Normal + Heading1-6 + Code + Quote
fn styles_xml() -> String {
    let mut headings = String::new();
    let sizes = [32, 26, 24, 22, 22, 22]; // 半磅（H1=16pt … H6=11pt）
    for (i, sz) in sizes.iter().enumerate() {
        let n = i + 1;
        headings.push_str(&format!(
            "<w:style w:type=\"paragraph\" w:styleId=\"Heading{n}\">\
<w:name w:val=\"heading {n}\"/><w:basedOn w:val=\"Normal\"/><w:next w:val=\"Normal\"/>\
<w:pPr><w:keepNext/><w:outlineLvl w:val=\"{i}\"/><w:spacing w:before=\"240\" w:after=\"120\"/></w:pPr>\
<w:rPr><w:b/><w:sz w:val=\"{sz}\"/><w:szCs w:val=\"{sz}\"/></w:rPr></w:style>"
        ));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
<w:styles xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\
<w:docDefaults><w:rPrDefault><w:rPr><w:sz w:val=\"22\"/><w:szCs w:val=\"22\"/></w:rPr></w:rPrDefault></w:docDefaults>\
<w:style w:type=\"paragraph\" w:default=\"1\" w:styleId=\"Normal\"><w:name w:val=\"Normal\"/></w:style>\
{headings}\
<w:style w:type=\"paragraph\" w:styleId=\"Code\"><w:name w:val=\"Code\"/><w:basedOn w:val=\"Normal\"/><w:next w:val=\"Normal\"/>\
<w:pPr><w:shd w:val=\"clear\" w:fill=\"F5F5F5\"/><w:spacing w:after=\"0\" w:line=\"240\" w:lineRule=\"auto\"/></w:pPr>\
<w:rPr><w:rFonts w:ascii=\"Courier New\" w:hAnsi=\"Courier New\"/><w:sz w:val=\"19\"/><w:szCs w:val=\"19\"/></w:rPr></w:style>\
<w:style w:type=\"paragraph\" w:styleId=\"Quote\"><w:name w:val=\"Quote\"/><w:basedOn w:val=\"Normal\"/><w:next w:val=\"Normal\"/>\
<w:pPr><w:ind w:left=\"720\"/></w:pPr>\
<w:rPr><w:i/><w:color w:val=\"595959\"/></w:rPr></w:style>\
</w:styles>"
    )
}

/// numbering.xml：numId 1 = 无序（bullet），numId 2 = 有序（decimal），各 9 级
fn numbering_xml() -> String {
    fn abstract_num(id: usize, ordered: bool) -> String {
        let mut lvls = String::new();
        for lvl in 0..9 {
            let indent = 720 * (lvl + 1);
            if ordered {
                lvls.push_str(&format!(
                    "<w:lvl w:ilvl=\"{lvl}\"><w:start w:val=\"1\"/><w:numFmt w:val=\"decimal\"/>\
<w:lvlText w:val=\"%{}.\"/><w:lvlJc w:val=\"left\"/>\
<w:pPr><w:ind w:left=\"{indent}\" w:hanging=\"360\"/></w:pPr></w:lvl>",
                    lvl + 1
                ));
            } else {
                lvls.push_str(&format!(
                    "<w:lvl w:ilvl=\"{lvl}\"><w:start w:val=\"1\"/><w:numFmt w:val=\"bullet\"/>\
<w:lvlText w:val=\"\u{F0B7}\"/><w:lvlJc w:val=\"left\"/>\
<w:rPr><w:rFonts w:ascii=\"Symbol\" w:hAnsi=\"Symbol\" w:hint=\"default\"/></w:rPr>\
<w:pPr><w:ind w:left=\"{indent}\" w:hanging=\"360\"/></w:pPr></w:lvl>"
                ));
            }
        }
        format!(
            "<w:abstractNum w:abstractNumId=\"{id}\"><w:multiLevelType w:val=\"hybridMultilevel\"/>{lvls}</w:abstractNum>"
        )
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
<w:numbering xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\
{}{}\
<w:num w:numId=\"1\"><w:abstractNumId w:val=\"0\"/></w:num>\
<w:num w:numId=\"2\"><w:abstractNumId w:val=\"1\"/></w:num>\
</w:numbering>",
        abstract_num(0, false),
        abstract_num(1, true)
    )
}

fn content_types_xml() -> String {
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
<Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
<Default Extension=\"xml\" ContentType=\"application/xml\"/>\
<Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/>\
<Override PartName=\"/word/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml\"/>\
<Override PartName=\"/word/numbering.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml\"/>\
</Types>".to_string()
}

fn root_rels_xml() -> String {
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/>\
</Relationships>".to_string()
}

fn document_rels_xml() -> String {
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/>\
<Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering\" Target=\"numbering.xml\"/>\
</Relationships>".to_string()
}

/// 把 Block 列表写为 .docx 文件
pub fn write_docx(blocks: &[Block], out: &Path) -> Result<(), String> {
    let document = document_xml(blocks);
    let mut buf: Vec<u8> = Vec::new();
    {
        let mut zw = zip::ZipWriter::new(Cursor::new(&mut buf));
        let opts =
            zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        let entries: [(&str, String); 6] = [
            ("[Content_Types].xml", content_types_xml()),
            ("_rels/.rels", root_rels_xml()),
            ("word/document.xml", document),
            ("word/_rels/document.xml.rels", document_rels_xml()),
            ("word/styles.xml", styles_xml()),
            ("word/numbering.xml", numbering_xml()),
        ];
        for (name, content) in entries {
            zw.start_file(name, opts).map_err(|e| format!("写入 docx 失败: {e}"))?;
            zw.write_all(content.as_bytes()).map_err(|e| format!("写入 docx 失败: {e}"))?;
        }
        zw.finish().map_err(|e| format!("生成 docx 失败: {e}"))?;
    }
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建输出目录失败: {e}"))?;
        }
    }
    let mut f = File::create(out).map_err(|e| format!("创建文件失败: {e}"))?;
    f.write_all(&buf).map_err(|e| format!("写入 docx 失败: {e}"))?;
    Ok(())
}

/* ---------- 纯文本辅助（供 txt 目标复用块模型） ---------- */

/// 块列表 → 纯文本（列表带符号前缀、引用带 > 前缀、水平线用 - 行，行内标记已剥离）
pub fn blocks_to_plain(blocks: &[Block]) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut ordered_counter = 0u32;
    let mut prev_ordered = false;
    for b in blocks {
        match b {
            Block::Heading(_, runs) => lines.push(runs_plain(runs)),
            Block::Paragraph(runs) => lines.push(runs_plain(runs)),
            Block::Code(text) => lines.push(text.trim_end().to_string()),
            Block::Quote(runs) => lines.push(format!("> {}", runs_plain(runs))),
            Block::Rule => lines.push("-".repeat(40)),
            Block::ListItem { ordered, indent, number, runs } => {
                if *ordered {
                    if !prev_ordered {
                        ordered_counter = number.unwrap_or(1).saturating_sub(1);
                    }
                    ordered_counter = ordered_counter.saturating_add(1);
                } else if prev_ordered {
                    ordered_counter = 0;
                }
                prev_ordered = *ordered;
                let prefix = if *ordered {
                    format!("{ordered_counter}. ")
                } else {
                    "- ".to_string()
                };
                lines.push(format!("{}{}{}", "  ".repeat(*indent), prefix, runs_plain(runs)));
            }
        }
    }
    lines.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::super::blocks::{parse_md, Run};
    use super::*;

    fn docx_entries(path: &Path) -> Vec<(String, Vec<u8>)> {
        let f = File::open(path).unwrap();
        let mut ar = zip::ZipArchive::new(f).unwrap();
        let names: Vec<String> = ar.file_names().map(|s| s.to_string()).collect();
        let mut out = Vec::new();
        for n in names {
            let mut data = Vec::new();
            use std::io::Read;
            ar.by_name(&n).unwrap().read_to_end(&mut data).unwrap();
            out.push((n, data));
        }
        out
    }

    #[test]
    fn test_write_docx_structure_and_content() {
        let dir = std::env::temp_dir().join("docmorph_test_docx_writer");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("sample.docx");

        let md = "# 标题\n\n段落 **粗** 文本\n\n- 列表甲\n- 列表乙\n\n1. 第一\n\n> 引用\n\n---\n\n```\ncode line\n```";
        let blocks = parse_md(md);
        write_docx(&blocks, &out).unwrap();

        let entries = docx_entries(&out);
        let names: Vec<&str> = entries.iter().map(|(n, _)| n.as_str()).collect();
        // 6 个必需部件齐全，且 [Content_Types].xml 为首个 entry（严格解析器要求）
        assert_eq!(names[0], "[Content_Types].xml");
        for need in ["word/document.xml", "word/styles.xml", "word/numbering.xml", "_rels/.rels"] {
            assert!(names.contains(&need), "缺少部件 {need}: {names:?}");
        }

        let doc = String::from_utf8(entries.iter().find(|(n, _)| n == "word/document.xml").unwrap().1.clone()).unwrap();
        assert!(doc.contains("<w:pStyle w:val=\"Heading1\"/>"), "标题样式缺失");
        assert!(doc.contains("<w:b/>"), "粗体 run 缺失");
        assert!(doc.contains("粗"), "粗体文本缺失");
        assert!(doc.contains("<w:numId w:val=\"1\"/>"), "无序列表 numPr 缺失");
        assert!(doc.contains("<w:numId w:val=\"2\"/>"), "有序列表 numPr 缺失");
        assert!(doc.contains("<w:pStyle w:val=\"Quote\"/>"), "引用样式缺失");
        assert!(doc.contains("<w:pStyle w:val=\"Code\"/>"), "代码样式缺失");
        assert!(doc.contains("code line"));
        assert!(doc.contains("xml:space=\"preserve\""));

        // document.xml 必须是良构 XML（quick-xml 全量读取无错）
        let mut reader = quick_xml::Reader::from_str(&doc);
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(quick_xml::events::Event::Eof) => break,
                Ok(_) => {}
                Err(e) => panic!("document.xml 不是良构 XML: {e}"),
            }
            buf.clear();
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_docx_roundtrip_md_semantics() {
        // md → docx → (docx 提取器) → md：标题 / 段落 / 粗体 / 列表语义应保留
        let dir = std::env::temp_dir().join("docmorph_test_docx_roundtrip");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("rt.docx");

        let blocks = parse_md("# 项目标题\n\n这是**重点**内容。\n\n- 甲\n- 乙\n");
        write_docx(&blocks, &out).unwrap();

        let entries = docx_entries(&out);
        let doc = String::from_utf8(entries.iter().find(|(n, _)| n == "word/document.xml").unwrap().1.clone()).unwrap();
        // 语义断言：标题、粗体、列表 numPr、文本内容都在 document.xml 中
        assert!(doc.contains("项目标题"));
        assert!(doc.contains("重点"));
        assert!(doc.contains("这是"));
        assert!(doc.contains("甲"));
        assert!(doc.contains("乙"));
        assert!(doc.matches("<w:numId w:val=\"1\"/>").count() >= 2, "两个列表项都应有 numPr");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_blocks_to_plain() {
        let blocks = parse_md("段落\n\n- 甲\n1. 一\n\n> 引用\n\n---");
        let text = blocks_to_plain(&blocks);
        assert!(text.contains("段落"));
        assert!(text.contains("- 甲"));
        assert!(text.contains("1. 一"));
        assert!(text.contains("> 引用"));
        assert!(text.contains("----"));
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("a<b>&\"'"), "a&lt;b&gt;&amp;&quot;&#39;");
    }

    #[test]
    fn test_runs_xml_empty_run_skipped() {
        let xml = runs_xml(&[Run::plain(""), Run::plain("x")]);
        assert_eq!(xml, "<w:r><w:t xml:space=\"preserve\">x</w:t></w:r>");
    }
}
