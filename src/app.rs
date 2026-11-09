use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use eframe::egui;

use crate::core::model::{ApplyReport, Effect, Task};
use crate::i18n::{self, t, Lang};
use crate::labwc::{environment, paths, rc_xml, reconfigure};
use crate::modules::{
    self, ConfigTarget, ModuleCtx, SettingsModule, StatusMessage, ENVIRONMENT_TASK_ID,
    RC_XML_TASK_ID,
};
use crate::services::fs;
use crate::services::worker::{WorkerEvent, WorkerHandle, WorkerRequest};
use crate::ui::{sidebar, status_bar, theme};

pub struct App {
    worker: WorkerHandle,
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

        let mut modules: Vec<Box<dyn SettingsModule>> = vec![
            Box::new(modules::display::DisplayModule::new(ctx)),
            Box::new(modules::appearance::AppearanceModule::new()),
            Box::new(modules::behaviour::BehaviourModule::new()),
            Box::new(modules::mouse::MouseModule::new()),
            Box::new(modules::keyboard::KeyboardModule::new()),
            Box::new(modules::touchscreen::TouchscreenModule::new()),
            Box::new(modules::keybind::KeybindModule::new()),
            Box::new(modules::labwc_display::LabwcDisplayModule::new()),
            Box::new(modules::env::EnvModule::new()),
        ];
        for module in &mut modules {
            module.refresh();
        }

        let worker = WorkerHandle::spawn(ctx.clone());
        Self {
            worker,
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

        let (file_tasks, error) =
            collect_file_tasks(&mut self.modules, &paths::rc_xml(), &paths::environment());
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

fn collect_file_tasks(
    modules: &mut [Box<dyn SettingsModule>],
    rc_path: &Path,
    env_path: &Path,
) -> (Vec<Task>, Option<String>) {
    let mut tasks = Vec::new();
    let mut errors = Vec::new();

    let (mut rc_raw, rc_exists) = match fs::read_optional(rc_path) {
        Ok(value) => value,
        Err(err) => {
            errors.push(err);
            (String::new(), false)
        }
    };
    let (mut env_raw, env_exists) = match fs::read_optional(env_path) {
        Ok(value) => value,
        Err(err) => {
            errors.push(err);
            (String::new(), false)
        }
    };
    let rc_expected = rc_exists.then(|| rc_raw.clone());
    let env_expected = env_exists.then(|| env_raw.clone());

    let mut rc_summaries = Vec::new();
    let mut env_summaries = Vec::new();
    let mut rc_counts: HashMap<&'static str, usize> = HashMap::new();
    let mut env_counts: HashMap<&'static str, usize> = HashMap::new();
    let mut env_sets = BTreeMap::new();
    let mut env_unsets = Vec::new();
    let mut active_env = environment::parse_active(&env_raw);

    for module in modules.iter_mut() {
        let module_id = module.meta().id;
        for edit in module.config_edits() {
            match edit.target {
                ConfigTarget::RcXml => {
                    let current = rc_xml::get(&rc_raw, &edit.key);
                    let unchanged = match &edit.value {
                        Some(value) => current.as_deref() == Some(value.as_str()),
                        None => current.is_none(),
                    };
                    if unchanged {
                        continue;
                    }
                    let result = match &edit.value {
                        Some(value) => rc_xml::set(&rc_raw, &edit.key, value),
                        None => rc_xml::remove(&rc_raw, &edit.key),
                    };
                    match result {
                        Ok(next) => {
                            rc_raw = next;
                            rc_summaries.push(edit.summary);
                            *rc_counts.entry(module_id).or_default() += 1;
                        }
                        Err(err) => errors.push(err),
                    }
                }
                ConfigTarget::Environment => {
                    let current = active_env.get(&edit.key).cloned();
                    let unchanged = match &edit.value {
                        Some(value) => current.as_deref() == Some(value.as_str()),
                        None => current.is_none(),
                    };
                    if unchanged {
                        continue;
                    }
                    match edit.value {
                        Some(value) => {
                            env_sets.insert(edit.key.clone(), value.clone());
                            active_env.insert(edit.key.clone(), value);
                        }
                        None => {
                            if !env_unsets.iter().any(|key| key == &edit.key) {
                                env_unsets.push(edit.key.clone());
                            }
                            active_env.remove(&edit.key);
                        }
                    }
                    env_summaries.push(edit.summary);
                    *env_counts.entry(module_id).or_default() += 1;
                }
            }
        }
    }

    if !env_sets.is_empty() || !env_unsets.is_empty() {
        env_raw = environment::patch(&env_raw, &env_sets, &env_unsets);
    }

    for module in modules.iter_mut() {
        let module_id = module.meta().id;

        match module.rc_xml_patch(&rc_raw) {
            Ok(Some(patch)) => {
                if patch.count > 0 {
                    rc_raw = patch.contents;
                    rc_summaries.push(patch.summary);
                    *rc_counts.entry(module_id).or_default() += patch.count;
                }
            }
            Ok(None) => {}
            Err(err) => errors.push(err),
        }

        match module.environment_patch(&env_raw) {
            Ok(Some(patch)) => {
                if patch.count > 0 {
                    env_raw = patch.contents;
                    env_summaries.push(patch.summary);
                    *env_counts.entry(module_id).or_default() += patch.count;
                }
            }
            Ok(None) => {}
            Err(err) => errors.push(err),
        }
    }

    let rc_count: usize = rc_counts.values().sum();
    let env_count: usize = env_counts.values().sum();
    let mut rc_attribution = rc_counts.into_iter().collect::<Vec<_>>();
    let mut env_attribution = env_counts.into_iter().collect::<Vec<_>>();
    rc_attribution.sort_by_key(|(module_id, _)| *module_id);
    env_attribution.sort_by_key(|(module_id, _)| *module_id);

    if rc_count > 0 {
        let command = reconfigure::command();
        tasks.push(Task {
            id: RC_XML_TASK_ID.to_string(),
            module_id: RC_XML_TASK_ID,
            title: crate::trf!("rc_xml_changes", rc_count),
            detail: rc_summaries.join("; "),
            change_count: rc_count,
            attribution: rc_attribution,
            effects: vec![
                Effect::WriteFile {
                    path: rc_path.to_path_buf(),
                    contents: rc_raw,
                    expected: rc_expected,
                },
                Effect::RunCommand {
                    program: command.program,
                    args: command.args,
                    best_effort: true,
                },
            ],
        });
    }

    if env_count > 0 {
        tasks.push(Task {
            id: ENVIRONMENT_TASK_ID.to_string(),
            module_id: ENVIRONMENT_TASK_ID,
            title: crate::trf!("environment_changes", env_count),
            detail: env_summaries.join("; "),
            change_count: env_count,
            attribution: env_attribution,
            effects: vec![Effect::WriteFile {
                path: env_path.to_path_buf(),
                contents: env_raw,
                expected: env_expected,
            }],
        });
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
    use crate::modules::{ConfigEdit, ConfigTarget, ModuleMeta};
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
                category: crate::modules::Category::Labwc,
            }
        }

        fn refresh(&mut self) {}

        fn on_applied(&mut self, _report: &ApplyReport) {}

        fn ui(&mut self, _ui: &mut egui::Ui, _ctx: &mut ModuleCtx) {}

        fn config_edits(&mut self) -> Vec<ConfigEdit> {
            self.edits.clone()
        }
    }

    fn edit(target: ConfigTarget, key: &str, value: &str) -> ConfigEdit {
        ConfigEdit {
            target,
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

    #[test]
    fn aggregates_rc_and_environment_writes_once() {
        let dir = temp_dir();
        let rc_path = dir.join("rc.xml");
        let env_path = dir.join("environment");

        let mut modules: Vec<Box<dyn SettingsModule>> = vec![
            Box::new(FakeModule {
                id: "appearance",
                edits: vec![
                    edit(
                        ConfigTarget::RcXml,
                        "/labwc_config/theme/cornerRadius",
                        "12",
                    ),
                    edit(ConfigTarget::Environment, "XKB_DEFAULT_LAYOUT", "us"),
                ],
            }),
            Box::new(FakeModule {
                id: "behaviour",
                edits: vec![
                    edit(ConfigTarget::RcXml, "/labwc_config/core/gap", "6"),
                    edit(
                        ConfigTarget::Environment,
                        "XKB_DEFAULT_OPTIONS",
                        "grp:alt_shift_toggle",
                    ),
                ],
            }),
        ];

        let (tasks, error) = collect_file_tasks(&mut modules, &rc_path, &env_path);
        assert!(error.is_none());

        let rc_tasks = tasks
            .iter()
            .filter(|task| task.id == RC_XML_TASK_ID)
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

        let env_tasks = tasks
            .iter()
            .filter(|task| task.id == ENVIRONMENT_TASK_ID)
            .collect::<Vec<_>>();
        assert_eq!(env_tasks.len(), 1);
        assert_eq!(env_tasks[0].change_count, 2);
        let Effect::WriteFile { contents, .. } = &env_tasks[0].effects[0] else {
            panic!("expected environment write");
        };
        assert!(contents.contains("XKB_DEFAULT_LAYOUT=us"));
        assert!(contents.contains("XKB_DEFAULT_OPTIONS=grp:alt_shift_toggle"));
        assert_eq!(contents.matches("# added by lxwc-settings").count(), 1);

        std::fs::remove_dir_all(dir).ok();
    }
}
