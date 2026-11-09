use eframe::egui::{self, RichText};

use crate::app::App;
use crate::ui::theme;

pub fn show(app: &mut App, ctx: &egui::Context) {
    egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if app.applying {
                ui.spinner();
                ui.label(RichText::new("执行中…").small());
            } else if let Some(status) = &app.status {
                let color = if status.error {
                    theme::DANGER
                } else {
                    ui.visuals().weak_text_color()
                };
                ui.colored_label(color, RichText::new(&status.text).small());
            } else {
                ui.label(RichText::new("就绪").small().weak());
            }

            let pending = app.pending.len();
            let failures = app
                .pending
                .iter()
                .filter(|task| app.failures.contains_key(&task.id))
                .count();
            let applying = app.applying;
            let has_report = app.report.is_some();

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let apply_text = format!("确认应用（{pending}）");
                let (fill, text_color) = if pending > 0 {
                    (theme::ACCENT, egui::Color32::WHITE)
                } else {
                    (
                        egui::Color32::from_rgb(228, 231, 236),
                        ui.visuals().text_color(),
                    )
                };
                let apply_button = egui::Button::new(
                    RichText::new(apply_text).strong().color(text_color),
                )
                .fill(fill);
                if ui
                    .add_enabled(pending > 0 && !applying, apply_button)
                    .clicked()
                {
                    app.confirm_open = true;
                }

                if ui
                    .add_enabled(!applying, egui::Button::new("重新扫描"))
                    .on_hover_text("清空待执行队列并重新读取 rc.xml / environment")
                    .clicked()
                {
                    app.refresh();
                }

                if has_report && ui.button("执行报告").clicked() {
                    app.report_open = true;
                }

                if failures > 0 {
                    ui.colored_label(theme::DANGER, format!("失败 {failures} 项"));
                }
                ui.label(format!("待执行 {pending} 项"));
            });
        });
        ui.add_space(2.0);
    });
}
