use eframe::egui::{self, RichText};

use crate::app::App;
use crate::core::model::ModuleKind;

pub fn show(app: &mut App, ctx: &egui::Context) {
    egui::SidePanel::left("sidebar")
        .exact_width(200.0)
        .resizable(false)
        .show(ctx, |ui| {
            ui.add_space(12.0);
            ui.heading("LXWC 设置");
            ui.label(RichText::new("Labwc × LXQt 控制中心").small().weak());
            ui.add_space(6.0);
            ui.separator();

            for category in ["硬件与显示", "桌面集成", "高级魔改"] {
                ui.add_space(8.0);
                ui.label(RichText::new(category).small().weak());
                for module in ModuleKind::ALL
                    .into_iter()
                    .filter(|module| module.category() == category)
                {
                    let count = app
                        .pending
                        .iter()
                        .filter(|task| task.module == module)
                        .count();
                    let text = if count > 0 {
                        format!("{}   ({})", module.title(), count)
                    } else {
                        module.title().to_string()
                    };
                    let selected = app.current == module;
                    if ui
                        .add_sized(
                            [ui.available_width(), 32.0],
                            egui::SelectableLabel::new(selected, text),
                        )
                        .clicked()
                    {
                        app.current = module;
                    }
                }
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.add_space(8.0);
                if let Some(baseline) = &app.baseline {
                    ui.label(
                        RichText::new(format!("env: {}", baseline.env_path.display()))
                            .small()
                            .weak(),
                    );
                    ui.label(
                        RichText::new(format!("rc.xml: {}", baseline.rc_path.display()))
                            .small()
                            .weak(),
                    );
                } else if let Some(err) = &app.load_error {
                    ui.colored_label(
                        crate::ui::theme::DANGER,
                        RichText::new(err).small(),
                    );
                }
                ui.separator();
                ui.label(RichText::new("手改配置后点“重新扫描”").small().weak());
            });
        });
}
