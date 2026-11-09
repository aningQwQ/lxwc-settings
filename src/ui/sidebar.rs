use eframe::egui::{self, RichText};

use crate::app::App;
use crate::i18n::{self, t, Lang};
use crate::modules::Category;

pub fn show(app: &mut App, ctx: &egui::Context) {
    egui::SidePanel::left("sidebar")
        .exact_width(220.0)
        .resizable(false)
        .show(ctx, |ui| {
            ui.add_space(12.0);
            ui.heading(t("lxwc_settings"));
            ui.label(RichText::new(t("labwc_lxqt_control_center")).small().weak());

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(t("language")).small().weak());
                let mut new_lang = app.lang;
                egui::ComboBox::from_id_source("language-select")
                    .selected_text(app.lang.label())
                    .width(120.0)
                    .show_ui(ui, |ui| {
                        for lang in Lang::ALL {
                            ui.selectable_value(&mut new_lang, lang, lang.label());
                        }
                    });
                if new_lang != app.lang {
                    app.lang = new_lang;
                    i18n::set_current(new_lang);
                    i18n::save(new_lang);
                }
            });

            ui.add_space(6.0);
            ui.separator();

            for category in Category::ALL {
                ui.add_space(8.0);
                ui.label(RichText::new(category.label()).small().weak());
                for (index, module) in app.modules.iter().enumerate() {
                    let meta = module.meta();
                    if meta.category != category {
                        continue;
                    }
                    let count: usize = app
                        .pending
                        .iter()
                        .flat_map(|task| task.attribution.iter())
                        .filter(|(module_id, _)| *module_id == meta.id)
                        .map(|(_, count)| *count)
                        .sum();
                    let title = t(meta.title_key);
                    let text = if count > 0 {
                        format!("{title}   ({count})")
                    } else {
                        title.to_string()
                    };
                    let selected = app.current == index;
                    if ui
                        .add_sized(
                            [ui.available_width(), 32.0],
                            egui::SelectableLabel::new(selected, text),
                        )
                        .clicked()
                    {
                        app.current = index;
                    }
                }
            }
        });
}
