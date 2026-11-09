use std::collections::HashMap;

use eframe::egui;

use crate::core::model::{ApplyReport, Task};
use crate::i18n::t;

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

pub struct ModuleCtx<'a> {
    pub failures: &'a HashMap<String, String>,
    pub status: &'a mut Option<StatusMessage>,
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
    fn pending(&mut self) -> Vec<Task>;
    fn on_applied(&mut self, report: &ApplyReport);
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut ModuleCtx);

    fn update(&mut self, _ui_ctx: &egui::Context, _ctx: &mut ModuleCtx) {}

    fn overlay(&mut self, _ui_ctx: &egui::Context, _ctx: &mut ModuleCtx) {}
}

pub mod display;
pub mod env;
pub mod keybind;
pub mod lxqt;
