use eframe::egui::{self, Color32};

pub const ACCENT: Color32 = Color32::from_rgb(31, 111, 235);
pub const OK: Color32 = Color32::from_rgb(30, 126, 52);
pub const WARN: Color32 = Color32::from_rgb(154, 103, 0);
pub const DANGER: Color32 = Color32::from_rgb(190, 30, 45);

pub fn apply(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::light();
    visuals.panel_fill = Color32::from_rgb(246, 247, 249);
    visuals.window_fill = Color32::WHITE;
    visuals.extreme_bg_color = Color32::WHITE;
    visuals.faint_bg_color = Color32::from_rgb(237, 240, 244);
    visuals.selection.bg_fill = Color32::from_rgb(210, 227, 252);
    visuals.selection.stroke.color = ACCENT;
    visuals.hyperlink_color = ACCENT;
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(233, 236, 240);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(223, 232, 246);
    visuals.widgets.active.bg_fill = Color32::from_rgb(210, 224, 243);
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (egui::TextStyle::Small, egui::FontId::proportional(13.0)),
        (egui::TextStyle::Body, egui::FontId::proportional(15.5)),
        (egui::TextStyle::Button, egui::FontId::proportional(15.0)),
        (egui::TextStyle::Heading, egui::FontId::proportional(22.0)),
        (egui::TextStyle::Monospace, egui::FontId::monospace(14.5)),
    ]
    .into();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    ctx.set_style(style);
}
