use eframe::egui;

use crate::backend::{Backend, Group};
use crate::modules::SettingsModule;

/// Capability id declared by the provider that owns the contract.
///
/// Providers mint their own ids with a namespace (`wlr.outputs`), and consumers
/// must name the exact contract they were written against. The registry only
/// matches strings; the actual API lives in the provider module, so a consumer
/// imports that provider directly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Capability(&'static str);

impl Capability {
    pub const fn new(name: &'static str) -> Self {
        Self(name)
    }

    #[cfg(test)]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

pub type PagesFactory = fn(&egui::Context) -> Vec<Box<dyn SettingsModule>>;

/// Static description of one optional unit.
///
/// Role lives here, not in the directory name: a plugin can provide a
/// capability, contribute pages, or be the backend.
pub struct Plugin {
    pub id: &'static str,
    pub provides: &'static [Capability],
    pub requires: &'static [Capability],
    /// Sections this plugin wants in the sidebar. The host merges them into
    /// the backend's groups and keeps backend order when ids collide.
    pub groups: &'static [Group],
    pub backend: Option<fn() -> Backend>,
    pub pages: Option<PagesFactory>,
}
