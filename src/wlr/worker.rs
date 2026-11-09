use std::sync::mpsc::{self, Receiver, Sender};

use super::{scan, set_mode, OutputInfo};

pub enum Request {
    Scan,
    ApplyMode { output: String, spec: String },
    RevertMode { output: String, spec: String },
}

pub enum Event {
    Outputs(Result<Vec<OutputInfo>, String>),
    ModeApplied {
        output: String,
        result: Result<(), String>,
    },
    ModeReverted {
        output: String,
        result: Result<(), String>,
    },
}

pub fn spawn(repaint: impl Fn() + Send + 'static) -> (Sender<Request>, Receiver<Event>) {
    let (request_tx, request_rx) = mpsc::channel::<Request>();
    let (event_tx, event_rx) = mpsc::channel::<Event>();

    let _ = std::thread::Builder::new()
        .name("lxwc-wlr-worker".to_string())
        .spawn(move || {
            while let Ok(request) = request_rx.recv() {
                let event = match request {
                    Request::Scan => Event::Outputs(scan()),
                    Request::ApplyMode { output, spec } => Event::ModeApplied {
                        output: output.clone(),
                        result: set_mode(&output, &spec),
                    },
                    Request::RevertMode { output, spec } => Event::ModeReverted {
                        output: output.clone(),
                        result: set_mode(&output, &spec),
                    },
                };
                let _ = event_tx.send(event);
                repaint();
            }
        });

    (request_tx, event_rx)
}
