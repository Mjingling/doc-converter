//! 网页转 PDF（WebView 渲染引擎，方案 2）
//!
//! 通过隐藏 WebView 窗口加载页面，等待渲染完成后按真实版面打印为 PDF：
//! - macOS：WKWebView printOperation（静默保存到文件）
//! - Windows：WebView2 PrintToPdf
//!
//! 与 commands::web（轻量文本提取引擎）互补：本引擎所见即所得，
//! 支持 JS 动态渲染 / CSS 布局 / 图片，适合现代 SPA 网页。

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tauri::{AppHandle, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

/// readyState 轮询间隔
const POLL_INTERVAL_MS: u64 = 300;
/// 页面加载最长等待
const LOAD_TIMEOUT_MS: u64 = 45_000;
/// readyState=complete 后再等 JS/图片渲染的时间
const RENDER_SETTLE_MS: u64 = 1_200;
/// 打印输出最长等待
const PRINT_TIMEOUT_MS: u64 = 30_000;
/// 隐藏窗口视口（宽 1280 触发常见桌面端响应式布局）
const VIEWPORT_W: f64 = 1280.0;
const VIEWPORT_H: f64 = 900.0;

static LABEL_SEQ: AtomicU64 = AtomicU64::new(0);

/// 将网页按真实渲染版面转换为 PDF
#[tauri::command]
pub async fn webpage_to_pdf_rendered(
    app: AppHandle,
    url: String,
    out_path: String,
) -> Result<String, String> {
    let ret_path = out_path.clone();
    tauri::async_runtime::spawn_blocking(move || render_and_print(&app, &url, &out_path))
        .await
        .map_err(|e| format!("网页渲染转 PDF 内部错误: {}", e))?
        .map(|_| ret_path)
}

fn render_and_print(app: &AppHandle, url: &str, out_path: &str) -> Result<(), String> {
    // 仅接受 http/https，杜绝 file:// 等本地页面绕过同源预期
    let parsed = tauri::Url::parse(url).map_err(|e| format!("URL 非法: {}", e))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err("仅支持 http/ https:// 开头的网页地址".into());
    }

    // 移除旧文件，避免「文件已存在」被误判为打印完成
    let _ = std::fs::remove_file(out_path);
    if let Some(parent) = std::path::Path::new(out_path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建输出目录失败: {}", e))?;
        }
    }

    let label = format!("web-print-{}", LABEL_SEQ.fetch_add(1, Ordering::Relaxed));
    let window = WebviewWindowBuilder::new(
        app,
        &label,
        WebviewUrl::External(parsed),
    )
    .title("DocMorph WebPrint")
    .inner_size(VIEWPORT_W, VIEWPORT_H)
    .resizable(false)
    .decorations(false)
    .visible(false)
    .build()
    .map_err(|e| format!("创建渲染窗口失败: {}", e))?;

    let result = (|| {
        wait_page_loaded(&window)?;
        std::thread::sleep(Duration::from_millis(RENDER_SETTLE_MS));
        print_webview(&window, out_path)?;
        wait_output_file(out_path)?;
        Ok(())
    })();

    // 打印窗口不参与 CloseRequested（hide-to-tray）逻辑，必须 destroy 彻底销毁
    let _ = window.destroy();
    result
}

/// 轮询 document.readyState 直到 complete（eval_with_callback 取回执行结果）
fn wait_page_loaded(window: &WebviewWindow) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_millis(LOAD_TIMEOUT_MS);
    loop {
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        // 页面跳转（如 https→http 重定向）过程中 eval 可能失败，失败仅当本轮未就绪处理
        if window
            .eval_with_callback("document.readyState", move |s| {
                let _ = tx.send(s);
            })
            .is_ok()
        {
            if let Ok(state) = rx.recv_timeout(Duration::from_millis(1_000)) {
                if state.contains("complete") {
                    return Ok(());
                }
            }
        }
        if Instant::now() > deadline {
            return Err("页面加载超时（45s），请检查网址是否可访问".into());
        }
        std::thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
    }
}

/// 等待打印产物落盘（大小连续两次采样一致视为写完）
fn wait_output_file(out_path: &str) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_millis(PRINT_TIMEOUT_MS);
    let mut last_len: u64 = 0;
    loop {
        std::thread::sleep(Duration::from_millis(600));
        match std::fs::metadata(out_path) {
            Ok(m) if m.len() > 0 => {
                if m.len() == last_len {
                    return Ok(());
                }
                last_len = m.len();
            }
            _ => last_len = 0,
        }
        if Instant::now() > deadline {
            return Err("生成 PDF 超时（30s）".into());
        }
    }
}

/* ---------- 平台打印实现 ---------- */

#[cfg(target_os = "macos")]
fn print_webview(window: &WebviewWindow, out_path: &str) -> Result<(), String> {
    use objc2::msg_send;
    use objc2::ClassType;
    use objc2_app_kit::NSPrintInfo;
    use objc2_web_kit::WKWebView;

    let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();
    let path = out_path.to_string();
    window
        .with_webview(move |wv| {
            // 回调运行在主线程，打印操作必须在主线程发起
            let r: Result<(), String> = (|| {
                let wk: &WKWebView = unsafe { &*(wv.inner() as *const WKWebView) };
                unsafe {
                    // 克隆共享 printInfo 配置，避免污染全局默认值
                    let shared: objc2::rc::Retained<NSPrintInfo> =
                        msg_send![NSPrintInfo::class(), sharedPrintInfo];
                    let info: objc2::rc::Retained<NSPrintInfo> = msg_send![&shared, copy];

                    // A4 @72dpi、零边距；横向 Fit（内容适配纸宽缩放）+ 纵向自动分页
                    let _: () = msg_send![&info, setPaperSize: objc2_foundation::NSSize::new(595.28, 841.89)];
                    let _: () = msg_send![&info, setLeftMargin: 0.0f64];
                    let _: () = msg_send![&info, setRightMargin: 0.0f64];
                    let _: () = msg_send![&info, setTopMargin: 0.0f64];
                    let _: () = msg_send![&info, setBottomMargin: 0.0f64];
                    // NSPrintingPaginationAutomatic = 0, Fit = 1
                    let _: () = msg_send![&info, setHorizontalPagination: 1i64];
                    let _: () = msg_send![&info, setVerticalPagination: 0i64];

                    // 保存到文件而非送打印机（msg_send 对象参数传 &* 解引用）
                    let save_job = objc2_foundation::NSString::from_str("NSPrintSaveJob");
                    let _: () = msg_send![&info, setJobDisposition: &*save_job];
                    let dict = info.dictionary();
                    let url = objc2_foundation::NSURL::fileURLWithPath(
                        &objc2_foundation::NSString::from_str(&path),
                    );
                    let save_url_key = objc2_foundation::NSString::from_str("NSPrintJobSavingURL");
                    let _: () = msg_send![&dict, setObject: &*url, forKey: &*save_url_key];

                    // 静默打印：不弹打印面板与进度面板
                    let op = wk.printOperationWithPrintInfo(&info);
                    let no = objc2::runtime::Bool::new(false);
                    let _: () = msg_send![&op, setShowsPrintPanel: no];
                    let _: () = msg_send![&op, setShowsProgressPanel: no];
                    // run() 将操作提交到 runloop，由主线程异步写出文件
                    let _: () = msg_send![&op, run];
                    // 等待打印操作开始执行：run() 是异步提交到 runloop 的，
                    // 如果立即返回并被调用方销毁窗口，打印会因访问已释放的 WebView 而崩溃
                    std::thread::sleep(Duration::from_millis(800));
                }
                Ok(())
            })();
            let _ = tx.send(r);
        })
        .map_err(|e| format!("访问 WebView 失败: {}", e))?;

    rx.recv_timeout(Duration::from_secs(5))
        .map_err(|_| "打印启动超时".to_string())?
}

#[cfg(windows)]
fn print_webview(window: &WebviewWindow, out_path: &str) -> Result<(), String> {
    use webview2_com::Microsoft::Web::WebView2::Win32::*;
    use webview2_com::PrintToPdfCompletedHandler;
    // Interface trait 提供 cast()（windows 0.61 绑定下 ICoreWebView2* 接口转换必需）
    use windows::core::Interface;

    let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();
    let path = out_path.to_string();
    window
        .with_webview(move |wv| {
            let r: Result<(), String> = (|| unsafe {
                let controller = wv.controller();
                let core = controller
                    .CoreWebView2()
                    .map_err(|e| format!("获取 WebView2 实例失败: {}", e))?;
                let core16: ICoreWebView2_16 = core.cast().map_err(|_| {
                    // to_string 显式类型：into() 在 map_err 闭包内会因推断歧义报 E0283
                    "当前 WebView2 运行时过低，不支持编程式打印 PDF，请更新 Edge/WebView2".to_string()
                })?;

                // 打印设置：A4 纵向、零边距、保留背景色、无页眉页脚（尺寸单位英寸，与 macOS A4 对齐）
                let env = wv.environment();
                let env9: ICoreWebView2Environment9 = env.cast().map_err(|_| {
                    "当前 WebView2 运行时过低，不支持创建打印设置".to_string()
                })?;
                let settings = env9
                    .CreatePrintSettings()
                    .map_err(|e| format!("创建打印设置失败: {}", e))?;
                let _ = settings.SetOrientation(COREWEBVIEW2_PRINT_ORIENTATION_PORTRAIT);
                let _ = settings.SetShouldPrintBackgrounds(true);
                let _ = settings.SetShouldPrintHeaderAndFooter(false);
                let _ = settings.SetScaleFactor(1.0);
                let _ = settings.SetPageWidth(8.27);
                let _ = settings.SetPageHeight(11.69);
                let _ = settings.SetMarginTop(0.0);
                let _ = settings.SetMarginBottom(0.0);
                let _ = settings.SetMarginLeft(0.0);
                let _ = settings.SetMarginRight(0.0);

                // 回调闭包参数已被宏转换：HRESULT → Result<()>，BOOL → bool
                let handler = PrintToPdfCompletedHandler::create(Box::new(
                    move |result, is_successful| {
                        let outcome = if let Err(e) = result {
                            Err(format!("打印失败: {}", e))
                        } else if !is_successful {
                            Err("打印失败：PDF 生成未成功".to_string())
                        } else {
                            Ok(())
                        };
                        // 接收端超时放弃时忽略发送错误
                        let _ = tx.send(outcome);
                        Ok(())
                    },
                ));
                core16
                    .PrintToPdf(&windows::core::HSTRING::from(path.as_str()), &settings, &handler)
                    .map_err(|e| format!("启动打印失败: {}", e))?;
                Ok(())
            })();
            if let Err(e) = r {
                let _ = tx.send(Err(e));
            }
        })
        .map_err(|e| format!("访问 WebView 失败: {}", e))?;

    rx.recv_timeout(Duration::from_secs(PRINT_TIMEOUT_MS))
        .map_err(|_| "打印超时".to_string())?
}

#[cfg(not(any(target_os = "macos", windows)))]
fn print_webview(_window: &WebviewWindow, _out_path: &str) -> Result<(), String> {
    Err("当前平台暂不支持网页渲染转 PDF（仅支持 macOS / Windows）".into())
}
