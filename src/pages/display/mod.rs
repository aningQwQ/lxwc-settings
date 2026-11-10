use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, Instant};

use eframe::egui::{self, RichText};

use crate::backend::GROUP_OUTPUT;
use crate::core::model::{ApplyReport, Task};
use crate::i18n::t;
use crate::modules::{
    ModuleCtx, ModuleMeta, ModulePayload, ModuleRequest, SettingsModule, DISPLAY_PERSIST_MODULE_ID,
};
use crate::ui::theme;
use crate::wlr::worker::{self, Event as DisplayEvent, Request as DisplayRequest};
use crate::wlr::{Mode, OutputInfo};

const WATCHDOG_SECONDS: u64 = 15;

#[derive(Clone, Debug)]
struct PendingModeChange {
    mode: Mode,
    previous: Option<Mode>,
}

#[derive(Clone, Debug)]
struct Watchdog {
    output: String,
    mode: Mode,
    previous: Mode,
    deadline: Instant,
}

impl Watchdog {
    fn remaining_secs(&self) -> u32 {
        self.deadline
            .saturating_duration_since(Instant::now())
            .as_secs_f32()
            .ceil() as u32
    }

    fn fraction(&self) -> f32 {
        (self.remaining_secs() as f32 / WATCHDOG_SECONDS as f32).clamp(0.0, 1.0)
    }
}

pub struct DisplayModule {
    tx: Sender<DisplayRequest>,
    rx: Receiver<DisplayEvent>,
    outputs: Vec<OutputInfo>,
    outputs_error: Option<String>,
    scanning: bool,
    selected_output: Option<String>,
    selected_mode: Option<Mode>,
    inflight: Option<PendingModeChange>,
    watchdog: Option<Watchdog>,
    reset_modes_on_scan: bool,
}

impl DisplayModule {
    pub fn new(ui_ctx: &egui::Context) -> Self {
        let repaint = {
            let ctx = ui_ctx.clone();
            move || ctx.request_repaint()
        };
        let (tx, rx) = worker::spawn(repaint);
        Self {
            tx,
            rx,
            outputs: Vec::new(),
            outputs_error: None,
            scanning: false,
            selected_output: None,
            selected_mode: None,
            inflight: None,
            watchdog: None,
            reset_modes_on_scan: true,
        }
    }

    fn request_scan(&mut self, reset_modes: bool) {
        self.scanning = true;
        self.outputs_error = None;
        self.reset_modes_on_scan = reset_modes;
        if reset_modes {
            self.selected_mode = None;
            self.sync_selected_mode();
        }
        let _ = self.tx.send(DisplayRequest::Scan);
    }

    fn apply_scan_result(&mut self, result: Result<Vec<OutputInfo>, String>) {
        self.scanning = false;
        match result {
            Ok(outputs) => {
                self.outputs_error = None;
                self.outputs = outputs;
                let still_exists = self
                    .selected_output
                    .as_ref()
                    .map(|name| self.outputs.iter().any(|item| &item.name == name))
                    .unwrap_or(false);
                if !still_exists {
                    self.selected_output = self
                        .outputs
                        .iter()
                        .find(|item| item.enabled)
                        .or_else(|| self.outputs.first())
                        .map(|item| item.name.clone());
                    self.selected_mode = None;
                }
                if std::mem::take(&mut self.reset_modes_on_scan) {
                    self.selected_mode = None;
                }
                self.sync_selected_mode();
            }
            Err(err) => {
                self.outputs_error = Some(err);
            }
        }
    }

    fn selected_output_info(&self) -> Option<OutputInfo> {
        let name = self.selected_output.as_ref()?;
        self.outputs.iter().find(|item| &item.name == name).cloned()
    }

    fn sync_selected_mode(&mut self) {
        let Some(output) = self.selected_output_info() else {
            self.selected_mode = None;
            return;
        };
        let valid = self
            .selected_mode
            .as_ref()
            .map(|mode| output.modes.iter().any(|item| item.same_mode(mode)))
            .unwrap_or(false);
        if !valid {
            self.selected_mode = output
                .current
                .clone()
                .or_else(|| output.modes.first().cloned());
        }
    }

    fn request_mode_apply(&mut self, ctx: &mut ModuleCtx) {
        if self.watchdog.is_some() || self.inflight.is_some() {
            return;
        }
        let Some(output) = self.selected_output_info() else {
            return;
        };
        let Some(mode) = self.selected_mode.clone() else {
            return;
        };
        if !output.enabled || output.current.is_none() {
            ctx.error(t("the_output_is_disabled_refusing_to_switch"));
            return;
        }
        if output
            .current
            .as_ref()
            .map(|current| current.same_mode(&mode))
            .unwrap_or(false)
        {
            return;
        }
        self.inflight = Some(PendingModeChange {
            mode: mode.clone(),
            previous: output.current.clone(),
        });
        ctx.info(crate::trf!("switching_to", mode.label()));
        let _ = self.tx.send(DisplayRequest::ApplyMode {
            output: output.name,
            spec: mode.spec(),
        });
    }

    fn revert_mode(&mut self, ctx: &mut ModuleCtx) {
        if let Some(watchdog) = self.watchdog.take() {
            let _ = self.tx.send(DisplayRequest::RevertMode {
                output: watchdog.output.clone(),
                spec: watchdog.previous.spec(),
            });
            ctx.info(crate::trf!("restoring", watchdog.output));
        }
    }
}

impl SettingsModule for DisplayModule {
    fn meta(&self) -> ModuleMeta {
        ModuleMeta {
            id: "display",
            title_key: "display_output",
            subtitle_key:
                "manage_outputs_directly_via_wlr_randr_with_a_15_second_rollback_watchdog",
            group: GROUP_OUTPUT,
        }
    }

    fn refresh(&mut self) {
        self.request_scan(true);
    }

    fn pending(&mut self) -> Vec<Task> {
        Vec::new()
    }

    fn on_applied(&mut self, _report: &ApplyReport) {}

    fn update(&mut self, ui_ctx: &egui::Context, ctx: &mut ModuleCtx) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                DisplayEvent::Outputs(result) => {
                    if let Err(err) = &result {
                        ctx.error(crate::trf!("scan_failed", err));
                    }
                    self.apply_scan_result(result);
                }
                DisplayEvent::ModeApplied { output, result } => {
                    let pending = self.inflight.take();
                    match result {
                        Ok(()) => {
                            self.selected_output = Some(output.clone());
                            self.sync_selected_mode();
                            let previous = pending.as_ref().and_then(|item| item.previous.clone());
                            let requested = pending.as_ref().map(|item| item.mode.clone());
                            match (previous, requested) {
                                (Some(previous), Some(mode)) => {
                                    self.watchdog = Some(Watchdog {
                                        output: output.clone(),
                                        mode: mode.clone(),
                                        previous,
                                        deadline: Instant::now()
                                            + Duration::from_secs(WATCHDOG_SECONDS),
                                    });
                                    ctx.info(crate::trf!(
                                        "switched_to_confirm_within_seconds_or_it_will_roll_back",
                                        mode.label(),
                                        WATCHDOG_SECONDS
                                    ));
                                }
                                _ => {
                                    ctx.error(t(
                                        "mode_switched_but_the_previous_mode_is_unavailable_auto_rollback_is_disabled",
                                    ));
                                }
                            }
                            self.request_scan(false);
                        }
                        Err(err) => {
                            ctx.error(crate::trf!("failed_to_switch_mode", err));
                            self.request_scan(false);
                        }
                    }
                }
                DisplayEvent::ModeReverted { output, result } => {
                    match result {
                        Ok(()) => ctx.info(crate::trf!("restored_to_its_previous_mode", output)),
                        Err(err) => ctx.error(crate::trf!("restore_failed", err)),
                    }
                    self.request_scan(false);
                }
            }
        }

        let deadline = match &self.watchdog {
            Some(watchdog) => watchdog.deadline,
            None => return,
        };
        let now = Instant::now();
        if now >= deadline {
            self.revert_mode(ctx);
        } else {
            ui_ctx.request_repaint_after((deadline - now).min(Duration::from_millis(200)));
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut ModuleCtx) {
        ui.label(t("switch_output_modes_directly_via_wlr_randr_a_15_second_confirmation_window_appears_after_applying_otherwise_it_rolls_back_automatically"));
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            if ui
                .button(t("rescan_displays"))
                .on_hover_text(t("restore_current_mode"))
                .clicked()
            {
                ctx.info(t("rescanning"));
                self.request_scan(true);
            }
            if self.scanning {
                ui.spinner();
                ui.label(RichText::new(t("scanning")).weak());
            }
        });

        if let Some(err) = self.outputs_error.clone() {
            ui.add_space(8.0);
            ui.colored_label(theme::DANGER, crate::trf!("scan_failed", err));
            return;
        }

        if self.outputs.is_empty() {
            ui.add_space(8.0);
            ui.label(RichText::new(t("no_outputs_detected")).weak());
            return;
        }

        ui.add_space(10.0);
        let outputs = self.outputs.clone();
        let selected_name = self.selected_output.clone();
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
            self.selected_output = Some(name);
            self.sync_selected_mode();
        }

        let Some(output) = self.selected_output_info() else {
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
        let selected_mode = self.selected_mode.clone();
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
            self.selected_mode = Some(mode);
        }

        let dirty = self
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
        let inflight = self.inflight.is_some();
        let waiting = self.watchdog.is_some();

        ui.add_space(10.0);
        let mut apply_clicked = false;
        ui.horizontal(|ui| {
            let can_apply =
                dirty && output.enabled && output.current.is_some() && !inflight && !waiting;
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
            self.request_mode_apply(ctx);
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

    fn overlay(&mut self, ui_ctx: &egui::Context, ctx: &mut ModuleCtx) {
        let Some(watchdog) = self.watchdog.clone() else {
            return;
        };
        let mut keep = false;
        let mut revert = false;
        let mut set_default = false;

        egui::Window::new(t("confirm_display_mode"))
            .anchor(egui::Align2::CENTER_TOP, [0.0, 24.0])
            .collapsible(false)
            .resizable(false)
            .show(ui_ctx, |ui| {
                ui.set_min_width(440.0);
                ui.label(crate::trf!(
                    "switched_to",
                    watchdog.output,
                    watchdog.mode.label()
                ));
                ui.add(
                    egui::ProgressBar::new(watchdog.fraction())
                        .text(crate::trf!("s", watchdog.remaining_secs()))
                        .animate(false),
                );
                ui.label(crate::trf!(
                    "seconds_until_auto_revert_to",
                    watchdog.remaining_secs(),
                    watchdog.previous.label()
                ));
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    if ui.button(t("roll_back_now")).clicked() {
                        revert = true;
                    }
                    if ui.button(t("keep_this_mode")).clicked() {
                        keep = true;
                    }
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(t("set_as_boot_default"))
                                    .strong()
                                    .color(egui::Color32::WHITE),
                            )
                            .fill(theme::ACCENT),
                        )
                        .clicked()
                    {
                        set_default = true;
                    }
                });
            });

        if revert {
            self.revert_mode(ctx);
        } else if keep {
            self.watchdog = None;
            ctx.info(t("new_display_mode_kept"));
        } else if set_default {
            let output = watchdog.output.clone();
            let spec = watchdog.mode.spec();
            self.watchdog = None;
            ctx.requests.push(ModuleRequest {
                target: DISPLAY_PERSIST_MODULE_ID,
                payload: ModulePayload::DisplayDefault { output, spec },
            });
            ctx.info(t("display_default_handed_off"));
        }
    }
}
