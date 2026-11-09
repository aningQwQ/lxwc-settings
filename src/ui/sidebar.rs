use eframe::egui::{self, RichText};

use crate::app::App;
use crate::core::model::ModuleKind;

pub fn show(app: &mut App, ctx: &egui::Context) {
    egui::SidePanel::left("sidebar")
        .exact_width(220.0)
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
        });
}
