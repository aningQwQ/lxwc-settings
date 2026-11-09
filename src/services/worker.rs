use std::process::Command as ProcessCommand;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Instant;

use eframe::egui;

use super::fs;

use crate::core::model::{ApplyReport, Effect, Task, TaskResult};

pub enum WorkerRequest {
    ApplyTasks(Vec<Task>),
}

pub enum WorkerEvent {
    Applied { report: ApplyReport },
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
        WorkerRequest::ApplyTasks(tasks) => WorkerEvent::Applied {
            report: apply_tasks(&tasks),
        },
    }
}

fn apply_tasks(tasks: &[Task]) -> ApplyReport {
    let started = Instant::now();
    let mut report = ApplyReport::default();

    for task in tasks {
        let mut warnings = Vec::new();
        let mut error = None;

        for effect in &task.effects {
            match effect {
                Effect::WriteFile {
                    path,
                    contents,
                    expected,
                } => {
                    if let Err(err) =
                        fs::write_file_if_unchanged(path, expected.as_deref(), contents)
                    {
                        error = Some(err);
                        break;
                    }
                }
                Effect::RunCommand {
                    program,
                    args,
                    best_effort,
                } => {
                    if let Err(err) = run_command(program, args) {
                        if *best_effort {
                            warnings.push(err);
                        } else {
                            error = Some(err);
                            break;
                        }
                    }
                }
            }
        }

        let ok = error.is_none();
        report.results.push(TaskResult {
            id: task.id.clone(),
            module_id: task.module_id,
            title: task.title.clone(),
            change_count: task.change_count,
            ok,
            error,
        });
        report.warnings.extend(warnings);
    }

    report.elapsed_ms = started.elapsed().as_millis();
    report
}

fn run_command(program: &str, args: &[String]) -> Result<(), String> {
    let output = ProcessCommand::new(program)
        .args(args)
        .output()
        .map_err(|err| crate::trf!("failed_to_run_command", program, err))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let detail = if stderr.trim().is_empty() {
        stdout.trim()
    } else {
        stderr.trim()
    };
    Err(crate::trf!("command_failed", program, detail))
}
