//! EP06：运行时创建宠物窗口 + 右下角定位
//!
//! 与前几集的区别：宠物窗口不再写在 tauri.conf.json 里，
//! 而是在 setup 阶段用 WebviewWindowBuilder 动态创建——
//! 这正是成品 DocMorph 的做法（宠物可开关，窗口按需创建/销毁）。
//!
//! 关键点：加载同一个前端页面，但带上 ?window=pet，
//! 前端据此分流渲染不同的根组件。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{LogicalPosition, WebviewUrl, WebviewWindowBuilder};

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
            let win = WebviewWindowBuilder::new(
                app,
                "pet",
                WebviewUrl::App("index.html?window=pet".into()),
            )
            .title("桌宠")
            .inner_size(PET_W, PET_H)
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .resizable(false)
            .skip_taskbar(true)
            .shadow(false)
            .build()?;

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
