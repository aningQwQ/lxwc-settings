use std::process::Command as ProcessCommand;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Instant;

use eframe::egui;

use super::config;
use super::keybinds;
use super::model::{ApplyReport, Baseline, OutputInfo, Task, TaskAction, TaskResult};
use super::output;

pub enum WorkerRequest {
    Refresh,
    ScanOutputs,
    ApplyTasks(Vec<Task>),
    ApplyMode { output: String, spec: String },
    RevertMode { output: String, spec: String },
}

pub enum WorkerEvent {
    Baseline(Result<Baseline, String>),
    Outputs(Result<Vec<OutputInfo>, String>),
    Applied {
        report: ApplyReport,
        baseline: Result<Baseline, String>,
    },
    ModeApplied {
        output: String,
        result: Result<(), String>,
    },
    ModeReverted {
        output: String,
        result: Result<(), String>,
    },
}

pub struct WorkerHandle {
    tx: Sender<WorkerRequest>,
    rx: Receiver<WorkerEvent>,
}

impl WorkerHandle {
    pub fn spawn(ctx: egui::Context) -> Self {
        let (request_tx, request_rx) = mpsc::channel::<WorkerRequest>();
        let (event_tx, event_rx) = mpsc::channel::<WorkerEvent>();

        let _ = std::thread::Builder::new()
            .name("lxwc-worker".to_string())
            .spawn(move || {
                while let Ok(request) = request_rx.recv() {
                    let event = handle(request);
                    let _ = event_tx.send(event);
                    ctx.request_repaint();
                }
            });

        Self {
            tx: request_tx,
            rx: event_rx,
        }
    }

    pub fn send(&self, request: WorkerRequest) {
        let _ = self.tx.send(request);
    }

    pub fn try_recv(&self) -> Option<WorkerEvent> {
        self.rx.try_recv().ok()
    }
}

fn handle(request: WorkerRequest) -> WorkerEvent {
    match request {
        WorkerRequest::Refresh => WorkerEvent::Baseline(config::load_baseline()),
        WorkerRequest::ScanOutputs => WorkerEvent::Outputs(output::scan()),
        WorkerRequest::ApplyTasks(tasks) => {
            let report = apply_tasks(&tasks);
            let baseline = config::load_baseline();
            WorkerEvent::Applied { report, baseline }
        }
        WorkerRequest::ApplyMode { output, spec } => WorkerEvent::ModeApplied {
            result: run_wlr_randr(&output, &spec),
            output,
        },
        WorkerRequest::RevertMode { output, spec } => WorkerEvent::ModeReverted {
            result: run_wlr_randr(&output, &spec),
            output,
        },
    }
}

fn apply_tasks(tasks: &[Task]) -> ApplyReport {
    let started = Instant::now();
    let mut report = ApplyReport::default();

    let file_tasks: Vec<&Task> = tasks
        .iter()
        .filter(|task| matches!(task.action, TaskAction::WriteFile { .. }))
        .collect();

    for task in &file_tasks {
        if let TaskAction::WriteFile {
            path,
            contents,
            expected,
        } = &task.action
        {
            let outcome = config::write_file_if_unchanged(path, expected.as_deref(), contents);
            report.results.push(result_for(task, &outcome));
            if outcome.is_ok() {
                report.files.push(path.display().to_string());
            }
        }
    }

    let keybind_tasks: Vec<&Task> = tasks
        .iter()
        .filter(|task| {
            matches!(
                task.action,
                TaskAction::KeybindAdd { .. } | TaskAction::KeybindRemove { .. }
            )
        })
        .collect();

    if !keybind_tasks.is_empty() {
        let path = config::rc_path();
        let outcome = (|| -> Result<(), String> {
            let raw = std::fs::read_to_string(&path)
                .map_err(|err| crate::trf!("failed_to_read", path.display(), err))?;
            let mut additions = Vec::new();
            let mut remove_keys: Vec<String> = Vec::new();

            for task in &keybind_tasks {
                match &task.action {
                    TaskAction::KeybindAdd {
                        key,
                        attrs,
                        actions,
                        replace,
                    } => {
                        additions.push(keybinds::Addition {
                            key: key.clone(),
                            attrs: attrs.clone(),
                            actions: actions.clone(),
                        });
                        if *replace && !remove_keys.contains(key) {
                            remove_keys.push(key.clone());
                        }
                    }
                    TaskAction::KeybindRemove { key } => {
                        if !remove_keys.contains(key) {
                            remove_keys.push(key.clone());
                        }
                    }
                    _ => {}
                }
            }

            let patched = keybinds::patch(&raw, &additions, &remove_keys)?;
            config::atomic_write(&path, &patched)
        })();

        for task in &keybind_tasks {
            report.results.push(result_for(task, &outcome));
        }
        if outcome.is_ok() {
            report.files.push(path.display().to_string());
            report.reconfigure_error = reconfigure_labwc().err();
        }
    }

    report.elapsed_ms = started.elapsed().as_millis();
    report
}

fn result_for(task: &Task, outcome: &Result<(), String>) -> TaskResult {
    TaskResult {
        id: task.id.clone(),
        module: task.module,
        title: task.title.clone(),
        change_count: task.change_count,
        ok: outcome.is_ok(),
        error: outcome.as_ref().err().cloned(),
    }
}

fn reconfigure_labwc() -> Result<(), String> {
    if let Ok(output) = ProcessCommand::new("labwc").arg("--reconfigure").output() {
        if output.status.success() {
            return Ok(());
        }
    }

    let output = ProcessCommand::new("pkill")
        .args(["-HUP", "-x", "labwc"])
        .output()
        .map_err(|err| crate::trf!("failed_to_invoke_pkill", err))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(crate::trf!(
            "pkill_hup_labwc_failed",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

fn run_wlr_randr(output: &str, spec: &str) -> Result<(), String> {
    let result = ProcessCommand::new("wlr-randr")
        .args(["--output", output, "--mode", spec])
        .output()
        .map_err(|err| crate::trf!("failed_to_run_wlr_randr", err))?;

    if result.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&result.stderr);
    let stdout = String::from_utf8_lossy(&result.stdout);
    let detail = if stderr.trim().is_empty() {
        stdout.trim()
    } else {
        stderr.trim()
    };
    Err(crate::trf!("wlr_randr_failed_to_switch_modes", detail))
}
