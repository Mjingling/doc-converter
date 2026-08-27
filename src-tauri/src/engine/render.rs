//! PDF 逐页渲染（内置引擎）：基于 Pdfium 动态库，免 LibreOffice。
//! 动态库由 scripts/download_pdfium.sh 下载、随安装包资源目录分发，启动时按候选路径加载。

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use pdfium_render::prelude::*;

/// Tauri 资源目录（setup 阶段注入，用于定位打包进安装包的 Pdfium 动态库）
static RESOURCE_DIR: OnceLock<PathBuf> = OnceLock::new();
/// Pdfium 实例（首次渲染时初始化，后续复用）
static PDFIUM: OnceLock<Result<Pdfium, String>> = OnceLock::new();

/// 启动时记录资源目录（lib.rs setup 调用）
pub fn set_resource_dir(dir: PathBuf) {
    let _ = RESOURCE_DIR.set(dir);
}

fn lib_file_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "libpdfium.dylib"
    } else if cfg!(target_os = "windows") {
        "pdfium.dll"
    } else {
        "libpdfium.so"
    }
}

/**
 * Pdfium 动态库候选路径（按序尝试）：
 * 1. 打包资源目录（resource_dir，含子目录变体）
 * 2. 可执行文件所在目录
 * 3. 开发态工作目录（项目根 / src-tauri 下的 resources/pdfium）
 * 4. 系统默认搜索路径（裸文件名）
 */
fn lib_candidates() -> Vec<String> {
    let name = lib_file_name();
    let mut c: Vec<String> = Vec::new();
    if let Some(res) = RESOURCE_DIR.get() {
        for sub in ["", "pdfium", "resources/pdfium"] {
            let p = if sub.is_empty() { res.join(name) } else { res.join(sub).join(name) };
            c.push(p.to_string_lossy().to_string());
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            c.push(dir.join(name).to_string_lossy().to_string());
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        c.push(cwd.join("src-tauri").join("resources").join("pdfium").join(name).to_string_lossy().to_string());
        c.push(cwd.join("resources").join("pdfium").join(name).to_string_lossy().to_string());
    }
    c.push(name.to_string());
    c
}

fn init_pdfium() -> Result<Pdfium, String> {
    let mut last_err = String::from("无候选路径");
    for path in lib_candidates() {
        match Pdfium::bind_to_library(&path) {
            Ok(lib) => return Ok(Pdfium::new(lib)),
            Err(e) => last_err = format!("{}: {}", path, e),
        }
    }
    Err(format!(
        "Pdfium 动态库加载失败（可运行 scripts/download_pdfium.sh 下载）：{}",
        last_err
    ))
}

fn pdfium() -> Result<&'static Pdfium, String> {
    PDFIUM.get_or_init(init_pdfium).as_ref().map_err(|e| e.clone())
}

/// 渲染参数校验（不依赖 Pdfium，可独立单测）
pub fn validate_render_params(format: &str, dpi: u16) -> Result<(), String> {
    if format != "png" && format != "jpg" {
        return Err(format!("仅支持 png / jpg 输出格式: {}", format));
    }
    if !(10..=1200).contains(&dpi) {
        return Err(format!("DPI 需在 10~1200 之间: {}", dpi));
    }
    Ok(())
}

/**
 * 将 PDF 逐页渲染为图片。
 * - pages：None 或空 = 全部页；否则为 1 基页码列表（越界报错）
 * - format：png / jpg；dpi：渲染分辨率（72 = 原始点密度）
 * - 输出到 out_dir，文件名 page_001.png …（保持页序）
 */
pub fn pdf_render_pages(
    input: &Path,
    out_dir: &Path,
    pages: Option<Vec<u32>>,
    format: &str,
    dpi: u16,
) -> Result<Vec<String>, String> {
    validate_render_params(format, dpi)?;
    let pdfium = pdfium()?;
    let path_str = input.to_str().ok_or_else(|| "无效路径".to_string())?;
    let doc = pdfium
        .load_pdf_from_file(path_str, None)
        .map_err(|e| format!("加载 PDF 失败: {}", e))?;
    let total = doc.pages().len() as u32;
    if total == 0 {
        return Err("PDF 无页面".to_string());
    }
    let idxs: Vec<u32> = match pages {
        Some(v) if !v.is_empty() => {
            for &p in &v {
                if p < 1 || p > total {
                    return Err(format!("页码越界: {}（共 {} 页）", p, total));
                }
            }
            v
        }
        _ => (1..=total).collect(),
    };
    std::fs::create_dir_all(out_dir).map_err(|e| format!("创建输出目录失败: {}", e))?;
    let scale = dpi as f32 / 72.0;
    let mut outs = Vec::with_capacity(idxs.len());
    for p in idxs {
        let page = doc
            .pages()
            .get((p - 1) as u16)
            .map_err(|e| format!("读取第 {} 页失败: {}", p, e))?;
        // 按 DPI 换算目标像素尺寸（PDF 点密度为 72pt/inch）
        let width_px = (page.width().value * scale).round() as i32;
        let height_px = (page.height().value * scale).round() as i32;
        let config = PdfRenderConfig::new()
            .set_target_width(width_px)
            .set_target_height(height_px);
        let bitmap = page
            .render_with_config(&config)
            .map_err(|e| format!("渲染第 {} 页失败: {}", p, e))?;
        let img = bitmap.as_image();
        let out = out_dir.join(format!("page_{:03}.{}", p, format));
        let save_result = if format == "jpg" {
            // jpg 不支持透明通道
            img.into_rgb8().save(&out)
        } else {
            img.save(&out)
        };
        save_result.map_err(|e| format!("保存图片失败: {}", e))?;
        outs.push(out.to_string_lossy().to_string());
    }
    Ok(outs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_render_params() {
        assert!(validate_render_params("png", 72).is_ok());
        assert!(validate_render_params("jpg", 300).is_ok());
        assert!(validate_render_params("webp", 150).is_err());
        assert!(validate_render_params("png", 0).is_err());
        assert!(validate_render_params("png", 2000).is_err());
    }

    /// 冒烟测试需要本地 Pdfium 动态库（CI 环境没有），仅本地手动运行：
    /// cargo test test_render_smoke -- --ignored
    #[test]
    #[ignore]
    fn test_render_smoke() {
        let dir = std::env::temp_dir().join(format!("render_smoke_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // 构造单页空白 PDF
        let mut doc = lopdf::Document::with_version("1.5");
        let mut page_dict = lopdf::Dictionary::new();
        page_dict.set("Type", "Page");
        page_dict.set("MediaBox", vec![
            lopdf::Object::Integer(0),
            lopdf::Object::Integer(0),
            lopdf::Object::Integer(200),
            lopdf::Object::Integer(100),
        ]);
        page_dict.set("Resources", lopdf::Dictionary::new());
        let page_id = doc.add_object(page_dict);
        let mut pages_dict = lopdf::Dictionary::new();
        pages_dict.set("Type", "Pages");
        pages_dict.set("Kids", vec![lopdf::Object::Reference(page_id)]);
        pages_dict.set("Count", 1i64);
        let pages_id = doc.add_object(pages_dict);
        doc.objects.get_mut(&page_id).unwrap().as_dict_mut().unwrap()
            .set("Parent", lopdf::Object::Reference(pages_id));
        let mut catalog = lopdf::Dictionary::new();
        catalog.set("Type", "Catalog");
        catalog.set("Pages", lopdf::Object::Reference(pages_id));
        let catalog_id = doc.add_object(catalog);
        doc.trailer.set("Root", catalog_id);
        let pdf_path = dir.join("blank.pdf");
        doc.save(&pdf_path).unwrap();

        let out_dir = dir.join("imgs");
        let outs = pdf_render_pages(&pdf_path, &out_dir, None, "png", 72).unwrap();
        assert_eq!(outs.len(), 1);
        assert!(Path::new(&outs[0]).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
