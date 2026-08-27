//! EP03：与 EP02 相同 —— 透明置顶窗口 + 右下角定位。
//! 本篇的重心在前端行为引擎，Rust 侧无新增内容。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{LogicalPosition, Manager};

const PET_W: f64 = 150.0;
const PET_H: f64 = 180.0;
const PET_MARGIN: f64 = 24.0;

fn bottom_right_position(area_x: f64, area_y: f64, area_w: f64, area_h: f64, scale: f64) -> (f64, f64) {
    let scale = if scale <= 0.0 { 1.0 } else { scale };
    let px = area_x + area_w - PET_W * scale - PET_MARGIN * scale;
    let py = area_y + area_h - PET_H * scale - PET_MARGIN * scale;
    (px / scale, py / scale)
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let win = app.get_webview_window("pet").expect("宠物窗口不存在");
            if let Ok(Some(m)) = win.primary_monitor() {
                let wa = m.work_area();
                let (x, y) = bottom_right_position(
                    wa.position.x as f64,
                    wa.position.y as f64,
                    wa.size.width as f64,
                    wa.size.height as f64,
                    m.scale_factor(),
                );
                let _ = win.set_position(LogicalPosition::new(x, y));
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("启动 Tauri 应用失败");
}
