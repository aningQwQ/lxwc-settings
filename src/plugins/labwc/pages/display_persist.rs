use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::Duration;

use eframe::egui::{self, RichText};

use crate::core::model::{ApplyReport, Effect, Task};
use crate::i18n::t;
use crate::modules::{ModuleCtx, ModuleMeta, ModulePayload, SettingsModule};
use crate::plugins::labwc::{autostart, paths, GROUP};
use crate::plugins::wlr::{Mode, OutputInfo, Outputs, WlrOutputs};
use crate::ui::theme;

pub const MODULE_ID: &str = crate::modules::DISPLAY_PERSIST_MODULE_ID;
const TASK_ID: &str = "display-default";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DisplayPersist {
    None,
    Static,
}

pub struct DisplayPersistModule {
    provider: Arc<dyn Outputs>,
    autostart_path: PathBuf,
    autostart_raw: String,
    autostart_exists: bool,
    autostart_error: Option<String>,
    outputs: Vec<OutputInfo>,
    scan_error: Option<String>,
    scan_rx: Option<Receiver<Result<Vec<OutputInfo>, String>>>,
    selected_output: Option<String>,
    selected_mode: Option<Mode>,
    display_persist: DisplayPersist,
}

impl DisplayPersistModule {
    pub fn new() -> Self {
        Self::with_path(paths::autostart())
    }

    fn with_path(autostart_path: PathBuf) -> Self {
        let mut module = Self {
            provider: Arc::new(WlrOutputs),
            autostart_path,
            autostart_raw: String::new(),
            autostart_exists: false,
            autostart_error: None,
            outputs: Vec::new(),
            scan_error: None,
            scan_rx: None,
            selected_output: None,
            selected_mode: None,
            display_persist: DisplayPersist::None,
        };
        module.reload_autostart();
        module.display_persist = module.detect_persist();
        module.selection_from_config();
        module
    }

    fn reload_autostart(&mut self) {
        match autostart::load(&self.autostart_path) {
            Ok(file) => {
                self.autostart_raw = file.raw;
                self.autostart_exists = file.exists;
                self.autostart_error = None;
            }
            Err(err) => {
                self.autostart_error = Some(err);
            }
        }
    }

    fn detect_persist(&self) -> DisplayPersist {
        if autostart::managed_block(&self.autostart_raw).is_some() {
            DisplayPersist::Static
        } else {
            DisplayPersist::None
        }
    }

    fn restore_from_config(&mut self) {
        self.reload_autostart();
        self.display_persist = self.detect_persist();
        self.selection_from_config();
    }

    fn saved_default(&self) -> Option<(String, Mode)> {
        autostart::managed_block(&self.autostart_raw)
            .and_then(|block| autostart::parse_saved(&block))
    }

    fn selection_from_config(&mut self) {
        if let Some((output, mode)) = self.saved_default() {
            self.selected_output = Some(output);
            self.selected_mode = Some(mode);
        } else {
            self.selected_output = None;
            self.selected_mode = None;
        }
    }

    fn scan_outputs(&mut self) {
        if self.scan_rx.is_some() {
            return;
        }
        self.scan_error = None;
        let provider = self.provider.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let _ = std::thread::Builder::new()
            .name("lxwc-output-scan".to_string())
            .spawn(move || {
                let _ = tx.send(provider.scan());
            });
        self.scan_rx = Some(rx);
    }

    fn poll_scan(&mut self) {
        let Some(rx) = &self.scan_rx else {
            return;
        };
        match rx.try_recv() {
            Ok(Ok(outputs)) => {
                self.outputs = outputs;
                self.sync_selection();
                self.scan_error = None;
                self.scan_rx = None;
            }
            Ok(Err(err)) => {
                self.scan_error = Some(err);
                self.scan_rx = None;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.scan_error = Some(crate::trf!("scan_failed", "worker stopped"));
                self.scan_rx = None;
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

        let Some(name) = self.selected_output.clone() else {
            self.selected_mode = None;
            return;
        };
        let Some(output) = self.outputs.iter_mut().find(|output| output.name == name) else {
            self.selected_mode = None;
            return;
        };
        if let Some(mode) = self.selected_mode.clone() {
            if !output.modes.iter().any(|item| item.same_mode(&mode)) {
                output.modes.push(mode);
            }
        } else {
            self.selected_mode = output.modes.first().cloned();
        }
    }

    fn selected_output_info(&self) -> Option<OutputInfo> {
        let name = self.selected_output.as_ref()?;
        self.outputs
            .iter()
            .find(|output| &output.name == name)
            .cloned()
    }

    fn pending_tasks(&self) -> Vec<Task> {
        match self.display_persist {
            DisplayPersist::None => self.clear_task().into_iter().collect(),
            DisplayPersist::Static => self.static_task().into_iter().collect(),
        }
    }

    fn static_task(&self) -> Option<Task> {
        if self.autostart_error.is_some() {
            return None;
        }
        let output_name = self.selected_output.as_ref()?;
        let mode = self.selected_mode.as_ref()?;
        let command = autostart::mode_command(output_name, mode);
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

    fn clear_task(&self) -> Option<Task> {
        if self.autostart_error.is_some() || autostart::managed_block(&self.autostart_raw).is_none()
        {
            return None;
        }
        let contents = autostart::remove_block(&self.autostart_raw);
        Some(Task {
            id: TASK_ID.to_string(),
            module_id: MODULE_ID,
            title: t("clear_persisted_display").to_string(),
            detail: t("clear_persisted_display_help").to_string(),
            change_count: 1,
            attribution: vec![(MODULE_ID, 1)],
            effects: vec![Effect::WriteFile {
                path: self.autostart_path.clone(),
                contents,
                expected: Some(self.autostart_raw.clone()),
            }],
        })
    }
}

impl SettingsModule for DisplayPersistModule {
    fn meta(&self) -> ModuleMeta {
        ModuleMeta {
            id: MODULE_ID,
            title_key: "default_display_configuration",
            subtitle_key: "default_display_subtitle",
            group: GROUP,
        }
    }

    fn refresh(&mut self) {
        self.restore_from_config();
        self.scan_outputs();
    }

    fn update(&mut self, ui_ctx: &egui::Context, _ctx: &mut ModuleCtx) {
        if self.scan_rx.is_some() {
            ui_ctx.request_repaint_after(Duration::from_millis(100));
        }
        self.poll_scan();
    }

    fn pending(&mut self) -> Vec<Task> {
        self.pending_tasks()
    }

    fn on_applied(&mut self, report: &ApplyReport) {
        if report.task_succeeded(MODULE_ID, TASK_ID) || report.task_failed(MODULE_ID, TASK_ID) {
            self.reload_autostart();
        }
    }

    fn on_message(&mut self, payload: &ModulePayload) {
        match payload {
            ModulePayload::DisplayDefault { output, spec } => {
                self.selected_output = Some(output.clone());
                self.selected_mode = autostart::parse_mode(spec);
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
                ui.label(RichText::new(t("output_and_mode")).strong());
                if self.scan_rx.is_some() {
                    ui.label(RichText::new(t("scanning")).small().weak());
                }
            });

            if let Some(err) = &self.scan_error {
                ui.colored_label(theme::DANGER, RichText::new(err).small());
            } else if self.scan_rx.is_none() && self.outputs.is_empty() {
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
                    let modes = output.modes.clone();
                    let saved = self.saved_default();
                    let selected_mode = self.selected_mode.clone();
                    let mode_combo_text = selected_mode
                        .as_ref()
                        .map(|mode| mode.label())
                        .unwrap_or_else(|| t("select_mode").to_string());
                    let mut new_mode: Option<Mode> = None;

                    ui.horizontal(|ui| {
                        ui.label(t("resolution_refresh_rate"));
                        egui::ComboBox::from_id_source("default-display-mode")
                            .width(300.0)
                            .selected_text(mode_combo_text)
                            .show_ui(ui, |ui| {
                                for mode in &modes {
                                    let mut label = mode.label();
                                    if saved
                                        .as_ref()
                                        .map(|(saved_output, saved_mode)| {
                                            saved_output == &output.name
                                                && saved_mode.same_mode(mode)
                                        })
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
                }
            }

            let static_block = autostart::managed_block(&self.autostart_raw);

            section(ui, "persistence_options");
            ui.radio_value(
                &mut self.display_persist,
                DisplayPersist::None,
                t("do_not_persist"),
            );
            ui.radio_value(
                &mut self.display_persist,
                DisplayPersist::Static,
                t("static_persist"),
            );

            if static_block.is_some() && self.display_persist != DisplayPersist::None {
                if ui.button(t("clear_persisted_display")).clicked() {
                    self.display_persist = DisplayPersist::None;
                }
            }

            let selection = self.selected_output.clone().zip(self.selected_mode.clone());
            match self.display_persist {
                DisplayPersist::None => {
                    if static_block.is_some() {
                        ui.colored_label(
                            theme::WARN,
                            RichText::new(t("will_clear_persisted_display")).small(),
                        );
                    } else {
                        ui.label(RichText::new(t("not_persisting_help")).small().weak());
                    }
                }
                DisplayPersist::Static => {
                    if let Some((output_name, mode)) = &selection {
                        ui.label(crate::trf!("will_persist", output_name, mode.label()));
                        ui.label(RichText::new(t("will_write")).small().strong());
                        let command = autostart::mode_command(output_name, mode);
                        ui.label(
                            RichText::new(crate::trf!("autostart_command", command))
                                .monospace()
                                .small(),
                        );

                        if self.autostart_error.is_none() {
                            if self.pending_tasks().is_empty() {
                                ui.colored_label(theme::OK, RichText::new(t("up_to_date")).small());
                            } else {
                                ui.colored_label(
                                    theme::WARN,
                                    RichText::new(t("will_update")).small(),
                                );
                            }
                        }
                    } else {
                        ui.label(RichText::new(t("no_selection_hint")).small().weak());
                    }
                }
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

fn section(ui: &mut egui::Ui, title: &'static str) {
    ui.add_space(10.0);
    ui.separator();
    ui.label(RichText::new(t(title)).strong());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::model::TaskResult;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_paths() -> (PathBuf, PathBuf) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("lxwc-labwc-display-test-{unique}"));
        std::fs::create_dir_all(&dir).unwrap();
        let autostart = dir.join("autostart");
        (dir, autostart)
    }

    fn report(ok: bool) -> ApplyReport {
        ApplyReport {
            results: vec![TaskResult {
                id: TASK_ID.to_string(),
                module_id: MODULE_ID,
                title: TASK_ID.to_string(),
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
    fn restore_resets_unsaved_selection_to_saved_default() {
        let (dir, autostart_path) = temp_paths();
        std::fs::write(
            &autostart_path,
            autostart::contents("", "wlr-randr --output A --mode 1x1@60.000000Hz"),
        )
        .unwrap();
        let mut module = DisplayPersistModule::with_path(autostart_path);
        module.display_persist = DisplayPersist::None;
        module.selected_output = Some("B".to_string());
        module.selected_mode = autostart::parse_mode("2x2@50.000000Hz");

        module.restore_from_config();
        assert_eq!(module.display_persist, DisplayPersist::Static);
        assert_eq!(module.selected_output.as_deref(), Some("A"));
        assert_eq!(
            module.selected_mode.as_ref().map(autostart::mode_spec),
            Some("1x1@60.000000Hz".to_string())
        );

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn detects_existing_persist_mode() {
        let (dir, autostart_path) = temp_paths();
        let mut module = DisplayPersistModule::with_path(autostart_path.clone());
        assert_eq!(module.display_persist, DisplayPersist::None);

        std::fs::write(
            &autostart_path,
            autostart::contents("", "wlr-randr --output A --mode 1x1@60.000000Hz"),
        )
        .unwrap();
        module = DisplayPersistModule::with_path(autostart_path);
        assert_eq!(module.display_persist, DisplayPersist::Static);
        assert_eq!(module.selected_output.as_deref(), Some("A"));
        assert_eq!(
            module.selected_mode.as_ref().map(autostart::mode_spec),
            Some("1x1@60.000000Hz".to_string())
        );

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn saved_default_comes_from_autostart() {
        let (dir, autostart_path) = temp_paths();
        std::fs::write(
            &autostart_path,
            autostart::contents("", "wlr-randr --output A --mode 1x1@60.000000Hz"),
        )
        .unwrap();
        let module = DisplayPersistModule::with_path(autostart_path);
        let (output, mode) = module.saved_default().unwrap();
        assert_eq!(output, "A");
        assert_eq!(autostart::mode_spec(&mode), "1x1@60.000000Hz");
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn display_default_writes_autostart() {
        let (dir, autostart) = temp_paths();
        let mut module = DisplayPersistModule::with_path(autostart);
        module.selected_output = Some("HDMI-A-1".to_string());
        module.selected_mode = Some(autostart::parse_mode("1920x1080@60.000000Hz").unwrap());
        module.display_persist = DisplayPersist::Static;

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
    fn clear_removes_managed_block() {
        let (dir, autostart_path) = temp_paths();
        std::fs::write(
            &autostart_path,
            autostart::contents("foo\n", "wlr-randr --output A --mode 1x1@60.000000Hz"),
        )
        .unwrap();
        let mut module = DisplayPersistModule::with_path(autostart_path.clone());
        module.display_persist = DisplayPersist::None;

        let tasks = module.pending();
        assert_eq!(tasks.len(), 1);
        let Effect::WriteFile { contents, .. } = &tasks[0].effects[0] else {
            panic!("expected autostart write");
        };
        assert!(!contents.contains("# BEGIN lxwc-settings default display"));
        assert!(contents.contains("foo"));

        std::fs::write(&autostart_path, contents).unwrap();
        module.on_applied(&report(true));
        assert!(module.pending().is_empty());

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn handoff_prefills_selection() {
        let (dir, autostart) = temp_paths();
        let mut module = DisplayPersistModule::with_path(autostart);
        module.on_message(&ModulePayload::DisplayDefault {
            output: "HDMI-A-1".to_string(),
            spec: "1920x1080@60.000000Hz".to_string(),
        });
        assert_eq!(module.selected_output.as_deref(), Some("HDMI-A-1"));
        assert_eq!(
            module.selected_mode.as_ref().map(autostart::mode_spec),
            Some("1920x1080@60.000000Hz".to_string())
        );
        std::fs::remove_dir_all(dir).ok();
    }
}
