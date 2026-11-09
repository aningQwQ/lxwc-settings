use std::path::PathBuf;

use eframe::egui::{self, RichText};

use crate::core::model::{ApplyReport, Effect, Task};
use crate::i18n::t;
use crate::labwc::{autostart, paths};
use crate::modules::{Category, ModuleCtx, ModuleMeta, ModulePayload, SettingsModule};
use crate::services::fs;
use crate::ui::theme;
use crate::wlr::{self, Mode, OutputInfo};

pub const MODULE_ID: &str = "labwc-display";
const TASK_ID: &str = "display-default";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DisplayPersist {
    Static,
    Dynamic,
}

pub struct LabwcDisplayModule {
    autostart_path: PathBuf,
    autostart_raw: String,
    autostart_exists: bool,
    autostart_error: Option<String>,
    outputs: Vec<OutputInfo>,
    scan_error: Option<String>,
    selected_output: Option<String>,
    selected_mode: Option<Mode>,
    display_persist: DisplayPersist,
}

impl LabwcDisplayModule {
    pub fn new() -> Self {
        Self::with_path(paths::autostart())
    }

    fn with_path(autostart_path: PathBuf) -> Self {
        let mut module = Self {
            autostart_path,
            autostart_raw: String::new(),
            autostart_exists: false,
            autostart_error: None,
            outputs: Vec::new(),
            scan_error: None,
            selected_output: None,
            selected_mode: None,
            display_persist: DisplayPersist::Static,
        };
        module.reload_autostart();
        module
    }

    fn reload_autostart(&mut self) {
        match fs::read_optional(&self.autostart_path) {
            Ok((raw, exists)) => {
                self.autostart_raw = raw;
                self.autostart_exists = exists;
                self.autostart_error = None;
            }
            Err(err) => {
                self.autostart_error = Some(err);
            }
        }
    }

    fn scan_outputs(&mut self) {
        match wlr::scan() {
            Ok(outputs) => {
                self.scan_error = None;
                self.outputs = outputs;
                self.sync_selection();
            }
            Err(err) => {
                self.scan_error = Some(err);
            }
        }
    }

    fn sync_selection(&mut self) {
        let selected_exists = self
            .selected_output
            .as_ref()
            .map(|name| self.outputs.iter().any(|output| &output.name == name))
            .unwrap_or(false);
        if !selected_exists {
            self.selected_output = self
                .outputs
                .iter()
                .find(|output| output.enabled)
                .or_else(|| self.outputs.first())
                .map(|output| output.name.clone());
            self.selected_mode = None;
        }

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

    fn selected_output_info(&self) -> Option<OutputInfo> {
        let name = self.selected_output.as_ref()?;
        self.outputs
            .iter()
            .find(|output| &output.name == name)
            .cloned()
    }

    fn pending_task(&self) -> Option<Task> {
        if self.autostart_error.is_some() {
            return None;
        }
        if self.display_persist != DisplayPersist::Static {
            return None;
        }
        let output_name = self.selected_output.as_ref()?;
        let mode = self.selected_mode.as_ref()?;
        let command = wlr::set_mode_command(output_name, &mode.spec());
        let contents = autostart::contents(&self.autostart_raw, &command);
        if contents == self.autostart_raw {
            return None;
        }

        Some(Task {
            id: TASK_ID.to_string(),
            module_id: MODULE_ID,
            title: t("persist_default_display").to_string(),
            detail: format!("{} {} — {}", output_name, mode.label(), command),
            change_count: 1,
            attribution: vec![(MODULE_ID, 1)],
            effects: vec![Effect::WriteFile {
                path: self.autostart_path.clone(),
                contents,
                expected: if self.autostart_exists {
                    Some(self.autostart_raw.clone())
                } else {
                    None
                },
            }],
        })
    }
}

impl SettingsModule for LabwcDisplayModule {
    fn meta(&self) -> ModuleMeta {
        ModuleMeta {
            id: MODULE_ID,
            title_key: "default_display_configuration",
            subtitle_key: "default_display_subtitle",
            category: Category::Labwc,
        }
    }

    fn refresh(&mut self) {
        self.reload_autostart();
        self.scan_outputs();
    }

    fn pending(&mut self) -> Vec<Task> {
        self.pending_task().into_iter().collect()
    }

    fn on_applied(&mut self, report: &ApplyReport) {
        let succeeded = report.task_succeeded(MODULE_ID, TASK_ID);
        let failed = report.task_failed(MODULE_ID, TASK_ID);
        if succeeded || failed {
            self.reload_autostart();
        }
    }

    fn on_message(&mut self, payload: &ModulePayload) {
        match payload {
            ModulePayload::DisplayDefault { output, spec } => {
                self.selected_output = Some(output.clone());
                self.selected_mode = wlr::parse_spec(spec);
                self.display_persist = DisplayPersist::Static;

                if let Some(mode) = &self.selected_mode {
                    if let Some(info) = self.outputs.iter_mut().find(|item| &item.name == output) {
                        if !info.modes.iter().any(|item| item.same_mode(mode)) {
                            info.modes.push(mode.clone());
                        }
                    }
                }
            }
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut ModuleCtx) {
        ui.scope(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(t("persistence_options")).strong());
                if ui.button(t("rescan")).clicked() {
                    ctx.info(t("rescanning"));
                    self.scan_outputs();
                }
            });

            if let Some(err) = &self.scan_error {
                ui.colored_label(theme::DANGER, RichText::new(err).small());
            } else if self.outputs.is_empty() {
                ui.label(RichText::new(t("no_outputs_detected")).weak());
            }

            if !self.outputs.is_empty() {
                let outputs = self.outputs.clone();
                let selected_name = self.selected_output.clone();
                let combo_text = selected_name
                    .clone()
                    .unwrap_or_else(|| t("select_output").to_string());
                let mut new_output: Option<String> = None;

                ui.horizontal(|ui| {
                    ui.label(t("output"));
                    egui::ComboBox::from_id_source("default-display-output")
                        .width(300.0)
                        .selected_text(combo_text)
                        .show_ui(ui, |ui| {
                            for output in &outputs {
                                let label = if output.enabled {
                                    output.name.clone()
                                } else {
                                    crate::trf!("disabled", output.name)
                                };
                                let selected =
                                    selected_name.as_deref() == Some(output.name.as_str());
                                if ui.selectable_label(selected, label).clicked() {
                                    new_output = Some(output.name.clone());
                                }
                            }
                        });
                });

                if let Some(name) = new_output {
                    self.selected_output = Some(name);
                    self.selected_mode = None;
                    self.sync_selection();
                }

                if let Some(output) = self.selected_output_info() {
                    let current_label = output
                        .current
                        .as_ref()
                        .map(|mode| mode.label())
                        .unwrap_or_else(|| t("unknown").to_string());
                    ui.label(crate::trf!("current_mode", current_label));

                    let modes = output.modes.clone();
                    let selected_mode = self.selected_mode.clone();
                    let mode_combo_text = selected_mode
                        .as_ref()
                        .map(|mode| mode.label())
                        .unwrap_or_else(|| t("select_mode").to_string());
                    let mut new_mode: Option<Mode> = None;
                    let mut use_current = false;

                    ui.horizontal(|ui| {
                        ui.label(t("resolution_refresh_rate"));
                        egui::ComboBox::from_id_source("default-display-mode")
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
                        if ui.button(t("use_current_mode")).clicked() {
                            use_current = true;
                        }
                    });

                    if let Some(mode) = new_mode {
                        self.selected_mode = Some(mode);
                    }
                    if use_current {
                        self.selected_mode = output.current.clone();
                    }
                }
            }

            let selection = self.selected_output.clone().zip(self.selected_mode.clone());
            if let Some((output_name, mode)) = &selection {
                ui.add_space(4.0);
                ui.label(crate::trf!("will_persist", output_name, mode.label()));
                ui.radio_value(
                    &mut self.display_persist,
                    DisplayPersist::Static,
                    t("static_persist"),
                );
                ui.add_enabled_ui(false, |ui| {
                    ui.radio_value(
                        &mut self.display_persist,
                        DisplayPersist::Dynamic,
                        t("dynamic_persist"),
                    );
                });
                ui.label(RichText::new(t("dynamic_not_implemented")).small().weak());

                let command = format!("wlr-randr --output {} --mode {}", output_name, mode.spec());
                ui.label(
                    RichText::new(crate::trf!("autostart_command", command))
                        .monospace()
                        .small(),
                );
                if self.pending_task().is_none() && self.autostart_error.is_none() {
                    ui.colored_label(
                        theme::OK,
                        RichText::new(t("default_display_persisted")).small(),
                    );
                }
            } else {
                ui.label(RichText::new(t("no_selection_hint")).small().weak());
            }

            ui.label(
                RichText::new(crate::trf!("autostart_path", self.autostart_path.display()))
                    .small()
                    .weak(),
            );

            if let Some(err) = &self.autostart_error {
                ui.colored_label(theme::DANGER, RichText::new(err).small());
            }
            if let Some(err) = ctx.failures.get(TASK_ID) {
                ui.colored_label(theme::DANGER, RichText::new(err).small());
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::model::TaskResult;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path() -> (PathBuf, PathBuf) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("lxwc-labwc-display-test-{unique}"));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("autostart");
        (dir, path)
    }

    fn report(ok: bool) -> ApplyReport {
        ApplyReport {
            results: vec![TaskResult {
                id: TASK_ID.to_string(),
                module_id: MODULE_ID,
                title: "display-default".to_string(),
                change_count: 1,
                ok,
                error: if ok {
                    None
                } else {
                    Some("write failed".to_string())
                },
            }],
            warnings: Vec::new(),
            elapsed_ms: 0,
        }
    }

    #[test]
    fn display_default_writes_autostart() {
        let (dir, path) = temp_path();
        let mut module = LabwcDisplayModule::with_path(path);
        module.selected_output = Some("HDMI-A-1".to_string());
        module.selected_mode = Some(wlr::parse_spec("1920x1080@60.000000Hz").unwrap());

        let tasks = module.pending();
        assert_eq!(tasks.len(), 1);
        let Effect::WriteFile {
            path,
            contents,
            expected,
        } = &tasks[0].effects[0]
        else {
            panic!("expected file write");
        };
        assert_eq!(path.file_name().unwrap(), "autostart");
        assert!(expected.is_none());
        assert!(contents.contains("wlr-randr --output HDMI-A-1 --mode 1920x1080@60.000000Hz"));
        std::fs::write(path, contents).unwrap();

        module.on_applied(&report(true));
        assert!(module.pending().is_empty());

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn handoff_prefills_selection() {
        let (dir, path) = temp_path();
        let mut module = LabwcDisplayModule::with_path(path);
        module.on_message(&ModulePayload::DisplayDefault {
            output: "HDMI-A-1".to_string(),
            spec: "1920x1080@60.000000Hz".to_string(),
        });
        assert_eq!(module.selected_output.as_deref(), Some("HDMI-A-1"));
        assert_eq!(
            module.selected_mode.as_ref().map(|mode| mode.spec()),
            Some("1920x1080@60.000000Hz".to_string())
        );
        std::fs::remove_dir_all(dir).ok();
    }
}
