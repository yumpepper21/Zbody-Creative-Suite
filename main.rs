// 始终以 GUI 子系统启动，避免 debug 运行时额外弹出控制台窗口
#![windows_subsystem = "windows"]

mod zbody_vector;

use eframe::egui;
use std::sync::Arc;

fn main() -> eframe::Result {
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Zbody Vector")
        .with_inner_size([1180.0, 760.0])
        .with_min_inner_size([860.0, 560.0]);

    if let Some(icon) = zbody_vector::load_icon() {
        viewport = viewport.with_icon(Arc::new(icon));
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Zbody Vector",
        native_options,
        Box::new(|cc| Ok(Box::new(zbody_vector::ZbodyVectorApp::new(cc)))),
    )
}
