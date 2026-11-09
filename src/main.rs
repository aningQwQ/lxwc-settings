mod app;
mod core;
mod modules;
mod ui;

use std::process::Command as ProcessCommand;

use eframe::egui;

fn load_system_cjk_font() -> Option<Vec<u8>> {
    let output = ProcessCommand::new("fc-match")
        .args(["-f", "%{file}", "sans-serif:lang=zh-CN"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        return None;
    }
    std::fs::read(path).ok()
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1060.0, 760.0])
            .with_min_inner_size([880.0, 620.0])
            .with_title("LXWC 设置中心"),
        ..Default::default()
    };

    eframe::run_native(
        "lxwc-settings",
        options,
        Box::new(|cc| {
            let mut fonts = egui::FontDefinitions::default();
            if let Some(data) = load_system_cjk_font() {
                fonts
                    .font_data
                    .insert("cjk".to_owned(), egui::FontData::from_owned(data));
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .insert(0, "cjk".to_owned());
                fonts
                    .families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .insert(0, "cjk".to_owned());
            }
            cc.egui_ctx.set_fonts(fonts);
            ui::theme::apply(&cc.egui_ctx);
            Ok(Box::new(app::App::new(&cc.egui_ctx)) as Box<dyn eframe::App>)
        }),
    )
}
