//! LibreOffice headless 引擎：检测与调用
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct LibreOfficeEngine {
    pub binary: Option<PathBuf>,
}

impl LibreOfficeEngine {
    pub fn detect() -> Self {
        #[cfg(target_os = "macos")]
        let candidates: Vec<PathBuf> = {
            let home = std::env::var("HOME").unwrap_or_default();
            vec![
                PathBuf::from("/Applications/LibreOffice.app/Contents/MacOS/soffice"),
                PathBuf::from("/Applications/LibreOffice.app/Contents/MacOS/soffice.bin"),
                PathBuf::from(format!(
                    "{}/Applications/LibreOffice.app/Contents/MacOS/soffice",
                    home
                )),
            ]
        };

        #[cfg(target_os = "windows")]
        let candidates: Vec<PathBuf> = vec![
            PathBuf::from(r"C:\Program Files\LibreOffice\program\soffice.exe"),
            PathBuf::from(r"C:\Program Files (x86)\LibreOffice\program\soffice.exe"),
        ];

        #[cfg(target_os = "linux")]
        let candidates: Vec<PathBuf> = {
            let mut v = vec![
                PathBuf::from("/usr/bin/soffice"),
                PathBuf::from("/usr/local/bin/soffice"),
            ];
            // 尝试 which soffice
            if let Ok(out) = Command::new("which").arg("soffice").output() {
                if out.status.success() {
                    let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if !p.is_empty() {
                        v.push(PathBuf::from(p));
                    }
                }
            }
            v
        };

        for c in candidates {
            if c.exists() {
                return Self { binary: Some(c) };
            }
        }
        Self { binary: None }
    }

    pub fn available(&self) -> bool {
        self.binary.is_some()
    }

    /// 转换文档：soffice --headless --convert-to <fmt> --outdir <dir> <input>
    /// 返回输出文件路径
    pub fn convert(&self, input: &Path, fmt: &str, out_dir: &Path) -> Result<PathBuf, String> {
        let bin = self.binary.as_ref().ok_or("LibreOffice 未安装")?;
        let out_ext = fmt.split(':').next().unwrap_or(fmt);
        let mut cmd = Command::new(bin);
        cmd.args(["--headless", "--convert-to", fmt, "--outdir"])
            .arg(out_dir);
        // 踩坑：PDF 转文字类格式（docx/doc/odt/rtf/txt/html）时，LibreOffice 默认按 Draw
        // 打开 PDF，导出时提示 no export filter 导致静默失败；必须加 --infilter 强制以
        // Writer 导入 PDF，才能导出文字类格式（图片类 png/jpg 不受影响）
        let is_pdf = input
            .extension()
            .map(|e| e.to_str().unwrap_or("").eq_ignore_ascii_case("pdf"))
            .unwrap_or(false);
        let is_text_target = matches!(out_ext, "docx" | "doc" | "odt" | "rtf" | "txt" | "html");
        if is_pdf && is_text_target {
            cmd.arg("--infilter=writer_pdf_import");
        }
        cmd.arg(input);
        let out = cmd
            .output()
            .map_err(|e| format!("启动 LibreOffice 失败: {}", e))?;

        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            return Err(format!("LibreOffice 转换失败: {}", err.trim()));
        }

        // 输出文件名 = 输入文件名（换扩展名）
        let stem = input
            .file_stem()
            .ok_or_else(|| "无效的文件名".to_string())?
            .to_string_lossy()
            .to_string();
        let out_file = out_dir.join(format!("{}.{}", stem, out_ext));
        if !out_file.exists() {
            return Err("LibreOffice 返回成功但未找到输出文件".into());
        }
        Ok(out_file)
    }
}
