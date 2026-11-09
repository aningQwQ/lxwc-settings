use eframe::egui::{self, Color32};

pub const ACCENT: Color32 = Color32::from_rgb(74, 144, 226);
pub const OK: Color32 = Color32::from_rgb(112, 200, 120);
pub const WARN: Color32 = Color32::from_rgb(240, 173, 78);
pub const DANGER: Color32 = Color32::from_rgb(235, 110, 110);

pub fn apply(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = Color32::from_rgb(27, 29, 33);
    visuals.window_fill = Color32::from_rgb(33, 36, 41);
    visuals.extreme_bg_color = Color32::from_rgb(20, 21, 24);
    visuals.faint_bg_color = Color32::from_rgb(40, 43, 49);
    visuals.selection.bg_fill = ACCENT;
    visuals.selection.stroke.color = Color32::WHITE;
    visuals.hyperlink_color = ACCENT;
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(58, 63, 72);
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    ctx.set_style(style);
}
