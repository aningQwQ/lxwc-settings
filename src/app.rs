use eframe::egui;
use std::collections::HashMap;

use crate::backend::Backend;
use crate::backends;
use crate::config::ConfigEdit;
use crate::core::model::{ApplyReport, Effect, Task};
use crate::i18n::{self, t, Lang};
use crate::modules::{ModuleCtx, SettingsModule, StatusMessage};
use crate::services::fs;
use crate::services::worker::{WorkerEvent, WorkerHandle, WorkerRequest};
use crate::ui::{sidebar, status_bar, theme};

pub struct App {
    worker: WorkerHandle,
    pub backend: Backend,
    pub modules: Vec<Box<dyn SettingsModule>>,
    pub current: usize,
    pub lang: Lang,
    pub confirm_open: bool,
    pub applying: bool,
    pub failures: HashMap<String, String>,
    pub pending: Vec<Task>,
    pub status: Option<StatusMessage>,
    pub report: Option<ApplyReport>,
    pub report_open: bool,
}

impl App {
    pub fn new(ctx: &egui::Context) -> Self {
        let lang = i18n::initial();
        i18n::set_current(lang);

        let backend = backends::create(backends::detect());
        let mut modules = backends::modules(backend.id, ctx);
        for module in &mut modules {
            module.refresh();
        }

        let worker = WorkerHandle::spawn(ctx.clone());
        Self {
            worker,
            backend,
            modules,
            current: 0,
            lang,
            confirm_open: false,
            applying: false,
            failures: HashMap::new(),
            pending: Vec::new(),
            status: Some(StatusMessage::info(t("ready"))),
            report: None,
            report_open: false,
        }
    }

    fn collect_tasks(&mut self) -> Vec<Task> {
        let mut tasks = Vec::new();
        for module in &mut self.modules {
            tasks.extend(module.pending());
        }

        let (file_tasks, error) = collect_file_tasks(&mut self.modules, &self.backend);
        tasks.extend(file_tasks);

        if let Some(error) = error {
            self.status = Some(StatusMessage::error(error));
        }

        tasks
    }

    fn poll_events(&mut self) {
        while let Some(WorkerEvent::Applied { report }) = self.worker.try_recv() {
            self.on_applied(report);
        }
    }

    fn on_applied(&mut self, report: ApplyReport) {
        self.applying = false;

        for module in &mut self.modules {
            module.on_applied(&report);
        }

        self.failures.clear();
        for result in &report.results {
            if !result.ok {
                if let Some(err) = &result.error {
                    self.failures.insert(result.id.clone(), err.clone());
                }
            }
        }

        let success = report.success_count();
        let failure = report.failure_count();
        let warnings = report.warnings.len();
        let mut text = crate::trf!("done_succeeded", success);
        if failure > 0 {
            text.push_str(&crate::trf!("failed_kept_in_queue", failure));
        }
        if warnings > 0 {
            text.push_str(&crate::trf!("warnings_suffix", warnings));
        }
        self.status = Some(StatusMessage {
            text,
            error: failure > 0,
        });
        self.report_open = failure > 0 || warnings > 0;
        self.report = Some(report);
    }

    pub fn refresh(&mut self) {
        if self.applying {
            return;
        }
        self.confirm_open = false;
        self.failures.clear();
        self.status = Some(StatusMessage::info(t("reloading_configuration")));
        for module in &mut self.modules {
            module.refresh();
        }
    }

    pub fn execute_pending(&mut self) {
        self.confirm_open = false;
        if self.pending.is_empty()
            || self.applying
            || self.modules.iter().any(|module| !module.is_valid())
        {
            return;
        }
        self.applying = true;
        self.status = Some(StatusMessage::info(t("applying_changes")));
        self.worker
            .send(WorkerRequest::ApplyTasks(self.pending.clone()));
    }

    fn confirm_dialog(&mut self, ctx: &egui::Context) {
        if !self.confirm_open {
            return;
        }
        let tasks = self.pending.clone();
        let change_count: usize = tasks.iter().map(|task| task.change_count).sum();
        let mut cancel = false;
        let mut go = false;

        egui::Window::new(t("confirm_apply"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(520.0);
                ui.label(crate::trf!("about_to_apply_changes", change_count));
                ui.add_space(4.0);
                egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                    for (index, task) in tasks.iter().enumerate() {
                        ui.horizontal_wrapped(|ui| {
                            ui.monospace(format!("{:>2}.", index + 1));
                            ui.label(egui::RichText::new(&task.title).strong());
                        });
                        ui.label(egui::RichText::new(&task.detail).small().weak());
                        ui.add_space(4.0);
                    }
                });
                ui.separator();
                ui.label(
                    egui::RichText::new(t(
                        "files_are_backed_up_as_lxwc_bak_before_writing_failed_tasks_stay_queued_for_retry",
                    ))
                    .small()
                    .weak(),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button(t("apply")).clicked() {
                        go = true;
                    }
                    if ui.button(t("cancel")).clicked() {
                        cancel = true;
                    }
                });
            });

        if go {
            self.execute_pending();
        } else if cancel {
            self.confirm_open = false;
        }
    }

    fn report_dialog(&mut self, ctx: &egui::Context) {
        if !self.report_open {
            return;
        }
        let Some(report) = self.report.clone() else {
            self.report_open = false;
            return;
        };
        let mut close = false;

        egui::Window::new(t("apply_report"))
            .collapsible(false)
            .resizable(true)
            .default_width(560.0)
            .show(ctx, |ui| {
                for result in &report.results {
                    ui.horizontal_wrapped(|ui| {
                        if result.ok {
                            ui.colored_label(theme::OK, t("success"));
                        } else {
                            ui.colored_label(theme::DANGER, t("failed"));
                        }
                        ui.label(&result.title);
                    });
                    if let Some(err) = &result.error {
                        ui.label(egui::RichText::new(err).small().color(theme::DANGER));
                    }
                }

                if !report.warnings.is_empty() {
                    ui.separator();
                    ui.colored_label(theme::WARN, t("warnings"));
                    for warning in &report.warnings {
                        ui.label(egui::RichText::new(warning).small());
                    }
                }

                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(crate::trf!("took_ms", report.elapsed_ms)).small(),
                    );
                    if ui.button(t("close")).clicked() {
                        close = true;
                    }
                });
            });

        if close {
            self.report_open = false;
        }
    }
}

/// Aggregate every page's intent into one task per backend file.
///
/// The loop knows nothing about file formats or paths: it reads the backend's
/// file list, asks each format to apply the collected edits, then runs the
/// page-specific `patch` hooks.
fn collect_file_tasks(
    modules: &mut [Box<dyn SettingsModule>],
    backend: &Backend,
) -> (Vec<Task>, Option<String>) {
    struct FileState {
        raw: String,
        expected: Option<String>,
        summaries: Vec<String>,
        counts: HashMap<&'static str, usize>,
    }

    let mut errors = Vec::new();
    let mut states = Vec::with_capacity(backend.files.len());
    for file in &backend.files {
        let (raw, exists) = match fs::read_optional(&file.path) {
            Ok(value) => value,
            Err(err) => {
                errors.push(err);
                (String::new(), false)
            }
        };
        states.push(FileState {
            expected: exists.then(|| raw.clone()),
            raw,
            summaries: Vec::new(),
            counts: HashMap::new(),
        });
    }

    let mut per_file: Vec<Vec<(&'static str, ConfigEdit)>> =
        backend.files.iter().map(|_| Vec::new()).collect();
    for module in modules.iter_mut() {
        let module_id = module.meta().id;
        for edit in module.config_edits() {
            match backend
                .files
                .iter()
                .position(|file| file.id == edit.file_id)
            {
                Some(index) => per_file[index].push((module_id, edit)),
                None => errors.push(format!("unknown config file id: {}", edit.file_id)),
            }
        }
    }

    for (index, edits) in per_file.into_iter().enumerate() {
        if edits.is_empty() {
            continue;
        }
        let plain = edits
            .iter()
            .map(|(_, edit)| edit.clone())
            .collect::<Vec<_>>();
        match backend.files[index]
            .format
            .apply(&states[index].raw, &plain)
        {
            Ok(applied) => {
                let mut changed = false;
                for ((module_id, edit), is_changed) in edits.iter().zip(applied.changed.iter()) {
                    if !is_changed {
                        continue;
                    }
                    states[index].summaries.push(edit.summary.clone());
                    *states[index].counts.entry(*module_id).or_default() += 1;
                    changed = true;
                }
                if changed {
                    states[index].raw = applied.contents;
                }
            }
            Err(err) => errors.push(err),
        }
    }

    for module in modules.iter_mut() {
        let module_id = module.meta().id;
        for (index, file) in backend.files.iter().enumerate() {
            match module.patch(file.id, &states[index].raw) {
                Ok(Some(patch)) => {
                    if patch.count > 0 {
                        states[index].raw = patch.contents;
                        states[index].summaries.push(patch.summary);
                        *states[index].counts.entry(module_id).or_default() += patch.count;
                    }
                }
                Ok(None) => {}
                Err(err) => errors.push(err),
            }
        }
    }

    let mut tasks = Vec::new();
    for (index, file) in backend.files.iter().enumerate() {
        let state = &mut states[index];
        let count: usize = state.counts.values().sum();
        if count == 0 {
            continue;
        }
        let mut attribution = state
            .counts
            .iter()
            .map(|(module_id, count)| (*module_id, *count))
            .collect::<Vec<_>>();
        attribution.sort_by_key(|(module_id, _)| *module_id);

        tasks.push(Task {
            id: file.id.to_string(),
            module_id: file.id,
            title: i18n::format_template(t(file.title_key), &[&count]),
            detail: state.summaries.join("; "),
            change_count: count,
            attribution,
            effects: vec![Effect::WriteFile {
                path: file.path.clone(),
                contents: std::mem::take(&mut state.raw),
                expected: state.expected.clone(),
            }],
        });
    }

    if let Some(command) = &backend.reload {
        let reload_file = backend
            .files
            .iter()
            .rev()
            .find(|file| file.needs_reload)
            .map(|file| file.id);
        if let Some(task) = tasks
            .iter_mut()
            .find(|task| Some(task.id.as_str()) == reload_file)
        {
            task.effects.push(Effect::RunCommand {
                program: command.program.clone(),
                args: command.args.clone(),
                best_effort: true,
            });
        }
    }

    (tasks, errors.into_iter().next())
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        i18n::set_current(self.lang);
        self.poll_events();

        let mut requests = Vec::new();

        {
            let modules = &mut self.modules;
            let failures = &self.failures;
            let status = &mut self.status;
            let requests = &mut requests;
            let mut module_ctx = ModuleCtx {
                failures,
                status,
                requests,
            };
            for module in modules {
                module.update(ctx, &mut module_ctx);
            }
        }

        self.pending = self.collect_tasks();

        sidebar::show(self, ctx);
        status_bar::show(self, ctx);

        let current = self.current;
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let meta = self.modules.get(current).map(|module| module.meta());
                    if let Some(meta) = meta {
                        ui.add_space(6.0);
                        ui.heading(t(meta.title_key));
                        ui.label(egui::RichText::new(t(meta.subtitle_key)).weak());
                        ui.add_space(10.0);
                    }

                    let modules = &mut self.modules;
                    let failures = &self.failures;
                    let status = &mut self.status;
                    let requests = &mut requests;
                    let mut module_ctx = ModuleCtx {
                        failures,
                        status,
                        requests,
                    };
                    if let Some(module) = modules.get_mut(current) {
                        module.ui(ui, &mut module_ctx);
                    }
                });
        });

        {
            let modules = &mut self.modules;
            let failures = &self.failures;
            let status = &mut self.status;
            let requests = &mut requests;
            let mut module_ctx = ModuleCtx {
                failures,
                status,
                requests,
            };
            for module in modules {
                module.overlay(ctx, &mut module_ctx);
            }
        }

        for request in requests {
            if let Some(index) = self
                .modules
                .iter()
                .position(|module| module.meta().id == request.target)
            {
                self.modules[index].on_message(&request.payload);
                self.current = index;
            }
        }

        self.confirm_dialog(ctx);
        self.report_dialog(ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::{Group, GROUP_OUTPUT};
    use crate::backends::labwc;
    use crate::config::{CommandSpec, ConfigFile};
    use crate::modules::ModuleMeta;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct FakeModule {
        id: &'static str,
        edits: Vec<ConfigEdit>,
    }

    impl SettingsModule for FakeModule {
        fn meta(&self) -> ModuleMeta {
            ModuleMeta {
                id: self.id,
                title_key: "appearance",
                subtitle_key: "appearance_subtitle",
                group: GROUP_OUTPUT,
            }
        }

        fn refresh(&mut self) {}

        fn on_applied(&mut self, _report: &ApplyReport) {}

        fn ui(&mut self, _ui: &mut egui::Ui, _ctx: &mut ModuleCtx) {}

        fn config_edits(&mut self) -> Vec<ConfigEdit> {
            self.edits.clone()
        }
    }

    fn edit(file_id: &'static str, key: &str, value: &str) -> ConfigEdit {
        ConfigEdit {
            file_id,
            key: key.to_string(),
            value: Some(value.to_string()),
            summary: format!("{} = {}", key, value),
        }
    }

    fn temp_dir() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("lxwc-aggregate-test-{unique}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn test_backend(rc_path: PathBuf, env_path: PathBuf) -> Backend {
        Backend {
            id: "test",
            groups: vec![Group {
                id: GROUP_OUTPUT,
                label_key: "wlr_module",
            }],
            files: vec![
                ConfigFile {
                    id: labwc::RC_XML_FILE,
                    path: rc_path,
                    format: &labwc::formats::RC_XML,
                    title_key: "rc_xml_changes",
                    needs_reload: true,
                },
                ConfigFile {
                    id: labwc::ENVIRONMENT_FILE,
                    path: env_path,
                    format: &labwc::formats::ENVIRONMENT,
                    title_key: "environment_changes",
                    needs_reload: false,
                },
            ],
            reload: Some(CommandSpec {
                program: "true".to_string(),
                args: Vec::new(),
            }),
        }
    }

    #[test]
    fn aggregates_rc_and_environment_writes_once() {
        let dir = temp_dir();
        let rc_path = dir.join("rc.xml");
        let env_path = dir.join("environment");
        let backend = test_backend(rc_path.clone(), env_path.clone());

        let mut modules: Vec<Box<dyn SettingsModule>> = vec![
            Box::new(FakeModule {
                id: "appearance",
                edits: vec![
                    edit(labwc::RC_XML_FILE, "/labwc_config/theme/cornerRadius", "12"),
                    edit(labwc::ENVIRONMENT_FILE, "XKB_DEFAULT_LAYOUT", "us"),
                ],
            }),
            Box::new(FakeModule {
                id: "behaviour",
                edits: vec![
                    edit(labwc::RC_XML_FILE, "/labwc_config/core/gap", "6"),
                    edit(
                        labwc::ENVIRONMENT_FILE,
                        "XKB_DEFAULT_OPTIONS",
                        "grp:alt_shift_toggle",
                    ),
                ],
            }),
        ];

        let (tasks, error) = collect_file_tasks(&mut modules, &backend);
        assert!(error.is_none());

        let rc_tasks = tasks
            .iter()
            .filter(|task| task.id == labwc::RC_XML_FILE)
            .collect::<Vec<_>>();
        assert_eq!(rc_tasks.len(), 1);
        assert_eq!(rc_tasks[0].change_count, 2);
        let rc_writes = rc_tasks[0]
            .effects
            .iter()
            .filter(|effect| matches!(effect, Effect::WriteFile { .. }))
            .collect::<Vec<_>>();
        assert_eq!(rc_writes.len(), 1);
        let Effect::WriteFile { contents, .. } = rc_writes[0] else {
            panic!("expected rc.xml write");
        };
        assert!(contents.contains("<cornerRadius>12</cornerRadius>"));
        assert!(contents.contains("<gap>6</gap>"));
        assert!(rc_tasks[0]
            .effects
            .iter()
            .any(|effect| matches!(effect, Effect::RunCommand { .. })));

        let env_tasks = tasks
            .iter()
            .filter(|task| task.id == labwc::ENVIRONMENT_FILE)
            .collect::<Vec<_>>();
        assert_eq!(env_tasks.len(), 1);
        assert_eq!(env_tasks[0].change_count, 2);
        assert!(env_tasks[0]
            .effects
            .iter()
            .all(|effect| matches!(effect, Effect::WriteFile { .. })));
        let Effect::WriteFile { contents, .. } = &env_tasks[0].effects[0] else {
            panic!("expected environment write");
        };
        assert!(contents.contains("XKB_DEFAULT_LAYOUT=us"));
        assert!(contents.contains("XKB_DEFAULT_OPTIONS=grp:alt_shift_toggle"));
        assert_eq!(
            contents
                .matches("# BEGIN lxwc-settings environment")
                .count(),
            1
        );
        assert_eq!(
            contents.matches("# END lxwc-settings environment").count(),
            1
        );
        assert!(!contents.contains("# added by lxwc-settings"));

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn unchanged_edits_do_not_create_tasks() {
        let dir = temp_dir();
        let rc_path = dir.join("rc.xml");
        let env_path = dir.join("environment");
        std::fs::write(&rc_path, "<labwc_config/>").unwrap();
        let backend = test_backend(rc_path, env_path);

        let mut modules: Vec<Box<dyn SettingsModule>> = vec![Box::new(FakeModule {
            id: "behaviour",
            edits: vec![edit(labwc::ENVIRONMENT_FILE, "XKB_DEFAULT_LAYOUT", "us")],
        })];

        let (tasks, error) = collect_file_tasks(&mut modules, &backend);
        assert!(error.is_none());
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, labwc::ENVIRONMENT_FILE);

        std::fs::remove_dir_all(dir).ok();
    }
}
