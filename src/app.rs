use std::collections::HashMap;

use eframe::egui;

use crate::core::model::{ApplyReport, Task};
use crate::i18n::{self, t, Lang};
use crate::modules::{self, ModuleCtx, SettingsModule, StatusMessage};
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
        if self.pending.is_empty() || self.applying {
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

        {
            let mut tasks = Vec::new();
            for module in &mut self.modules {
                tasks.extend(module.pending());
            }
            self.pending = tasks;
        }

        sidebar::show(self, ctx);
        status_bar::show(self, ctx);

        let current = self.current;
        egui::CentralPanel::default().show(ctx, |ui| {
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
