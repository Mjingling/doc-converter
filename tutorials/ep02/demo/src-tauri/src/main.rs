//! EP02：透明置顶桌宠窗口
//!
//! 要点：
//! 1. 窗口属性（透明/无边框/置顶）由 tauri.conf.json 声明
//! 2. 位置由 Rust 在 setup 阶段计算：主显示器工作区右下角（考虑 HiDPI 缩放）
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{LogicalPosition, Manager};

/// 宠物窗口尺寸与屏幕边距（逻辑像素）
const PET_W: f64 = 150.0;
const PET_H: f64 = 180.0;
const PET_MARGIN: f64 = 24.0;

/// 计算工作区右下角对应的窗口逻辑坐标（纯函数，便于单测）
///
/// 入参均为物理像素：工作区 (x, y, w, h) 与显示器缩放 scale。
/// Tauri 的 set_position 使用逻辑坐标，因此结果要除以 scale。
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
            // 取主显示器工作区（工作区 = 去掉菜单栏/任务栏的可用区域）
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_1x_scale() {
        // 1440×900 工作区：x = 1440-150-24 = 1266，y = 900-180-24 = 696
        let (x, y) = bottom_right_position(0.0, 0.0, 1440.0, 900.0, 1.0);
        assert_eq!((x.round(), y.round()), (1266.0, 696.0));
    }

    #[test]
    fn test_retina_2x() {
        // 2x 缩放（物理 2880×1800 → 逻辑 1440×900），结果与 1x 一致
        let (x, y) = bottom_right_position(0.0, 0.0, 2880.0, 1800.0, 2.0);
        assert_eq!((x.round(), y.round()), (1266.0, 696.0));
    }

    #[test]
    fn test_menu_bar_offset() {
        // macOS 菜单栏占 25px：工作区从 y=25 开始
        let (x, y) = bottom_right_position(0.0, 25.0, 1440.0, 875.0, 1.0);
        assert_eq!((x.round(), y.round()), (1266.0, 696.0));
    }
}
