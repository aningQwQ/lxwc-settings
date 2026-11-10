use std::collections::HashMap;

use eframe::egui;

use crate::core::model::{ApplyReport, Task};

pub use crate::config::{ConfigEdit, PatchOutcome};

/// Module id of the page that persists a default display for the backend.
pub const DISPLAY_PERSIST_MODULE_ID: &str = "display-persist";

#[derive(Clone, Debug)]
pub struct StatusMessage {
    pub text: String,
    pub error: bool,
}

impl StatusMessage {
    pub fn info(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            error: false,
        }
    }

    pub fn error(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            error: true,
        }
    }
}

pub struct ModuleRequest {
    pub target: &'static str,
    pub payload: ModulePayload,
}

pub enum ModulePayload {
    DisplayDefault { output: String, spec: String },
}

pub struct ModuleCtx<'a> {
    pub failures: &'a HashMap<String, String>,
    pub status: &'a mut Option<StatusMessage>,
    pub requests: &'a mut Vec<ModuleRequest>,
}

impl ModuleCtx<'_> {
    pub fn info(&mut self, text: impl Into<String>) {
        *self.status = Some(StatusMessage::info(text));
    }

    pub fn error(&mut self, text: impl Into<String>) {
        *self.status = Some(StatusMessage::error(text));
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ModuleMeta {
    pub id: &'static str,
    pub title_key: &'static str,
    pub subtitle_key: &'static str,
    /// Sidebar group id, resolved against the active backend's groups.
    pub group: &'static str,
}

/// Contract between the application shell and a settings page.
///
/// Pages describe intent. They never know file paths, formats or commands:
/// edits name a backend file id and patches return text.
pub trait SettingsModule {
    fn meta(&self) -> ModuleMeta;
    fn refresh(&mut self);
    fn pending(&mut self) -> Vec<Task> {
        Vec::new()
    }
    fn on_applied(&mut self, report: &ApplyReport);
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut ModuleCtx);

    fn is_valid(&self) -> bool {
        true
    }

    fn config_edits(&mut self) -> Vec<ConfigEdit> {
        Vec::new()
    }

    /// Page-specific patch step for one backend file.
    ///
    /// Used where a plain key/value edit cannot express the change, such as
    /// keybind entries or pointer curves.
    fn patch(
        &mut self,
        _file_id: &'static str,
        _raw: &str,
    ) -> Result<Option<PatchOutcome>, String> {
        Ok(None)
    }

    fn on_message(&mut self, _payload: &ModulePayload) {}

    fn update(&mut self, _ui_ctx: &egui::Context, _ctx: &mut ModuleCtx) {}

    fn overlay(&mut self, _ui_ctx: &egui::Context, _ctx: &mut ModuleCtx) {}
}
