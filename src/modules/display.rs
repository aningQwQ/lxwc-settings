use eframe::egui::{self, RichText};

use crate::app::App;
use crate::core::model::Mode;
use crate::i18n::t;
use crate::ui::theme;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    ui.label(t("switch_output_modes_directly_via_wlr_randr_a_15_second_confirmation_window_appears_after_applying_otherwise_it_rolls_back_automatically"));
    ui.add_space(8.0);

    ui.horizontal(|ui| {
        if ui
            .button(t("rescan_displays"))
            .on_hover_text(t("restore_current_mode"))
            .clicked()
        {
            app.rescan_outputs();
        }
        if app.scanning {
            ui.spinner();
            ui.label(RichText::new(t("scanning")).weak());
        }
    });

    if let Some(err) = app.outputs_error.clone() {
        ui.add_space(8.0);
        ui.colored_label(theme::DANGER, crate::trf!("scan_failed", err));
        return;
    }

    if app.outputs.is_empty() {
        ui.add_space(8.0);
        ui.label(RichText::new(t("no_outputs_detected")).weak());
        return;
    }

    ui.add_space(10.0);
    let outputs = app.outputs.clone();
    let selected_name = app.selected_output.clone();
    let combo_text = selected_name
        .clone()
        .unwrap_or_else(|| t("select_output").to_string());
    let mut new_output: Option<String> = None;

    ui.horizontal(|ui| {
        ui.label(t("output"));
        egui::ComboBox::from_id_source("output-combo")
            .width(300.0)
            .selected_text(combo_text)
            .show_ui(ui, |ui| {
                for output in &outputs {
                    let label = if output.enabled {
                        output.name.clone()
                    } else {
                        crate::trf!("disabled", output.name)
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
                ui.colored_label(theme::OK, t("enabled"));
            } else {
                ui.colored_label(theme::DANGER, t("disabled_2"));
            }
        });
        if !output.description.is_empty() {
            ui.label(RichText::new(&output.description).small().weak());
        }
        ui.label(crate::trf!(
            "current_mode",
            output
                .current
                .as_ref()
                .map(|mode| mode.label())
                .unwrap_or_else(|| t("unknown").to_string())
        ));
    });

    ui.add_space(10.0);
    let modes = output.modes.clone();
    let selected_mode = app.selected_mode.clone();
    let mode_combo_text = selected_mode
        .as_ref()
        .map(|mode| mode.label())
        .unwrap_or_else(|| t("select_mode").to_string());
    let mut new_mode: Option<Mode> = None;

    ui.horizontal(|ui| {
        ui.label(t("resolution_refresh_rate"));
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
                        label.push_str(t("current"));
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
        let can_apply = dirty
            && output.enabled
            && output.current.is_some()
            && !inflight
            && !waiting
            && !applying;
        if ui
            .add_enabled(
                can_apply,
                egui::Button::new(RichText::new(t("apply_now_auto_rollback_in_15_s")).strong()),
            )
            .clicked()
        {
            apply_clicked = true;
        }
        if inflight {
            ui.spinner();
            ui.label(RichText::new(t("switching")).small());
        }
        if waiting {
            ui.colored_label(theme::WARN, t("waiting_for_confirmation"));
        }
    });
    if apply_clicked {
        app.request_mode_apply();
    }

    if !output.enabled {
        ui.label(
            RichText::new(t(
                "the_output_is_disabled_switching_is_blocked_to_keep_rollback_possible",
            ))
            .small()
            .weak(),
        );
    } else if output.current.is_none() {
        ui.label(
            RichText::new(t(
                "the_current_mode_could_not_be_read_so_auto_rollback_is_disabled",
            ))
            .small()
            .weak(),
        );
    }

    ui.add_space(10.0);
    ui.label(
        RichText::new(crate::trf!("available_modes", modes.len()))
            .small()
            .weak(),
    );
}
