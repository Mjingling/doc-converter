//! 桌面宠物窗口：透明置顶小窗口常驻桌面右下角，渲染机器人形象
//!
//! 窗口特性：无边框 + 透明 + 始终置顶 + 不进任务栏 + 无阴影 + 关闭拖放；
//! 内容复用前端入口（`index.html?window=pet`，App.vue 按 query 分流渲染 PetWindow）。

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

/// 宠物窗口 label（capabilities 权限按此放行）
pub const PET_LABEL: &str = "pet";
/// 宠物窗口尺寸（逻辑像素）
const PET_W: f64 = 150.0;
const PET_H: f64 = 180.0;
/// 距屏幕工作区右下角的边距（逻辑像素）
const PET_MARGIN: f64 = 24.0;

/// 计算工作区右下角对应的窗口逻辑坐标（纯函数，便于单测）
///
/// 入参均为物理像素：工作区 (x, y, w, h) 与显示器缩放 scale。
fn bottom_right_position(area_x: f64, area_y: f64, area_w: f64, area_h: f64, scale: f64) -> (f64, f64) {
    let scale = if scale <= 0.0 { 1.0 } else { scale };
    let px = area_x + area_w - PET_W * scale - PET_MARGIN * scale;
    let py = area_y + area_h - PET_H * scale - PET_MARGIN * scale;
    (px / scale, py / scale)
}

/// 显示桌面宠物（已存在则直接 show；否则创建并定位到主显示器右下角）
#[tauri::command]
pub fn pet_show(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window(PET_LABEL) {
        let _ = win.show();
        return Ok(());
    }
    let (x, y) = match app.primary_monitor() {
        Ok(Some(m)) => {
            let wa = m.work_area();
            bottom_right_position(
                wa.position.x as f64,
                wa.position.y as f64,
                wa.size.width as f64,
                wa.size.height as f64,
                m.scale_factor(),
            )
        }
        _ => bottom_right_position(0.0, 0.0, 1440.0, 900.0, 1.0),
    };
    WebviewWindowBuilder::new(
        &app,
        PET_LABEL,
        WebviewUrl::App("index.html?window=pet".into()),
    )
    .title("DocMorph Pet")
    .inner_size(PET_W, PET_H)
    .position(x, y)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .shadow(false)
    .resizable(false)
    .build()
    .map(|win: WebviewWindow| {
        // 始终置顶在部分平台 build 后需再确认一次，避免被后续窗口压住
        let _ = win.set_always_on_top(true);
    })
    .map_err(|e| format!("创建桌面宠物窗口失败: {e}"))?;
    Ok(())
}

/// 关闭桌面宠物（destroy 不触发 CloseRequested，直接销毁窗口）
#[tauri::command]
pub fn pet_hide(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window(PET_LABEL) {
        let _ = win.destroy();
    }
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
        let (x, y) = bottom_right_position(0.0, 0.0, 1440.0, 900.0, 1.0);
        assert_eq!((x.round(), y.round()), (1266.0, 696.0));
    }

    #[test]
    fn test_bottom_right_position_scaled() {
        // 2x 缩放（Retina）：物理 2880×1800 → 逻辑 1440×900，结果与 1x 一致
        let (x, y) = bottom_right_position(0.0, 0.0, 2880.0, 1800.0, 2.0);
        assert_eq!((x.round(), y.round()), (1266.0, 696.0));
    }

    #[test]
    fn test_bottom_right_position_offset_work_area() {
        // 工作区带偏移（如 macOS 菜单栏 25px：y 从 25 开始，高 875）
        let (x, y) = bottom_right_position(0.0, 25.0, 1440.0, 875.0, 1.0);
        assert_eq!((x.round(), y.round()), (1266.0, 696.0)); // 25 + 875 - 180 - 24 = 696
    }

    #[test]
    fn test_bottom_right_position_invalid_scale() {
        // 非法缩放回退 1x，不 panic
        let (x, y) = bottom_right_position(0.0, 0.0, 1440.0, 900.0, 0.0);
        assert_eq!((x.round(), y.round()), (1266.0, 696.0));
    }
}
