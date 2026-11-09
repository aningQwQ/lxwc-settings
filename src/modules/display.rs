use eframe::egui::{self, RichText};

use crate::app::App;
use crate::core::model::Mode;
use crate::i18n::t;
use crate::ui::theme;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    ui.label(t(
        "通过 wlr-randr 直接切换输出模式。应用后会弹出 15 秒确认窗口，未确认将自动回滚。",
        "Switch output modes directly via wlr-randr. A 15-second confirmation window appears after applying; otherwise it rolls back automatically.",
    ));
    ui.add_space(8.0);

    ui.horizontal(|ui| {
        if ui
            .button(t("重新扫描显示器", "Rescan displays"))
            .on_hover_text(t(
                "恢复当前分辨率",
                "Restore current mode",
            ))
            .clicked()
        {
            app.rescan_outputs();
        }
        if app.scanning {
            ui.spinner();
            ui.label(RichText::new(t("扫描中…", "Scanning…")).weak());
        }
    });

    if let Some(err) = app.outputs_error.clone() {
        ui.add_space(8.0);
        ui.colored_label(theme::DANGER, crate::trf!("扫描失败：{err}", "Scan failed: {err}"));
        return;
    }

    if app.outputs.is_empty() {
        ui.add_space(8.0);
        ui.label(RichText::new(t("没有检测到任何输出。", "No outputs detected.")).weak());
        return;
    }

    ui.add_space(10.0);
    let outputs = app.outputs.clone();
    let selected_name = app.selected_output.clone();
    let combo_text = selected_name
        .clone()
        .unwrap_or_else(|| t("选择输出", "Select output").to_string());
    let mut new_output: Option<String> = None;

    ui.horizontal(|ui| {
        ui.label(t("输出设备", "Output"));
        egui::ComboBox::from_id_source("output-combo")
            .width(300.0)
            .selected_text(combo_text)
            .show_ui(ui, |ui| {
                for output in &outputs {
                    let label = if output.enabled {
                        output.name.clone()
                    } else {
                        crate::trf!("{}（未启用）", "{} (disabled)", output.name)
                    };
                    let selected = selected_name.as_deref() == Some(output.name.as_str());
                    if ui.selectable_label(selected, label).clicked() {
                        new_output = Some(output.name.clone());
                    }
                }
            });
    });

    if let Some(name) = new_output {
        app.selected_output = Some(name);
        app.sync_selected_mode();
    }

    let Some(output) = app.selected_output_info() else {
        return;
    };

    ui.add_space(8.0);
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(&output.name).strong());
            if output.enabled {
                ui.colored_label(theme::OK, t("已启用", "Enabled"));
            } else {
                ui.colored_label(theme::DANGER, t("未启用", "Disabled"));
            }
        });
        if !output.description.is_empty() {
            ui.label(RichText::new(&output.description).small().weak());
        }
        ui.label(crate::trf!(
            "当前模式：{}",
            "Current mode: {}",
            output
                .current
                .as_ref()
                .map(|mode| mode.label())
                .unwrap_or_else(|| t("未知", "Unknown").to_string())
        ));
    });

    ui.add_space(10.0);
    let modes = output.modes.clone();
    let selected_mode = app.selected_mode.clone();
    let mode_combo_text = selected_mode
        .as_ref()
        .map(|mode| mode.label())
        .unwrap_or_else(|| t("选择模式", "Select mode").to_string());
    let mut new_mode: Option<Mode> = None;

    ui.horizontal(|ui| {
        ui.label(t("分辨率 / 刷新率", "Resolution / Refresh rate"));
        egui::ComboBox::from_id_source("mode-combo")
            .width(300.0)
            .selected_text(mode_combo_text)
            .show_ui(ui, |ui| {
                for mode in &modes {
                    let mut label = mode.label();
                    if output
                        .current
                        .as_ref()
                        .map(|current| current.same_mode(mode))
                        .unwrap_or(false)
                    {
                        label.push_str(t("（当前）", " (current)"));
                    }
                    let selected = selected_mode
                        .as_ref()
                        .map(|current| current.same_mode(mode))
                        .unwrap_or(false);
                    if ui.selectable_label(selected, label).clicked() {
                        new_mode = Some(mode.clone());
                    }
                }
            });
    });

    if let Some(mode) = new_mode {
        app.selected_mode = Some(mode);
    }

    let dirty = app
        .selected_mode
        .as_ref()
        .map(|mode| {
            output
                .current
                .as_ref()
                .map(|current| !current.same_mode(mode))
                .unwrap_or(true)
        })
        .unwrap_or(false);
    let inflight = app.mode_inflight.is_some();
    let waiting = app.watchdog.is_some();
    let applying = app.applying;

    ui.add_space(10.0);
    let mut apply_clicked = false;
    ui.horizontal(|ui| {
        let can_apply =
            dirty && output.enabled && output.current.is_some() && !inflight && !waiting && !applying;
        if ui
            .add_enabled(
                can_apply,
                egui::Button::new(
                    RichText::new(t(
                        "立即应用（15 秒后自动回滚）",
                        "Apply now (auto-rollback in 15 s)",
                    ))
                    .strong(),
                ),
            )
            .clicked()
        {
            apply_clicked = true;
        }
        if inflight {
            ui.spinner();
            ui.label(RichText::new(t("正在切换…", "Switching…")).small());
        }
        if waiting {
            ui.colored_label(theme::WARN, t("等待确认中…", "Waiting for confirmation…"));
        }
    });
    if apply_clicked {
        app.request_mode_apply();
    }

    if !output.enabled {
        ui.label(
            RichText::new(t(
                "输出未启用，禁止切换，避免无法回滚。",
                "The output is disabled; switching is blocked to keep rollback possible.",
            ))
                .small()
                .weak(),
        );
    } else if output.current.is_none() {
        ui.label(
            RichText::new(t(
                "无法读取当前模式，已禁用自动回滚。",
                "The current mode could not be read, so auto-rollback is disabled.",
            ))
                .small()
                .weak(),
        );
    }

    ui.add_space(10.0);
    ui.label(
        RichText::new(crate::trf!(
            "共 {} 个可用模式",
            "{} available modes",
            modes.len(),
        ))
            .small()
            .weak(),
    );
}
