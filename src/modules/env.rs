use eframe::egui::{self, RichText};

use crate::app::App;
use crate::core::model::MANAGED_ENV;
use crate::i18n::t;
use crate::ui::theme;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    ui.label(t("these_switches_are_written_to_config_labwc_environment_and_only_take_effect_the_next_time_the_labwc_session_starts"));
    if let Some(baseline) = &app.baseline {
        if !baseline.env_exists {
            ui.label(
                RichText::new(crate::trf!(
                    "does_not_exist_and_will_be_created_when_applying",
                    baseline.env_path.display()
                ))
                .small()
                .weak(),
            );
        }
    }
    ui.add_space(10.0);

    for item in MANAGED_ENV {
        let mut checked = app.env_value(item.key).is_some();
        let baseline_value = app
            .baseline
            .as_ref()
            .and_then(|baseline| baseline.environment.get(item.key).cloned());
        let set_id = format!("env:{}={}", item.key, "1");
        let unset_id = format!("env:{}", item.key);
        let failure = app
            .failure_for(&set_id)
            .cloned()
            .or_else(|| app.failure_for(&unset_id).cloned());
        let mut changed = false;

        ui.group(|ui| {
            ui.horizontal(|ui| {
                if ui
                    .checkbox(&mut checked, RichText::new(item.label()).strong())
                    .changed()
                {
                    changed = true;
                }
                ui.label(RichText::new(item.key).monospace().weak());
            });
            ui.label(RichText::new(item.help()).small().weak());
            ui.label(
                RichText::new(crate::trf!(
                    "current_file",
                    baseline_value
                        .map(|value| format!("{}={}", item.key, value))
                        .unwrap_or_else(|| t("not_set").to_string())
                ))
                .small(),
            );
            if let Some(err) = &failure {
                ui.colored_label(theme::DANGER, RichText::new(err).small());
            }
        });

        if changed {
            app.set_env(item.key, checked);
        }

        ui.add_space(6.0);
    }

    ui.separator();
    ui.label(
        RichText::new(t(
            "these_variables_are_not_hot_reloaded_log_out_and_back_in_to_verify_the_effect",
        ))
        .small()
        .weak(),
    );
}
