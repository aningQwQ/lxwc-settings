use eframe::egui::{self, RichText};

use crate::app::App;
use crate::i18n::t;
use crate::ui::theme;

pub fn show(app: &mut App, ctx: &egui::Context) {
    egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if app.applying {
                ui.spinner();
                ui.label(RichText::new(t("applying")).small());
            } else if let Some(status) = &app.status {
                let color = if status.error {
                    theme::DANGER
                } else {
                    ui.visuals().weak_text_color()
                };
                ui.colored_label(color, RichText::new(&status.text).small());
            } else {
                ui.label(RichText::new(t("ready")).small().weak());
            }

            let pending: usize = app.pending.iter().map(|task| task.change_count).sum();
            let failures: usize = app
                .pending
                .iter()
                .filter(|task| app.failures.contains_key(&task.id))
                .map(|task| task.change_count)
                .sum();
            let applying = app.applying;
            let has_report = app.report.is_some();
            let valid = app.modules.iter().all(|module| module.is_valid());

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let apply_text = crate::trf!("apply_2", pending);
                let (fill, text_color) = if pending > 0 {
                    (theme::ACCENT, egui::Color32::WHITE)
                } else {
                    (
                        egui::Color32::from_rgb(228, 231, 236),
                        ui.visuals().text_color(),
                    )
                };
                let apply_button =
                    egui::Button::new(RichText::new(apply_text).strong().color(text_color))
                        .fill(fill);
                if ui
                    .add_enabled(pending > 0 && !applying && valid, apply_button)
                    .on_disabled_hover_text(if valid {
                        ""
                    } else {
                        t("fix_invalid_input_before_applying")
                    })
                    .clicked()
                {
                    app.confirm_open = true;
                }

                if ui
                    .add_enabled(!applying, egui::Button::new(t("rescan")))
                    .on_hover_text(t("reset_queue_and_restore_defaults"))
                    .clicked()
                {
                    app.refresh();
                }

                if has_report && ui.button(t("report")).clicked() {
                    app.report_open = true;
                }

                if failures > 0 {
                    ui.colored_label(theme::DANGER, crate::trf!("failed_2", failures));
                }
                ui.label(crate::trf!("pending", pending));
            });
        });
        ui.add_space(2.0);
    });
}
