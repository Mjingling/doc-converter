//! 桌面宠物窗口：透明置顶小窗口常驻桌面右下角，渲染机器人形象
//!
//! 窗口特性：无边框 + 透明 + 始终置顶 + 不进任务栏 + 无阴影 + 关闭拖放；
//! 内容复用前端入口（`index.html?window=pet`，App.vue 按 query 分流渲染 PetWindow）。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

/// diag 写入锁：worker 创建线程 / on_page_load 回调 / 延时复核线程 / 主线程并发调用，
/// 轮转（删文件）与追加必须原子，否则竞态会导致日志超限增长或行序交错
static DIAG_LOCK: Mutex<()> = Mutex::new(());

/// 宠物窗口诊断日志：写入应用数据目录，Windows 排障时一次运行即可定位问题环节。
/// 记录点：创建参数（位置/缩放）→ 页面加载 → show 结果 → 延时后的实际可见性/位置。
/// 超过 1MB 自动重置，避免长期运行无限增长。
fn diag(app: &AppHandle, msg: &str) {
    let line = format!("[{}] {msg}\n", fmt_utc_now());
    eprint!("{line}");
    let _guard = DIAG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if let Ok(dir) = app.path().app_data_dir() {
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("pet-diag.log");
        if std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0) > 1_000_000 {
            let _ = std::fs::remove_file(&path);
        }
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()));
    }
}

/// 人类可读的 UTC 时间戳（不引入 chrono，手算公历日期）
fn fmt_utc_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    fmt_utc(secs)
}

/// 秒数→公历日期串（单独拆出便于单测日期算法）
fn fmt_utc(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    // 公历反推（Howard Hinnant 的 days→civil 算法）
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
        if m <= 2 { y + 1 } else { y },
        m,
        d,
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// 宠物窗口 label（capabilities 权限按此放行）
pub const PET_LABEL: &str = "pet";
/// 宠物窗口尺寸（逻辑像素）
const PET_W: f64 = 150.0;
const PET_H: f64 = 180.0;
/// 距屏幕工作区右下角的边距（逻辑像素）
const PET_MARGIN: f64 = 24.0;

/// 计算工作区右下角对应的窗口逻辑坐标（纯函数，便于单测）
///
/// 入参均为物理像素：工作区 (x, y, w, h)、显示器缩放 scale、宠物窗口缩放 pet_scale（1.0 = 150×180）。
fn bottom_right_position(area_x: f64, area_y: f64, area_w: f64, area_h: f64, scale: f64, pet_scale: f64) -> (f64, f64) {
    let scale = if scale <= 0.0 { 1.0 } else { scale };
    let pet_scale = if pet_scale <= 0.0 { 1.0 } else { pet_scale };
    let px = area_x + area_w - PET_W * pet_scale * scale - PET_MARGIN * scale;
    let py = area_y + area_h - PET_H * pet_scale * scale - PET_MARGIN * scale;
    (px / scale, py / scale)
}

/// 桌宠创建进行中标记：创建卡住时窗口未注册到管理器，重复调用会再建窗口，需拦截
static PET_CREATING: AtomicBool = AtomicBool::new(false);

/// 桌宠期望可见标记：延迟创建期间用户关闭开关时置 false，创建线程醒来后取消创建
static PET_WANTED: AtomicBool = AtomicBool::new(false);

/// 创建标记 RAII 复位守卫：创建线程 panic 时也复位，避免桌宠本会话永久失效
struct CreatingGuard;
impl Drop for CreatingGuard {
    fn drop(&mut self) {
        PET_CREATING.store(false, Ordering::SeqCst);
    }
}

/// 创建前等待：让 WebView2 环境完成初始化（主窗口刚建好时立刻建第二个控制器容易卡死）
const PET_CREATE_DELAY_MS: u64 = 1500;

/// 显示桌面宠物（已存在则直接 show；否则按 scale 创建并定位到主显示器右下角）
#[tauri::command]
pub fn pet_show(app: AppHandle, scale: f64) -> Result<(), String> {
    PET_WANTED.store(true, Ordering::SeqCst);
    if let Some(win) = app.get_webview_window(PET_LABEL) {
        let shown = win.show();
        diag(&app, &format!("reuse: existing window, show={shown:?} visible={:?}", win.is_visible()));
        return Ok(());
    }
    if PET_CREATING.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    // 必须在独立线程创建：WebView2 第二个控制器的创建回调不来时 wry wait_with_pump 死等，
    // 若占用主线程，应用其余全部 IPC（打开日志/开发者工具等）会排队永久挂起。
    // 与 web_render 打印窗口同模式（阻塞线程创建），最坏情况桌宠不出现，应用不受影响。
    std::thread::spawn(move || {
        let _guard = CreatingGuard;
        std::thread::sleep(std::time::Duration::from_millis(PET_CREATE_DELAY_MS));
        // 延迟等待期间用户关闭了开关：取消本次创建，避免开关已关桌宠却弹出
        let result = if PET_WANTED.load(Ordering::SeqCst) {
            create_pet_blocking(&app, scale)
        } else {
            diag(&app, "cancelled: pet disabled during create delay");
            Ok(())
        };
        if let Err(e) = &result {
            eprintln!("[pet] create failed: {e}");
        }
    });
    Ok(())
}

/// 宠物窗口创建（在独立线程上执行，见 pet_show）
fn create_pet_blocking(app: &AppHandle, scale: f64) -> Result<(), String> {
    diag(app, "creating (worker thread)...");
    let (x, y) = match app.primary_monitor() {
        Ok(Some(m)) => {
            let wa = m.work_area();
            bottom_right_position(
                wa.position.x as f64,
                wa.position.y as f64,
                wa.size.width as f64,
                wa.size.height as f64,
                m.scale_factor(),
                scale,
            )
        }
        _ => bottom_right_position(0.0, 0.0, 1440.0, 900.0, 1.0, scale),
    };
    WebviewWindowBuilder::new(
        app,
        PET_LABEL,
        WebviewUrl::App("index.html?window=pet".into()),
    )
    .title("DocMorph Pet")
    .inner_size(PET_W * scale, PET_H * scale)
    .position(x, y)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .shadow(false)
    .resizable(false)
    // Windows 下透明+无边框窗口在页面加载完成前 show，DWM 可能不合成内容（窗口存在但不可见）；
    // 改为隐藏创建，等 on_page_load（WebView2 内容就绪）再 show（macOS 同样适用）
    .visible(false)
    .on_page_load(|win, _| {
        // 延迟创建期间开关被关闭：窗口刚建成直接销毁，不显示
        if !PET_WANTED.load(Ordering::SeqCst) {
            let app = win.app_handle().clone();
            diag(&app, "page_load: cancelled by pet_hide, destroying");
            let _ = win.destroy();
            return;
        }
        // 内容已渲染：此时显示能稳定触发合成；幂等，重复加载再 show 无副作用
        let shown = win.show();
        #[cfg(target_os = "windows")]
        {
            // Windows 透明层兼容招：±1 物理像素尺寸扰动强制重新合成，再恢复并激活窗口；
            // 个别 WebView2 版本首次渲染透明窗口内容丢失，抖动一下即可找回（无害）
            if let Ok(size) = win.outer_size() {
                let nudged = tauri::Size::Physical(tauri::PhysicalSize {
                    width: size.width.saturating_add(1),
                    height: size.height.saturating_add(1),
                });
                let _ = win.set_size(nudged);
                let _ = win.set_size(tauri::Size::Physical(size));
            }
            let _ = win.set_focus();
        }
        let app = win.app_handle().clone();
        diag(&app, &format!("page_load: show={shown:?} visible={:?}", win.is_visible()));
        // 800ms 后复核：若仍不可见/位置越界，日志里直接能看出是哪一环失败（DWM 合成 / 定位）
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(800));
            if let Some(w) = app.get_webview_window(PET_LABEL) {
                let pos = w.outer_position();
                let size = w.outer_size();
                diag(&app, &format!(
                    "verify: visible={:?} pos=({:?},{:?}) size=({:?},{:?})",
                    w.is_visible(),
                    pos.as_ref().map(|p| p.x),
                    pos.as_ref().map(|p| p.y),
                    size.as_ref().map(|s| s.width),
                    size.as_ref().map(|s| s.height),
                ));
            }
        });
    })
    .build()
    .map(|win: WebviewWindow| {
        // 始终置顶在部分平台 build 后需再确认一次，避免被后续窗口压住
        let _ = win.set_always_on_top(true);
        diag(app, &format!("created: pos=({x},{y}) size=({PET_W}x{PET_H})"));
    })
    .map_err(|e| format!("创建桌面宠物窗口失败: {e}"))?;
    Ok(())
}

/// 关闭桌面宠物（destroy 不触发 CloseRequested，直接销毁窗口）
#[tauri::command]
pub fn pet_hide(app: AppHandle) -> Result<(), String> {
    // 取消在途创建：延迟窗口内 pet_show 已排队但窗口尚未注册，这里作废请求
    PET_WANTED.store(false, Ordering::SeqCst);
    if let Some(win) = app.get_webview_window(PET_LABEL) {
        let _ = win.destroy();
    }
    Ok(())
}

/// 调整宠物窗口大小（scale 1.0 = 150×180）：设置页修改宠物大小时即时生效；
/// 窗口重定位到主显示器右下角，避免放大后超出屏幕
#[tauri::command]
pub fn resize_pet(app: AppHandle, scale: f64) -> Result<(), String> {
    let Some(win) = app.get_webview_window(PET_LABEL) else {
        return Ok(()); // 窗口未创建：下次 pet_show 会按新 scale 创建
    };
    let (x, y) = match app.primary_monitor() {
        Ok(Some(m)) => {
            let wa = m.work_area();
            bottom_right_position(
                wa.position.x as f64,
                wa.position.y as f64,
                wa.size.width as f64,
                wa.size.height as f64,
                m.scale_factor(),
                scale,
            )
        }
        _ => bottom_right_position(0.0, 0.0, 1440.0, 900.0, 1.0, scale),
    };
    win.set_size(tauri::Size::Logical(tauri::LogicalSize {
        width: PET_W * scale,
        height: PET_H * scale,
    }))
    .map_err(|e| format!("调整宠物窗口大小失败: {e}"))?;
    win.set_position(tauri::Position::Logical(tauri::LogicalPosition {
        x,
        y,
    }))
    .map_err(|e| format!("调整宠物窗口位置失败: {e}"))?;
    diag(&app, &format!("resized: scale={scale}"));
    Ok(())
}

/// 宠物唤起主窗口：传面板 id 则切到对应功能面板，否则切到 AI 助手
#[tauri::command]
pub fn pet_open_main(app: AppHandle, panel: Option<String>) -> Result<(), String> {
    crate::show_main_window(&app);
    match panel {
        Some(id) => {
            let _ = app.emit("open-panel", id);
        }
        None => {
            let _ = app.emit("open-assistant", ());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bottom_right_position_basic() {
        // 1x 缩放，1440×900 工作区：x = 1440 - 150 - 24 = 1266，y = 900 - 180 - 24 = 696
        let (x, y) = bottom_right_position(0.0, 0.0, 1440.0, 900.0, 1.0, 1.0);
        assert_eq!((x.round(), y.round()), (1266.0, 696.0));
    }

    #[test]
    fn test_bottom_right_position_scaled() {
        // 2x 缩放（Retina）：物理 2880×1800 → 逻辑 1440×900，结果与 1x 一致
        let (x, y) = bottom_right_position(0.0, 0.0, 2880.0, 1800.0, 2.0, 1.0);
        assert_eq!((x.round(), y.round()), (1266.0, 696.0));
    }

    #[test]
    fn test_bottom_right_position_offset_work_area() {
        // 工作区带偏移（如 macOS 菜单栏 25px：y 从 25 开始，高 875）
        let (x, y) = bottom_right_position(0.0, 25.0, 1440.0, 875.0, 1.0, 1.0);
        assert_eq!((x.round(), y.round()), (1266.0, 696.0)); // 25 + 875 - 180 - 24 = 696
    }

    #[test]
    fn test_bottom_right_position_invalid_scale() {
        // 非法缩放回退 1x，不 panic
        let (x, y) = bottom_right_position(0.0, 0.0, 1440.0, 900.0, 0.0, 1.0);
        assert_eq!((x.round(), y.round()), (1266.0, 696.0));
    }

    #[test]
    fn test_fmt_utc_known_values() {
        // epoch / 闰年 2/29 / 闰年 3/1 / 普通日期 + 时分秒，覆盖年边界与闰年算法
        assert_eq!(fmt_utc(0), "1970-01-01 00:00:00 UTC");
        assert_eq!(fmt_utc(951_782_400), "2000-02-29 00:00:00 UTC");
        assert_eq!(fmt_utc(951_868_800), "2000-03-01 00:00:00 UTC");
        assert_eq!(fmt_utc(1_756_339_200 + 3661), "2025-08-28 01:01:01 UTC");
    }
}
