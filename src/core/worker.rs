use std::collections::BTreeMap;
use std::process::Command as ProcessCommand;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Instant;

use eframe::egui;

use super::config;
use super::env_file;
use super::keybinds;
use super::model::{ApplyReport, Baseline, OutputInfo, Task, TaskAction, TaskResult};
use super::output;

pub enum WorkerRequest {
    Refresh,
    ScanOutputs,
    ApplyTasks(Vec<Task>),
    ApplyMode {
        output: String,
        spec: String,
    },
    RevertMode {
        output: String,
        spec: String,
    },
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

    let env_tasks: Vec<&Task> = tasks
        .iter()
        .filter(|task| {
            matches!(
                task.action,
                TaskAction::EnvSet { .. } | TaskAction::EnvUnset { .. }
            )
        })
        .collect();

    if !env_tasks.is_empty() {
        let mut sets = BTreeMap::new();
        let mut unsets = Vec::new();
        for task in &env_tasks {
            match &task.action {
                TaskAction::EnvSet { key, value } => {
                    sets.insert(key.clone(), value.clone());
                }
                TaskAction::EnvUnset { key } => unsets.push(key.clone()),
                _ => {}
            }
        }

        let path = config::environment_path();
        let outcome = (|| -> Result<(), String> {
            let (raw, _) = config::read_optional(&path)?;
            let patched = env_file::patch(&raw, &sets, &unsets);
            config::atomic_write(&path, &patched)
        })();

        for task in &env_tasks {
            report.results.push(result_for(task, &outcome));
        }
        if outcome.is_ok() {
            report.files.push(path.display().to_string());
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
                .map_err(|err| crate::trf!("读取 {} 失败：{err}", "Failed to read {}: {err}", path.display()))?;
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
        ok: outcome.is_ok(),
        error: outcome.as_ref().err().cloned(),
    }
}

fn reconfigure_labwc() -> Result<(), String> {
    if let Ok(output) = ProcessCommand::new("labwc")
        .arg("--reconfigure")
        .output()
    {
        if output.status.success() {
            return Ok(());
        }
    }

    let output = ProcessCommand::new("pkill")
        .args(["-HUP", "-x", "labwc"])
        .output()
        .map_err(|err| crate::trf!("无法调用 pkill：{err}", "Failed to invoke pkill: {err}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(crate::trf!(
            "pkill -HUP labwc 失败：{}",
            "pkill -HUP labwc failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

fn run_wlr_randr(output: &str, spec: &str) -> Result<(), String> {
    let result = ProcessCommand::new("wlr-randr")
        .args(["--output", output, "--mode", spec])
        .output()
        .map_err(|err| crate::trf!("无法执行 wlr-randr：{err}", "Failed to run wlr-randr: {err}"))?;

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
    Err(crate::trf!(
        "wlr-randr 切换模式失败：{detail}",
        "wlr-randr failed to switch modes: {detail}"
    ))
}
