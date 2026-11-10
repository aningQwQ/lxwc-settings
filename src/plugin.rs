use std::sync::Arc;

use eframe::egui;

use crate::backend::{Backend, Deps, Group};
use crate::core::outputs::Outputs;
use crate::modules::SettingsModule;

pub mod capability {
    pub const BACKEND: &str = "backend";
    pub const OUTPUTS: &str = "outputs";
}

pub type OutputsFactory = fn() -> Arc<dyn Outputs>;
pub type PagesFactory = fn(&egui::Context, &Deps) -> Vec<Box<dyn SettingsModule>>;

/// Static description of one optional unit.
///
/// Role lives here, not in the directory name: a plugin can provide a
/// capability, contribute pages, or be the backend.
pub struct Plugin {
    pub id: &'static str,
    pub provides: &'static [&'static str],
    pub requires: &'static [&'static str],
    /// Sections this plugin wants in the sidebar. The host merges them into
    /// the backend's groups and keeps backend order when ids collide.
    pub groups: &'static [Group],
    pub backend: Option<fn() -> Backend>,
    pub pages: Option<PagesFactory>,
    pub outputs: Option<OutputsFactory>,
}
