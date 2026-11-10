use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;

use crate::plugins::wlr::{Mode, OutputInfo, Outputs};

pub enum Request {
    Scan,
    ApplyMode { output: String, mode: Mode },
    RevertMode { output: String, mode: Mode },
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

pub fn spawn(
    outputs: Arc<dyn Outputs>,
    repaint: impl Fn() + Send + 'static,
) -> (Sender<Request>, Receiver<Event>) {
    let (request_tx, request_rx) = mpsc::channel::<Request>();
    let (event_tx, event_rx) = mpsc::channel::<Event>();

    let _ = std::thread::Builder::new()
        .name("lxwc-output-worker".to_string())
        .spawn(move || {
            while let Ok(request) = request_rx.recv() {
                let event = match request {
                    Request::Scan => Event::Outputs(outputs.scan()),
                    Request::ApplyMode { output, mode } => Event::ModeApplied {
                        result: outputs.apply(&output, &mode),
                        output,
                    },
                    Request::RevertMode { output, mode } => Event::ModeReverted {
                        result: outputs.apply(&output, &mode),
                        output,
                    },
                };
                let _ = event_tx.send(event);
                repaint();
            }
        });

    (request_tx, event_rx)
}
