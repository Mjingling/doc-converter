//! 格式支持矩阵：每种输入格式可转换的目标格式列表
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Doc,
    Docx,
    Odt,
    Rtf,
    Txt,
    Html,
    Md,
    Xls,
    Xlsx,
    Ods,
    Csv,
    Ppt,
    Pptx,
    Odp,
    Pdf,
    Png,
    Jpg,
    Epub,
}

impl Format {
    /// 根据文件扩展名识别格式
    pub fn from_ext(ext: &str) -> Option<Format> {
        Some(match ext.to_lowercase().as_str() {
            "doc" => Format::Doc,
            "docx" => Format::Docx,
            "odt" => Format::Odt,
            "rtf" => Format::Rtf,
            "txt" => Format::Txt,
            "html" | "htm" => Format::Html,
            "md" | "markdown" => Format::Md,
            "xls" => Format::Xls,
            "xlsx" => Format::Xlsx,
            "ods" => Format::Ods,
            "csv" => Format::Csv,
            "ppt" => Format::Ppt,
            "pptx" => Format::Pptx,
            "odp" => Format::Odp,
            "pdf" => Format::Pdf,
            "epub" => Format::Epub,
            _ => return None,
        })
    }

    pub fn ext(&self) -> &'static str {
        match self {
            Format::Doc => "doc",
            Format::Docx => "docx",
            Format::Odt => "odt",
            Format::Rtf => "rtf",
            Format::Txt => "txt",
            Format::Html => "html",
            Format::Md => "md",
            Format::Xls => "xls",
            Format::Xlsx => "xlsx",
            Format::Ods => "ods",
            Format::Csv => "csv",
            Format::Ppt => "ppt",
            Format::Pptx => "pptx",
            Format::Odp => "odp",
            Format::Pdf => "pdf",
            Format::Png => "png",
            Format::Jpg => "jpg",
            Format::Epub => "epub",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Format::Doc => "Word 97 (doc)",
            Format::Docx => "Word (docx)",
            Format::Odt => "OpenDocument 文本 (odt)",
            Format::Rtf => "富文本 (rtf)",
            Format::Txt => "纯文本 (txt)",
            Format::Html => "网页 (html)",
            Format::Md => "Markdown (md)",
            Format::Xls => "Excel 97 (xls)",
            Format::Xlsx => "Excel (xlsx)",
            Format::Ods => "OpenDocument 表格 (ods)",
            Format::Csv => "CSV 表格 (csv)",
            Format::Ppt => "PowerPoint 97 (ppt)",
            Format::Pptx => "PowerPoint (pptx)",
            Format::Odp => "OpenDocument 演示 (odp)",
            Format::Pdf => "PDF 文档",
            Format::Png => "PNG 图片",
            Format::Jpg => "JPEG 图片",
            Format::Epub => "EPUB 电子书",
        }
    }

    /// 可转换的目标格式（不含自身）
    pub fn targets(&self) -> Vec<Format> {
        use Format::*;
        match self {
            // 文字处理类
            Doc | Docx | Odt | Rtf | Txt | Html | Md | Epub => {
                vec![Pdf, Docx, Doc, Odt, Rtf, Txt, Html, Png, Jpg]
            }
            // 电子表格类
            Xls | Xlsx | Ods | Csv => vec![Pdf, Xlsx, Xls, Ods, Csv],
            // 演示文稿类
            Ppt | Pptx | Odp => vec![Pdf, Pptx, Ppt, Odp],
            // PDF：LibreOffice 可导入（有限支持），图片导出
            Pdf => vec![Docx, Png, Jpg],
            Png | Jpg => vec![],
        }
    }

    /// 内置引擎（轻量）可转换的目标格式：零依赖解析 Office 内部 XML，
    /// 仅提取文本/数据，不做版式渲染；其余格式需 LibreOffice 引擎
    pub fn light_targets(&self) -> Vec<Format> {
        use Format::*;
        match self {
            // Word 仅支持 docx（doc/odt/rtf 为其他封装，需 LibreOffice）
            Docx => vec![Txt, Html, Md],
            // 表格：xlsx 提取第一个工作表
            Xlsx => vec![Csv],
            // 演示：逐页提取文本
            Pptx => vec![Txt],
            // EPUB 电子书：提取正文
            Epub => vec![Txt, Html, Md],
            // 纯文本 / Markdown / HTML → PDF（零依赖版式渲染）
            Txt | Md | Html => vec![Pdf],
            _ => vec![],
        }
    }
}
