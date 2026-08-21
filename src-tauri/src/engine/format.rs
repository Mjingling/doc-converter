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
            "png" => Format::Png,
            "jpg" | "jpeg" => Format::Jpg,
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
            Format::Doc => "convert.formatDoc",
            Format::Docx => "convert.formatDocx",
            Format::Odt => "convert.formatOdt",
            Format::Rtf => "convert.formatRtf",
            Format::Txt => "convert.formatTxt",
            Format::Html => "convert.formatHtml",
            Format::Md => "convert.formatMd",
            Format::Xls => "convert.formatXls",
            Format::Xlsx => "convert.formatXlsx",
            Format::Ods => "convert.formatOds",
            Format::Csv => "convert.formatCsv",
            Format::Ppt => "convert.formatPpt",
            Format::Pptx => "convert.formatPptx",
            Format::Odp => "convert.formatOdp",
            Format::Pdf => "convert.formatPdf",
            Format::Png => "convert.formatPng",
            Format::Jpg => "convert.formatJpg",
            Format::Epub => "convert.formatEpub",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_ext_common() {
        assert_eq!(Format::from_ext("docx"), Some(Format::Docx));
        assert_eq!(Format::from_ext("pdf"), Some(Format::Pdf));
        assert_eq!(Format::from_ext("xlsx"), Some(Format::Xlsx));
        assert_eq!(Format::from_ext("pptx"), Some(Format::Pptx));
        assert_eq!(Format::from_ext("epub"), Some(Format::Epub));
    }

    #[test]
    fn test_from_ext_case_insensitive() {
        assert_eq!(Format::from_ext("PDF"), Some(Format::Pdf));
        assert_eq!(Format::from_ext("DOCX"), Some(Format::Docx));
        assert_eq!(Format::from_ext("Xlsx"), Some(Format::Xlsx));
    }

    #[test]
    fn test_from_ext_aliases() {
        assert_eq!(Format::from_ext("htm"), Some(Format::Html));
        assert_eq!(Format::from_ext("markdown"), Some(Format::Md));
    }

    #[test]
    fn test_from_ext_unknown() {
        assert_eq!(Format::from_ext("unknown"), None);
        assert_eq!(Format::from_ext(""), None);
    }

    #[test]
    fn test_ext() {
        assert_eq!(Format::Docx.ext(), "docx");
        assert_eq!(Format::Pdf.ext(), "pdf");
        assert_eq!(Format::Xlsx.ext(), "xlsx");
        assert_eq!(Format::Png.ext(), "png");
        assert_eq!(Format::Epub.ext(), "epub");
    }

    #[test]
    fn test_from_ext_roundtrip() {
        for f in [Format::Doc, Format::Docx, Format::Odt, Format::Rtf, Format::Txt,
                  Format::Html, Format::Md, Format::Xls, Format::Xlsx, Format::Ods,
                  Format::Csv, Format::Ppt, Format::Pptx, Format::Odp, Format::Pdf,
                  Format::Png, Format::Jpg, Format::Epub]
        {
            assert_eq!(Format::from_ext(f.ext()), Some(f), "roundtrip failed for {:?}", f);
        }
    }

    #[test]
    fn test_targets_word_processing() {
        for f in [Format::Doc, Format::Docx, Format::Odt, Format::Rtf, Format::Txt,
                  Format::Html, Format::Md, Format::Epub]
        {
            let t = f.targets();
            assert!(t.contains(&Format::Pdf), "{:?} 应支持转 PDF", f);
            assert!(t.contains(&Format::Png), "{:?} 应支持转 PNG", f);
        }
    }

    #[test]
    fn test_targets_spreadsheet() {
        for f in [Format::Xls, Format::Xlsx, Format::Ods, Format::Csv] {
            let t = f.targets();
            assert!(t.contains(&Format::Pdf), "{:?} 应支持转 PDF", f);
            assert!(t.contains(&Format::Csv), "{:?} 应支持转 CSV", f);
        }
    }

    #[test]
    fn test_targets_presentation() {
        for f in [Format::Ppt, Format::Pptx, Format::Odp] {
            let t = f.targets();
            assert!(t.contains(&Format::Pdf), "{:?} 应支持转 PDF", f);
        }
    }

    #[test]
    fn test_targets_pdf() {
        let t = Format::Pdf.targets();
        assert!(t.contains(&Format::Docx));
        assert!(t.contains(&Format::Png));
        assert!(t.contains(&Format::Jpg));
    }

    #[test]
    fn test_targets_image_empty() {
        assert!(Format::Png.targets().is_empty(), "PNG 无转换目标");
        assert!(Format::Jpg.targets().is_empty(), "JPG 无转换目标");
    }

    #[test]
    fn test_light_targets_docx() {
        let t = Format::Docx.light_targets();
        assert!(t.contains(&Format::Txt));
        assert!(t.contains(&Format::Html));
        assert!(t.contains(&Format::Md));
    }

    #[test]
    fn test_light_targets_xlsx() {
        assert_eq!(Format::Xlsx.light_targets(), vec![Format::Csv]);
    }

    #[test]
    fn test_light_targets_text_to_pdf() {
        assert_eq!(Format::Txt.light_targets(), vec![Format::Pdf]);
        assert_eq!(Format::Md.light_targets(), vec![Format::Pdf]);
        assert_eq!(Format::Html.light_targets(), vec![Format::Pdf]);
    }

    #[test]
    fn test_light_targets_old_format_empty() {
        // doc/odt/rtf 等旧格式需 LibreOffice，内置引擎不支持
        assert!(Format::Doc.light_targets().is_empty());
        assert!(Format::Odt.light_targets().is_empty());
        assert!(Format::Rtf.light_targets().is_empty());
        assert!(Format::Ppt.light_targets().is_empty());
        assert!(Format::Xls.light_targets().is_empty());
    }
}
