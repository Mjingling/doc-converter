//! 网页转 PDF（WebView 渲染引擎，方案 2）
//!
//! 通过**常驻隐藏打印窗口**加载页面，等待渲染完成后按真实版面打印为 PDF：
//! - macOS：WKWebView printOperation（静默保存到文件）
//! - Windows：WebView2 PrintToPdf
//!
//! 与 commands::web（轻量文本提取引擎）互补：本引擎所见即所得，
//! 支持 JS 动态渲染 / CSS 布局 / 图片，适合现代 SPA 网页。
//!
//! 窗口生命周期（重要教训）：
//! 每次转换后销毁窗口曾导致间歇性崩溃——eval / with_webview 消息是
//! fire-and-forget 派发到主循环的，destroy 之后队列中的残留消息会访问
//! 已释放的 WKWebView（ObjC 异常无法被 Rust 捕获 → abort）。因此打印
//! 窗口**只创建一次、跨任务复用、永不销毁**（应用退出时随进程清理），
//! 从根上消灭销毁竞态。

use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

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

/// 常驻打印窗口 label（跨任务复用）
const PRINT_LABEL: &str = "web-print";

/// 打印任务互斥锁：共享打印窗口，同一时刻只允许一个转换（导航/打印互相覆盖）
static PRINT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// 取常驻打印窗口：不存在则创建（about:blank 起步，由调用方导航）。
/// 创建经事件循环代理派发到主线程执行，任意线程调用安全。
///
/// 窗口定位在屏幕外（负坐标）但保持可见状态：createPDF 只能捕获
/// 实际渲染合成的内容，visible(false) 的窗口 WKWebView 不做合成
/// （曾输出 3.7KB 空白 PDF）；移出所有显示器范围即可让系统正常渲染
/// 而用户完全看不到。
fn ensure_print_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    if let Some(win) = app.get_webview_window(PRINT_LABEL) {
        return Ok(win);
    }
    WebviewWindowBuilder::new(
        app,
        PRINT_LABEL,
        WebviewUrl::External("about:blank".parse().unwrap()),
    )
    .title("DocMorph WebPrint")
    .inner_size(VIEWPORT_W, VIEWPORT_H)
    .position(-2000.0, -2000.0)
    .resizable(false)
    .decorations(false)
    .build()
    .map_err(|e| format!("创建打印窗口失败: {e}"))
}

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
            std::fs::create_dir_all(parent).map_err(|e| format!("创建输出目录失败: {e}"))?;
        }
    }

    // 常驻窗口：取或建 → 导航到目标页（复用时不销毁重建）。
    // 全程持锁：共享窗口不支持并发转换（导航会互相覆盖）
    let _guard = PRINT_LOCK.lock().expect("print lock poisoned");
    let window = ensure_print_window(app)?;
    let prev_url = window.url().map(|u| u.to_string()).unwrap_or_default();
    window
        .navigate(parsed.clone())
        .map_err(|e| format!("页面导航失败: {e}"))?;

    // 导航生效 + 加载完成合并等待（容忍重定向与同 URL 重载）：
    // 仅当「URL 已离开上一页」或「readyState 出现过非 complete（重载回落）」
    // 之后的 complete 才算新页面加载完成，避免旧页面的 complete 被误判
    wait_loaded(&window, &prev_url)?;
    std::thread::sleep(Duration::from_millis(RENDER_SETTLE_MS));
    print_webview(&window, out_path)?;
    wait_output_file(out_path)?;
    Ok(())
}

/// 等待新页面加载完成：
/// - 跳转/重定向：URL 离开 prev（新 URL 可以是重定向后的任意地址，不要求等于目标）
/// - 同 URL 重载：URL 不变，但 readyState 会先跌回 loading/interactive 再回 complete
/// 两种信号任一出现后的 readyState=complete 即视为完成
fn wait_loaded(window: &WebviewWindow, prev: &str) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_millis(LOAD_TIMEOUT_MS);
    let mut nav_confirmed = prev.is_empty(); // 初次 about:blank 起步视作已离开
    loop {
        if let Ok(current) = window.url() {
            if current.as_str() != prev {
                nav_confirmed = true;
            }
        }
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        // 页面跳转（如 https→http 重定向）过程中 eval 可能失败，失败仅当本轮未就绪处理
        if window
            .eval_with_callback(
                "(document.readyState !== 'complete') ? 'loading' : document.readyState",
                move |s| {
                    let _ = tx.send(s);
                },
            )
            .is_ok()
        {
            if let Ok(state) = rx.recv_timeout(Duration::from_millis(1_000)) {
                if state.contains("loading") {
                    nav_confirmed = true; // readyState 跌落 → 新文档正在加载（同 URL 重载路径）
                } else if nav_confirmed && state.contains("complete") {
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
    use objc2_web_kit::WKWebView;

    let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();
    let path = out_path.to_string();
    window
        .with_webview(move |wv| {
            // 回调运行在主线程，PDF 导出必须在主线程发起
            let r: Result<(), String> = (|| unsafe {
                let wk: &WKWebView = &*(wv.inner() as *const WKWebView);
                // macOS 13+ 现代 PDF 导出 API（Safari 同源实现），替代不稳定的
                // NSPrintOperation 打印路径——打印分页回调中的 ObjC 异常无法被
                // Rust 捕获，曾三次导致整个应用 abort。
                // nil 配置 = 捕获整页内容（输出为单页连续长图式 PDF，阅读器可滚动）。
                // 文件由完成回调直接写出（无需 NSPrintSaveJob 与打印面板）。
                let tx_cb = tx.clone();
                let handler = block2::RcBlock::new(
                    move |data: *mut objc2_foundation::NSData,
                          err: *mut objc2_foundation::NSError| {
                        let outcome = if !err.is_null() {
                            Err(format!("PDF 生成失败: {:?}", *err))
                        } else if data.is_null() {
                            Err("PDF 生成失败：未返回数据".into())
                        } else {
                            std::fs::write(&path, (*data).to_vec())
                                .map_err(|e| format!("写入 PDF 失败: {e}"))
                        };
                        let _ = tx_cb.send(outcome);
                    },
                );
                wk.createPDFWithConfiguration_completionHandler(None, &handler);
                Ok(())
            })();
            if let Err(e) = r {
                let _ = tx.send(Err(e));
            }
        })
        .map_err(|e| format!("访问 WebView 失败: {e}"))?;

    // createPDF 为异步完成回调，此处等待回调送达（文件已由回调写完）
    rx.recv_timeout(Duration::from_secs(PRINT_TIMEOUT_MS))
        .map_err(|_| "生成 PDF 超时（30s）".to_string())?
}

#[cfg(windows)]
fn print_webview(window: &WebviewWindow, out_path: &str) -> Result<(), String> {
    use webview2_com::Microsoft::Web::WebView2::Win32::*;
    use webview2_com::PrintToPdfCompletedHandler;
    // Interface trait 提供 cast()（webview2-com 0.38 绑定：方法名为 CoreWebView2 而非 GetCoreWebView2）
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
                // Sender 非 Copy：clone 一份给异步回调，外层保留原始 tx 发初始化错误
                let tx_cb = tx.clone();
                let handler = PrintToPdfCompletedHandler::create(Box::new(
                    move |result, is_successful| {
                        let outcome = if let Err(e) = result {
                            Err(format!("打印失败: {}", e))
                        } else if !is_successful {
                            Err("打印失败：PDF 生成未成功".into())
                        } else {
                            Ok(())
                        };
                        // 接收端超时放弃时忽略发送错误
                        let _ = tx_cb.send(outcome);
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
