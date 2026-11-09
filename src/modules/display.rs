use eframe::egui::{self, RichText};

use crate::app::App;
use crate::core::model::Mode;
use crate::ui::theme;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    ui.label(
        "通过 wlr-randr 直接切换输出模式。应用后会弹出 15 秒确认窗口，未确认将自动回滚。",
    );
    ui.add_space(8.0);

    ui.horizontal(|ui| {
        if ui.button("重新扫描显示器").clicked() {
            app.rescan_outputs();
        }
        if app.scanning {
            ui.spinner();
            ui.label(RichText::new("扫描中…").weak());
        }
    });

    if let Some(err) = app.outputs_error.clone() {
        ui.add_space(8.0);
        ui.colored_label(theme::DANGER, format!("扫描失败：{err}"));
        return;
    }

    if app.outputs.is_empty() {
        ui.add_space(8.0);
        ui.label(RichText::new("没有检测到任何输出。").weak());
        return;
    }

    ui.add_space(10.0);
    let outputs = app.outputs.clone();
    let selected_name = app.selected_output.clone();
    let combo_text = selected_name
        .clone()
        .unwrap_or_else(|| "选择输出".to_string());
    let mut new_output: Option<String> = None;

    ui.horizontal(|ui| {
        ui.label("输出设备");
        egui::ComboBox::from_id_source("output-combo")
            .width(300.0)
            .selected_text(combo_text)
            .show_ui(ui, |ui| {
                for output in &outputs {
                    let label = if output.enabled {
                        output.name.clone()
                    } else {
                        format!("{}（未启用）", output.name)
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
                ui.colored_label(theme::OK, "已启用");
            } else {
                ui.colored_label(theme::DANGER, "未启用");
            }
        });
        if !output.description.is_empty() {
            ui.label(RichText::new(&output.description).small().weak());
        }
        ui.label(format!(
            "当前模式：{}",
            output
                .current
                .as_ref()
                .map(|mode| mode.label())
                .unwrap_or_else(|| "未知".to_string())
        ));
    });

    ui.add_space(10.0);
    let modes = output.modes.clone();
    let selected_mode = app.selected_mode.clone();
    let mode_combo_text = selected_mode
        .as_ref()
        .map(|mode| mode.label())
        .unwrap_or_else(|| "选择模式".to_string());
    let mut new_mode: Option<Mode> = None;

    ui.horizontal(|ui| {
        ui.label("分辨率 / 刷新率");
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
                        label.push_str("（当前）");
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
                    RichText::new("立即应用（15 秒后自动回滚）").strong(),
                ),
            )
            .clicked()
        {
            apply_clicked = true;
        }
        if inflight {
            ui.spinner();
            ui.label(RichText::new("正在切换…").small());
        }
        if waiting {
            ui.colored_label(theme::WARN, "等待确认中…");
        }
    });
    if apply_clicked {
        app.request_mode_apply();
    }

    if !output.enabled {
        ui.label(
            RichText::new("输出未启用，禁止切换，避免无法回滚。")
                .small()
                .weak(),
        );
    } else if output.current.is_none() {
        ui.label(
            RichText::new("无法读取当前模式，已禁用自动回滚。")
                .small()
                .weak(),
        );
    }

    ui.add_space(10.0);
    ui.label(
        RichText::new(format!("共 {} 个可用模式", modes.len()))
            .small()
            .weak(),
    );
}
