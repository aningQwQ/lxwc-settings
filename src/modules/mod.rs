use std::collections::HashMap;

use eframe::egui;

use crate::core::model::{ApplyReport, Task};
use crate::i18n::t;

pub const RC_XML_TASK_ID: &str = "rc-xml";
pub const ENVIRONMENT_TASK_ID: &str = "environment";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigTarget {
    RcXml,
    Environment,
}

#[derive(Clone, Debug)]
pub struct ConfigEdit {
    pub target: ConfigTarget,
    pub key: String,
    pub value: Option<String>,
    pub summary: String,
}

#[derive(Clone, Debug)]
pub struct PatchOutcome {
    pub contents: String,
    pub summary: String,
    pub count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Wlr,
    Labwc,
    Environment,
}

impl Category {
    pub const ALL: [Category; 3] = [Category::Wlr, Category::Labwc, Category::Environment];

    pub fn label(self) -> &'static str {
        match self {
            Category::Wlr => t("wlr_module"),
            Category::Labwc => t("labwc_module"),
            Category::Environment => t("environment_system"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ModuleMeta {
    pub id: &'static str,
    pub title_key: &'static str,
    pub subtitle_key: &'static str,
    pub category: Category,
}

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

    fn rc_xml_patch(&mut self, _raw: &str) -> Result<Option<PatchOutcome>, String> {
        Ok(None)
    }

    fn environment_patch(&mut self, _raw: &str) -> Result<Option<PatchOutcome>, String> {
        Ok(None)
    }

    fn on_message(&mut self, _payload: &ModulePayload) {}

    fn update(&mut self, _ui_ctx: &egui::Context, _ctx: &mut ModuleCtx) {}

    fn overlay(&mut self, _ui_ctx: &egui::Context, _ctx: &mut ModuleCtx) {}
}

mod scalar;

pub mod appearance;
pub mod behaviour;
pub mod display;
pub mod env;
pub mod keybind;
pub mod keyboard;
pub mod labwc_display;
pub mod mouse;
pub mod touchscreen;
