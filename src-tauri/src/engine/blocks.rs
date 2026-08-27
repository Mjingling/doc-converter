//! 块级文档模型与解析器：Markdown / HTML → 结构化块（Block/Run）
//!
//! 供 docx 写入器（docx_writer）、HTML 渲染、PDF 降级渲染共用。
//! 支持的块级：标题、段落、代码块、引用、有序/无序列表（含嵌套缩进）、水平线；
//! 行内：**粗体**、*斜体*、`行内代码`（未闭合标记按字面文本处理）。

/// 行内文本片段：标记位描述样式（可组合，如粗斜体）
#[derive(Debug, Clone, PartialEq)]
pub struct Run {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub code: bool,
}

impl Run {
    pub fn plain(text: impl Into<String>) -> Self {
        Run { text: text.into(), bold: false, italic: false, code: false }
    }
}

/// 块级元素
#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    /// 标题（级别 1-6）
    Heading(u8, Vec<Run>),
    Paragraph(Vec<Run>),
    /// 代码块（原文，含换行）
    Code(String),
    /// 引用块
    Quote(Vec<Run>),
    /// 列表项：ordered=true 为有序；indent 为嵌套层级（0 起）；number 为有序列表起始序号
    ListItem { ordered: bool, indent: usize, number: Option<u32>, runs: Vec<Run> },
    /// 水平分割线
    Rule,
}

/// 把片段列表拍平为纯文本（PDF 降级渲染 / 纯文本目标用）
pub fn runs_plain(runs: &[Run]) -> String {
    runs.iter().map(|r| r.text.as_str()).collect()
}

/* ---------- 行内解析 ---------- */

/// 行内解析：先切 `` ` `` 行内代码（未闭合按普通文本），再做 *斜体* / **粗体**
pub fn parse_inline(s: &str) -> Vec<Run> {
    let mut runs = Vec::new();
    let mut rest = s;
    while let Some(start) = rest.find('`') {
        // 找配对反引号；未闭合则整体按普通文本处理
        match rest[start + 1..].find('`') {
            Some(len) => {
                let end = start + 1 + len;
                if start > 0 {
                    runs.extend(parse_emphasis(&rest[..start]));
                }
                runs.push(Run {
                    text: rest[start + 1..end].to_string(),
                    bold: false,
                    italic: false,
                    code: true,
                });
                rest = &rest[end + 1..];
            }
            None => break,
        }
    }
    runs.extend(parse_emphasis(rest));
    normalize(runs)
}

/// 粗体/斜体解析：标记必须成对出现；打开前向后找配对，未闭合的标记按字面字符保留
fn parse_emphasis(s: &str) -> Vec<Run> {
    let chars: Vec<char> = s.chars().collect();
    let mut runs = Vec::new();
    let mut bold = false;
    let mut italic = false;
    let mut buf = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '*' {
            let double = i + 1 < chars.len() && chars[i + 1] == '*';
            let (marker_len, marker): (usize, &str) = if double { (2, "**") } else { (1, "*") };
            let remaining: String = chars[i + marker_len..].iter().collect();
            // 已处于对应样式时任何同类标记都是关闭符；否则向后找配对才能打开
            let is_toggle = if double {
                bold || remaining.contains("**")
            } else {
                italic || remaining.contains('*')
            };
            if is_toggle {
                if !buf.is_empty() {
                    runs.push(Run { text: std::mem::take(&mut buf), bold, italic, code: false });
                }
                if double {
                    bold = !bold;
                } else {
                    italic = !italic;
                }
            } else {
                // 未闭合：按字面字符保留
                buf.push_str(marker);
            }
            i += marker_len;
        } else {
            buf.push(chars[i]);
            i += 1;
        }
    }
    if !buf.is_empty() {
        runs.push(Run { text: buf, bold, italic, code: false });
    }
    runs
}

/// 合并相邻同样式片段、剔除空片段
fn normalize(runs: Vec<Run>) -> Vec<Run> {
    let mut out: Vec<Run> = Vec::new();
    for r in runs {
        if r.text.is_empty() {
            continue;
        }
        if let Some(last) = out.last_mut() {
            if last.bold == r.bold && last.italic == r.italic && last.code == r.code {
                last.text.push_str(&r.text);
                continue;
            }
        }
        out.push(r);
    }
    out
}

/* ---------- Markdown 解析 ---------- */

/// Markdown 解析器：标题 / 段落 / 代码块 / 引用 / 列表（嵌套缩进）/ 水平线 / 行内样式
pub fn parse_md(text: &str) -> Vec<Block> {
    let mut blocks: Vec<Block> = Vec::new();
    let mut para_buf: Vec<String> = Vec::new();
    let mut in_code = false;
    let mut code_buf = String::new();

    macro_rules! flush_para {
        () => {
            if !para_buf.is_empty() {
                let joined = para_buf.join(" ");
                para_buf.clear();
                blocks.push(Block::Paragraph(parse_inline(&joined)));
            }
        };
    }

    for line in text.lines() {
        // 围栏代码块：``` 开关（支持 ```lang）
        if line.trim_start().starts_with("```") {
            if in_code {
                blocks.push(Block::Code(std::mem::take(&mut code_buf)));
                in_code = false;
            } else {
                flush_para!();
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

        // 空行：段落结束
        if line.trim().is_empty() {
            flush_para!();
            continue;
        }

        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();

        // 标题（ATX 风格 # ~ ######）
        if let Some(rest) = strip_heading(trimmed) {
            flush_para!();
            blocks.push(Block::Heading(rest.0, parse_inline(rest.1)));
            continue;
        }
        // 水平线：--- / *** / ___（3 个及以上）
        if is_rule(trimmed) {
            flush_para!();
            blocks.push(Block::Rule);
            continue;
        }
        // 引用：> 或 > 开头（嵌套引用按一层处理）
        if let Some(rest) = trimmed.strip_prefix('>') {
            flush_para!();
            let content = rest.strip_prefix(' ').unwrap_or(rest);
            blocks.push(Block::Quote(parse_inline(content)));
            continue;
        }
        // 列表项：- / * / + 无序；1. 有序（数字可任意）
        if let Some((ordered, number, text)) = parse_list_item(trimmed) {
            flush_para!();
            blocks.push(Block::ListItem {
                ordered,
                // 每 2 个空格缩进算一层（容错：>=2 空格即一层，4 空格两层）
                indent: (indent / 2).min(9),
                number,
                runs: parse_inline(&text),
            });
            continue;
        }

        // 普通文本：连续行合并为段落（延续到空行或块级边界）
        para_buf.push(trimmed.to_string());
    }
    flush_para!();
    if in_code && !code_buf.is_empty() {
        blocks.push(Block::Code(code_buf));
    }
    blocks
}

/// 提取 ATX 标题：返回 (级别, 内容)
fn strip_heading(line: &str) -> Option<(u8, &str)> {
    let mut level = 0u8;
    for c in line.chars() {
        if c == '#' {
            level += 1;
        } else {
            break;
        }
    }
    if level == 0 || level > 6 {
        return None;
    }
    let rest = &line[level as usize..];
    let rest = rest.strip_prefix(' ').unwrap_or(rest);
    // 行尾闭合 # （如 "## 标题 ##"）可省略
    let rest = rest.trim_end().trim_end_matches('#').trim_end();
    Some((level, rest))
}

/// 判断水平线：--- / *** / ___ 三个及以上
fn is_rule(line: &str) -> bool {
    let t = line.trim();
    if t.len() < 3 {
        return false;
    }
    let first = t.chars().next().unwrap();
    (first == '-' || first == '*' || first == '_') && t.chars().all(|c| c == first)
}

/// 解析列表项：返回 (有序, 序号, 内容)
fn parse_list_item(line: &str) -> Option<(bool, Option<u32>, String)> {
    let bytes = line.as_bytes();
    // 无序：- / * / + 后跟空格
    if bytes.len() >= 2 && (bytes[0] == b'-' || bytes[0] == b'*' || bytes[0] == b'+') && bytes[1] == b' ' {
        let content = line[2..].trim().to_string();
        if !content.is_empty() {
            return Some((false, None, content));
        }
    }
    // 有序：数字 + . 或 ) 后跟空格
    let dot = line.find(". ").or_else(|| line.find(") "));
    if let Some(pos) = dot {
        let digits = &line[..pos];
        if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) {
            if let Ok(n) = digits.parse::<u32>() {
                let content = line[pos + 2..].trim().to_string();
                if !content.is_empty() {
                    return Some((true, Some(n), content));
                }
            }
        }
    }
    None
}

/* ---------- HTML 转义与渲染 ---------- */

/// HTML 文本转义（& < > " '）
pub fn escape_html(s: &str) -> String {
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

fn inline_html(runs: &[Run]) -> String {
    runs.iter()
        .map(|r| {
            let t = escape_html(&r.text);
            if r.code {
                format!("<code>{t}</code>")
            } else {
                let mut s = t;
                if r.bold {
                    s = format!("<strong>{s}</strong>");
                }
                if r.italic {
                    s = format!("<em>{s}</em>");
                }
                s
            }
        })
        .collect()
}

/// 块列表 → HTML 片段（无骨架；docx→html 目标用，与历史输出形态一致）
pub fn blocks_to_html_body(blocks: &[Block]) -> String {
    let mut body = String::new();
    // 当前打开的列表是否有序（None = 无列表打开）；连续同类型列表项归入同一容器
    let mut in_list: Option<bool> = None;

    for block in blocks {
        match block {
            Block::ListItem { ordered, indent: _, number, runs } => {
                // 缩进层级在 HTML 输出中拍平（v1 限制）
                if in_list != Some(*ordered) {
                    if let Some(prev) = in_list.take() {
                        body.push_str(if prev { "</ol>\n" } else { "</ul>\n" });
                    }
                    let tag = if *ordered { "ol" } else { "ul" };
                    // 有序且首项序号非 1 时带 start 属性
                    let start = if *ordered && *number != Some(1) {
                        format!(" start=\"{}\"", number.unwrap_or(1))
                    } else {
                        String::new()
                    };
                    body.push_str(&format!("<{tag}{start}>\n"));
                    in_list = Some(*ordered);
                }
                body.push_str(&format!("<li>{}</li>\n", inline_html(runs)));
            }
            Block::Heading(level, runs) => {
                close_list(&mut body, &mut in_list);
                body.push_str(&format!("<h{level}>{}</h{level}>\n", inline_html(runs)));
            }
            Block::Paragraph(runs) => {
                close_list(&mut body, &mut in_list);
                body.push_str(&format!("<p>{}</p>\n", inline_html(runs)));
            }
            Block::Code(text) => {
                close_list(&mut body, &mut in_list);
                body.push_str(&format!("<pre><code>{}</code></pre>\n", escape_html(text)));
            }
            Block::Quote(runs) => {
                close_list(&mut body, &mut in_list);
                body.push_str(&format!("<blockquote><p>{}</p></blockquote>\n", inline_html(runs)));
            }
            Block::Rule => {
                close_list(&mut body, &mut in_list);
                body.push_str("<hr>\n");
            }
        }
    }
    close_list(&mut body, &mut in_list);
    body
}

/// 块列表 → 完整 HTML 文档（含 charset 骨架，浏览器可直接打开）
pub fn blocks_to_html(blocks: &[Block], title: &str) -> String {
    format!(
        "<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<title>{}</title>\n</head>\n<body>\n{}</body>\n</html>\n",
        escape_html(title),
        blocks_to_html_body(blocks)
    )
}

fn close_list(body: &mut String, in_list: &mut Option<bool>) {
    if let Some(ordered) = in_list.take() {
        body.push_str(if ordered { "</ol>\n" } else { "</ul>\n" });
    }
}

/* ---------- 块列表 → Markdown / HTML 片段（docx 提取目标复用） ---------- */

fn inline_md(runs: &[Run]) -> String {
    runs.iter()
        .map(|r| {
            if r.code {
                format!("`{}`", r.text)
            } else {
                let mut s = r.text.clone();
                if r.bold {
                    s = format!("**{s}**");
                }
                if r.italic {
                    s = format!("*{s}*");
                }
                s
            }
        })
        .collect()
}

/// 块列表 → Markdown（连续列表项单换行连接，其余块之间空行分隔）
pub fn blocks_to_md(blocks: &[Block]) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut ordered_counter = 0u32;
    let mut prev_ordered = false;
    for b in blocks {
        match b {
            Block::Heading(level, runs) => {
                lines.push(format!("{} {}", "#".repeat(*level as usize), inline_md(runs)));
                prev_ordered = false;
            }
            Block::Paragraph(runs) => {
                lines.push(inline_md(runs));
                prev_ordered = false;
            }
            Block::Code(text) => {
                lines.push(format!("```\n{}\n```", text.trim_end()));
                prev_ordered = false;
            }
            Block::Quote(runs) => {
                lines.push(format!("> {}", inline_md(runs)));
                prev_ordered = false;
            }
            Block::Rule => {
                lines.push("---".to_string());
                prev_ordered = false;
            }
            Block::ListItem { ordered, indent, number, runs } => {
                if *ordered {
                    if !prev_ordered {
                        ordered_counter = number.unwrap_or(1).saturating_sub(1);
                    }
                    ordered_counter = ordered_counter.saturating_add(1);
                } else if prev_ordered {
                    ordered_counter = 0;
                }
                prev_ordered = true;
                let prefix = if *ordered { format!("{ordered_counter}. ") } else { "- ".to_string() };
                lines.push(format!("{}{}{}", "  ".repeat(*indent), prefix, inline_md(runs)));
            }
        }
    }
    // 连续列表行（前缀为缩进+列表符号）用单换行，其余用空行
    let mut out = String::new();
    let mut prev_was_list = false;
    for line in lines {
        let is_list = line.trim_start().starts_with("- ") || line.trim_start().starts_with(|c: char| c.is_ascii_digit());
        if !out.is_empty() && !(is_list && prev_was_list) {
            out.push('\n');
        }
        out.push_str(&line);
        out.push('\n');
        prev_was_list = is_list;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_inline_basic() {
        let runs = parse_inline("普通 **粗体** *斜体* `代码`");
        assert_eq!(runs.len(), 6);
        assert_eq!(runs[0], Run::plain("普通 "));
        assert!(runs[1].bold && !runs[1].italic);
        assert_eq!(runs[1].text, "粗体");
        assert!(runs[3].italic && !runs[3].bold);
        assert!(runs[5].code);
        assert_eq!(runs[5].text, "代码");
    }

    #[test]
    fn test_parse_inline_unclosed_marker_kept_literal() {
        // 未闭合 ** 保留字面字符
        let runs = parse_inline("a ** b");
        assert_eq!(runs_plain(&runs), "a ** b");
        // 未闭合 ` 保留
        let runs = parse_inline("x ` y");
        assert_eq!(runs_plain(&runs), "x ` y");
    }

    #[test]
    fn test_parse_inline_bold_italic_combo() {
        let runs = parse_inline("***粗斜***");
        let last = runs.last().unwrap();
        assert!(last.bold && last.italic, "{:?}", runs);
    }

    #[test]
    fn test_parse_md_headings_and_code() {
        let md = "# 标题一\n\n段落一\n\n```\nfn main() {}\n```\n";
        let blocks = parse_md(md);
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0], Block::Heading(1, vec![Run::plain("标题一")]));
        assert!(matches!(&blocks[2], Block::Code(c) if c.contains("fn main()")));
    }

    #[test]
    fn test_parse_md_lists_nested() {
        let md = "- 甲\n- 乙\n  - 乙一\n1. 第一\n2. 第二\n";
        let blocks = parse_md(md);
        assert_eq!(blocks.len(), 5);
        match &blocks[0] {
            Block::ListItem { ordered, indent, number, .. } => {
                assert!(!*ordered && *indent == 0 && number.is_none());
            }
            other => panic!("应为列表项: {:?}", other),
        }
        match &blocks[2] {
            Block::ListItem { indent, .. } => assert_eq!(*indent, 1, "两空格缩进应算一层"),
            other => panic!("应为嵌套列表项: {:?}", other),
        }
        match &blocks[4] {
            Block::ListItem { ordered, number, .. } => {
                assert!(*ordered && *number == Some(2));
            }
            other => panic!("应为有序列表项: {:?}", other),
        }
    }

    #[test]
    fn test_parse_md_quote_and_rule() {
        let md = "> 引用内容\n\n---\n";
        let blocks = parse_md(md);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0], Block::Quote(vec![Run::plain("引用内容")]));
        assert_eq!(blocks[1], Block::Rule);
    }

    #[test]
    fn test_parse_md_paragraph_merge() {
        let md = "第一行\n第二行\n\n新段落";
        let blocks = parse_md(md);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0], Block::Paragraph(vec![Run::plain("第一行 第二行")]));
    }

    #[test]
    fn test_parse_md_inline_in_heading() {
        let blocks = parse_md("## 带 **粗** 的标题");
        match &blocks[0] {
            Block::Heading(level, runs) => {
                assert_eq!(*level, 2);
                assert!(runs.iter().any(|r| r.bold));
            }
            other => panic!("应为标题: {:?}", other),
        }
    }

    #[test]
    fn test_blocks_to_html_structure() {
        let blocks = parse_md("# 标题\n\n- 甲\n- 乙\n\n1. 一\n\n**粗** 段落");
        let html = blocks_to_html(&blocks, "测试");
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<meta charset=\"utf-8\">"));
        assert!(html.contains("<h1>标题</h1>"));
        assert!(html.contains("<ul>"));
        assert!(html.contains("<li>甲</li>"));
        assert!(html.contains("<ol>"));
        assert!(html.contains("<strong>粗</strong>"));
        assert!(html.contains("</ul>"));
    }

    #[test]
    fn test_runs_plain() {
        let runs = vec![Run::plain("a"), Run::plain("b"), Run { text: "c".into(), bold: true, italic: false, code: false }];
        assert_eq!(runs_plain(&runs), "abc");
    }
}
