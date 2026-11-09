use eframe::egui::{self, RichText};

use crate::app::App;
use crate::core::model::MANAGED_ENV;
use crate::ui::theme;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    ui.label("这些开关写入 ~/.config/labwc/environment，仅在下次启动 labwc 会话后生效。");
    if let Some(baseline) = &app.baseline {
        if !baseline.env_exists {
            ui.label(
                RichText::new(format!(
                    "{} 不存在，应用时会自动创建。",
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
                    .checkbox(&mut checked, RichText::new(item.label).strong())
                    .changed()
                {
                    changed = true;
                }
                ui.label(RichText::new(item.key).monospace().weak());
            });
            ui.label(RichText::new(item.help).small().weak());
            ui.label(
                RichText::new(format!(
                    "文件现状：{}",
                    baseline_value
                        .map(|value| format!("{}={}", item.key, value))
                        .unwrap_or_else(|| "未设置".to_string())
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
        RichText::new("提示：这些变量不会热重载。应用后请注销并重新登录以验证效果。")
            .small()
            .weak(),
    );
}
