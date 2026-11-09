use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use eframe::egui::{self, RichText};

use crate::core::model::{ApplyReport, Effect, Task};
use crate::i18n::t;
use crate::kanshi;
use crate::labwc::{autostart, paths};
use crate::modules::{Category, ModuleCtx, ModuleMeta, ModulePayload, SettingsModule};
use crate::ui::theme;
use crate::wlr::{self, Mode, OutputInfo};

pub const MODULE_ID: &str = "labwc-display";
const TASK_ID: &str = "display-default";
const DYNAMIC_TASK_ID: &str = "display-dynamic";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DisplayPersist {
    None,
    Static,
    Dynamic,
}

pub struct LabwcDisplayModule {
    autostart_path: PathBuf,
    autostart_raw: String,
    autostart_exists: bool,
    autostart_error: Option<String>,
    kanshi_path: PathBuf,
    kanshi_raw: String,
    kanshi_exists: bool,
    kanshi_error: Option<String>,
    outputs: Vec<OutputInfo>,
    scan_error: Option<String>,
    scan_rx: Option<Receiver<Result<Vec<OutputInfo>, String>>>,
    selected_output: Option<String>,
    selected_mode: Option<Mode>,
    display_persist: DisplayPersist,
}

impl LabwcDisplayModule {
    pub fn new() -> Self {
        Self::with_paths(paths::autostart(), kanshi::config_path())
    }

    fn with_paths(autostart_path: PathBuf, kanshi_path: PathBuf) -> Self {
        let mut module = Self {
            autostart_path,
            autostart_raw: String::new(),
            autostart_exists: false,
            autostart_error: None,
            kanshi_path,
            kanshi_raw: String::new(),
            kanshi_exists: false,
            kanshi_error: None,
            outputs: Vec::new(),
            scan_error: None,
            scan_rx: None,
            selected_output: None,
            selected_mode: None,
            display_persist: DisplayPersist::None,
        };
        module.reload_autostart();
        module.reload_kanshi();
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

    fn reload_kanshi(&mut self) {
        match kanshi::load(&self.kanshi_path) {
            Ok(file) => {
                self.kanshi_raw = file.raw;
                self.kanshi_exists = file.exists;
                self.kanshi_error = None;
            }
            Err(err) => {
                self.kanshi_error = Some(err);
            }
        }
    }

    fn detect_persist(&self) -> DisplayPersist {
        if autostart::managed_block(&self.autostart_raw).is_some() {
            DisplayPersist::Static
        } else if kanshi::managed_block(&self.kanshi_raw).is_some() {
            DisplayPersist::Dynamic
        } else {
            DisplayPersist::None
        }
    }

    fn selection_from_config(&mut self) {
        let parsed = autostart::managed_block(&self.autostart_raw)
            .and_then(|block| autostart::parse_command(&block))
            .or_else(|| {
                kanshi::managed_block(&self.kanshi_raw)
                    .and_then(|block| kanshi::parse_profile(&block))
            });
        if let Some((output, spec)) = parsed {
            self.selected_output = Some(output);
            self.selected_mode = wlr::parse_spec(&spec);
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
        self.scan_rx = Some(wlr::scan_async());
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
            DisplayPersist::None => [self.clear_static_task(), self.clear_dynamic_task()]
                .into_iter()
                .flatten()
                .collect(),
            DisplayPersist::Static => self.static_task().into_iter().collect(),
            DisplayPersist::Dynamic => self.dynamic_task().into_iter().collect(),
        }
    }

    fn static_task(&self) -> Option<Task> {
        if self.autostart_error.is_some() {
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

    fn clear_static_task(&self) -> Option<Task> {
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

    fn clear_dynamic_task(&self) -> Option<Task> {
        if self.kanshi_error.is_some() || kanshi::managed_block(&self.kanshi_raw).is_none() {
            return None;
        }
        let contents = kanshi::remove_block(&self.kanshi_raw);
        Some(Task {
            id: DYNAMIC_TASK_ID.to_string(),
            module_id: MODULE_ID,
            title: t("clear_persisted_display").to_string(),
            detail: t("clear_persisted_display_help").to_string(),
            change_count: 1,
            attribution: vec![(MODULE_ID, 1)],
            effects: vec![Effect::WriteFile {
                path: self.kanshi_path.clone(),
                contents,
                expected: Some(self.kanshi_raw.clone()),
            }],
        })
    }

    fn dynamic_task(&self) -> Option<Task> {
        if self.kanshi_error.is_some() {
            return None;
        }
        let output_name = self.selected_output.as_ref()?;
        let mode = self.selected_mode.as_ref()?;
        let profile = kanshi::profile(&self.outputs, output_name, mode)?;
        let contents = kanshi::contents(&self.kanshi_raw, &profile);
        if contents == self.kanshi_raw {
            return None;
        }

        Some(Task {
            id: DYNAMIC_TASK_ID.to_string(),
            module_id: MODULE_ID,
            title: t("persist_dynamic_display").to_string(),
            detail: format!("{} {} — kanshi", output_name, mode.label()),
            change_count: 1,
            attribution: vec![(MODULE_ID, 1)],
            effects: vec![Effect::WriteFile {
                path: self.kanshi_path.clone(),
                contents,
                expected: if self.kanshi_exists {
                    Some(self.kanshi_raw.clone())
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
        self.reload_kanshi();
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
        if report.task_succeeded(MODULE_ID, DYNAMIC_TASK_ID)
            || report.task_failed(MODULE_ID, DYNAMIC_TASK_ID)
        {
            self.reload_kanshi();
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
                ui.label(RichText::new(t("output_and_mode")).strong());
                if ui.button(t("reload_and_rescan")).clicked() {
                    ctx.info(t("rescanning"));
                    self.reload_autostart();
                    self.reload_kanshi();
                    self.display_persist = self.detect_persist();
                    self.selection_from_config();
                    self.scan_outputs();
                }
            });

            if self.scan_rx.is_some() {
                ui.label(RichText::new(t("scanning")).small().weak());
            } else if let Some(err) = &self.scan_error {
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
                    let modes = output.modes.clone();
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
                                    let selected = selected_mode
                                        .as_ref()
                                        .map(|current| current.same_mode(mode))
                                        .unwrap_or(false);
                                    if ui.selectable_label(selected, mode.label()).clicked() {
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
            let dynamic_block = kanshi::managed_block(&self.kanshi_raw);

            section(ui, "current_written");
            block_view(ui, "current_autostart", static_block.as_deref());
            ui.label(
                RichText::new(crate::trf!("autostart_path", self.autostart_path.display()))
                    .small()
                    .weak(),
            );
            block_view(ui, "current_kanshi", dynamic_block.as_deref());
            ui.label(
                RichText::new(crate::trf!("kanshi_path", self.kanshi_path.display()))
                    .small()
                    .weak(),
            );

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
            ui.radio_value(
                &mut self.display_persist,
                DisplayPersist::Dynamic,
                t("dynamic_persist"),
            );

            let any_block = static_block.is_some() || dynamic_block.is_some();
            if any_block && self.display_persist != DisplayPersist::None {
                if ui.button(t("clear_persisted_display")).clicked() {
                    self.display_persist = DisplayPersist::None;
                }
            }

            let selection = self.selected_output.clone().zip(self.selected_mode.clone());
            match self.display_persist {
                DisplayPersist::None => {
                    if any_block {
                        ui.colored_label(
                            theme::WARN,
                            RichText::new(t("will_clear_persisted_display")).small(),
                        );
                    } else {
                        ui.label(RichText::new(t("not_persisting_help")).small().weak());
                    }
                }
                DisplayPersist::Static | DisplayPersist::Dynamic => {
                    if let Some((output_name, mode)) = &selection {
                        ui.label(crate::trf!("will_persist", output_name, mode.label()));
                        if self.display_persist == DisplayPersist::Dynamic {
                            ui.label(RichText::new(t("dynamic_persist_help")).small().weak());
                        }

                        ui.label(RichText::new(t("will_write")).small().strong());
                        match self.display_persist {
                            DisplayPersist::Static => {
                                let command = wlr::set_mode_command(output_name, &mode.spec());
                                ui.label(
                                    RichText::new(crate::trf!("autostart_command", command))
                                        .monospace()
                                        .small(),
                                );
                            }
                            DisplayPersist::Dynamic => {
                                if let Some(profile) =
                                    kanshi::profile(&self.outputs, output_name, mode)
                                {
                                    ui.label(
                                        RichText::new(t("kanshi_profile_preview")).small().strong(),
                                    );
                                    ui.label(RichText::new(profile).monospace().small());
                                } else {
                                    ui.label(
                                        RichText::new(t("no_outputs_detected")).small().weak(),
                                    );
                                }
                            }
                            DisplayPersist::None => {}
                        }

                        let file_error = match self.display_persist {
                            DisplayPersist::Static => self.autostart_error.is_some(),
                            DisplayPersist::Dynamic => self.kanshi_error.is_some(),
                            DisplayPersist::None => false,
                        };
                        if !file_error {
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

            if let Some(err) = &self.autostart_error {
                ui.colored_label(theme::DANGER, RichText::new(err).small());
            }
            if let Some(err) = &self.kanshi_error {
                ui.colored_label(theme::DANGER, RichText::new(err).small());
            }
            if let Some(err) = ctx.failures.get(TASK_ID) {
                ui.colored_label(theme::DANGER, RichText::new(err).small());
            }
            if let Some(err) = ctx.failures.get(DYNAMIC_TASK_ID) {
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

fn block_view(ui: &mut egui::Ui, title: &'static str, block: Option<&str>) {
    ui.label(RichText::new(t(title)).small().strong());
    match block {
        Some(block) => {
            ui.label(RichText::new(block).monospace().small());
        }
        None => {
            ui.label(RichText::new(t("not_set")).small().weak());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::model::TaskResult;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_paths() -> (PathBuf, PathBuf, PathBuf) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("lxwc-labwc-display-test-{unique}"));
        std::fs::create_dir_all(&dir).unwrap();
        (dir.clone(), dir.join("autostart"), dir.join("kanshi"))
    }

    fn report_id(id: &str, ok: bool) -> ApplyReport {
        ApplyReport {
            results: vec![TaskResult {
                id: id.to_string(),
                module_id: MODULE_ID,
                title: id.to_string(),
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

    fn report(ok: bool) -> ApplyReport {
        report_id(TASK_ID, ok)
    }

    #[test]
    fn detects_existing_persist_mode() {
        let (dir, autostart_path, kanshi_path) = temp_paths();
        let mut module =
            LabwcDisplayModule::with_paths(autostart_path.clone(), kanshi_path.clone());
        assert_eq!(module.display_persist, DisplayPersist::None);

        std::fs::write(
            &autostart_path,
            autostart::contents("", "wlr-randr --output A --mode 1x1@60.000000Hz"),
        )
        .unwrap();
        module = LabwcDisplayModule::with_paths(autostart_path, kanshi_path);
        assert_eq!(module.display_persist, DisplayPersist::Static);
        assert_eq!(module.selected_output.as_deref(), Some("A"));
        assert_eq!(
            module.selected_mode.as_ref().map(|mode| mode.spec()),
            Some("1x1@60.000000Hz".to_string())
        );

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn reads_selection_from_kanshi_profile() {
        let (dir, autostart_path, kanshi_path) = temp_paths();
        std::fs::write(
            &kanshi_path,
            kanshi::contents(
                "",
                "profile {\n    output \"HDMI-A-1\" mode 1920x1080@60.000000Hz position 0,0\n}",
            ),
        )
        .unwrap();
        let module = LabwcDisplayModule::with_paths(autostart_path, kanshi_path);
        assert_eq!(module.display_persist, DisplayPersist::Dynamic);
        assert_eq!(module.selected_output.as_deref(), Some("HDMI-A-1"));
        assert_eq!(
            module.selected_mode.as_ref().map(|mode| mode.spec()),
            Some("1920x1080@60.000000Hz".to_string())
        );

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn display_default_writes_autostart() {
        let (dir, autostart, kanshi) = temp_paths();
        let mut module = LabwcDisplayModule::with_paths(autostart, kanshi);
        module.selected_output = Some("HDMI-A-1".to_string());
        module.selected_mode = Some(wlr::parse_spec("1920x1080@60.000000Hz").unwrap());
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

    fn output(name: &str, mode: Mode, position: (i32, i32)) -> OutputInfo {
        OutputInfo {
            name: name.to_string(),
            description: String::new(),
            enabled: true,
            current: Some(mode.clone()),
            modes: vec![mode],
            position: Some(position),
            scale: Some(1.0),
        }
    }

    #[test]
    fn dynamic_display_writes_kanshi_profile() {
        let (dir, autostart, kanshi_path) = temp_paths();
        let mut module = LabwcDisplayModule::with_paths(autostart, kanshi_path.clone());
        module.outputs = vec![
            output(
                "eDP-1",
                Mode {
                    width: 1920,
                    height: 1080,
                    refresh: 60.0,
                },
                (0, 0),
            ),
            output(
                "HDMI-A-1",
                Mode {
                    width: 2560,
                    height: 1440,
                    refresh: 60.0,
                },
                (1920, 0),
            ),
        ];
        module.selected_output = Some("HDMI-A-1".to_string());
        module.selected_mode = Some(Mode {
            width: 1920,
            height: 1080,
            refresh: 60.0,
        });
        module.display_persist = DisplayPersist::Dynamic;

        let tasks = module.pending();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, DYNAMIC_TASK_ID);
        let Effect::WriteFile {
            path,
            contents,
            expected,
        } = &tasks[0].effects[0]
        else {
            panic!("expected kanshi write");
        };
        assert_eq!(path, &kanshi_path);
        assert!(expected.is_none());
        assert!(contents.contains("profile {"));
        assert!(contents.contains("output \"eDP-1\" mode 1920x1080@60.000000Hz position 0,0"));
        assert!(contents.contains("output \"HDMI-A-1\" mode 1920x1080@60.000000Hz position 1920,0"));

        std::fs::write(&kanshi_path, contents).unwrap();
        module.on_applied(&report_id(DYNAMIC_TASK_ID, true));
        assert!(module.pending().is_empty());

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn clear_removes_managed_block() {
        let (dir, autostart_path, kanshi_path) = temp_paths();
        std::fs::write(
            &autostart_path,
            autostart::contents("foo\n", "wlr-randr --output A --mode 1x1@60.000000Hz"),
        )
        .unwrap();
        let mut module = LabwcDisplayModule::with_paths(autostart_path.clone(), kanshi_path);
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
        let (dir, autostart, kanshi) = temp_paths();
        let mut module = LabwcDisplayModule::with_paths(autostart, kanshi);
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
