use std::collections::HashMap;

use eframe::egui;

use crate::core::model::{ApplyReport, Task};

pub trait SettingsModule {
    fn refresh(&mut self);
    fn pending(&self) -> Vec<Task>;
    fn on_applied(&mut self, report: &ApplyReport);
    fn ui(&mut self, ui: &mut egui::Ui, failures: &HashMap<String, String>);
}

pub mod display;
pub mod env;
pub mod keybind;
