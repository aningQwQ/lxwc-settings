use eframe::egui;

use crate::backend::Backend;
use crate::modules::SettingsModule;
use crate::plugin::{Capability, Plugin};

#[cfg(feature = "labwc")]
pub mod labwc;
#[cfg(feature = "wlr")]
pub mod wlr;
#[cfg(feature = "wlr-ui")]
pub mod wlr_ui;

pub const PLUGINS: &[Plugin] = &[
    #[cfg(feature = "labwc")]
    labwc::PLUGIN,
    #[cfg(feature = "wlr")]
    wlr::PLUGIN,
    #[cfg(feature = "wlr-ui")]
    wlr_ui::PLUGIN,
];

pub struct Composition {
    pub backend: Backend,
    pub modules: Vec<Box<dyn SettingsModule>>,
}

/// Detect the backend plugin to use.
///
/// `LXWC_BACKEND` wins. Future backends hook their session socket variable in
/// here (`HYPRLAND_INSTANCE_SIGNATURE`, `SWAYSOCK`, `NIRI_SOCKET`,
/// `WAYFIRE_SOCKET`); the first backend plugin is the fallback.
pub fn detect() -> &'static str {
    if let Some(requested) = std::env::var_os("LXWC_BACKEND") {
        let requested = requested.to_string_lossy();
        if let Some(found) = PLUGINS
            .iter()
            .find(|plugin| plugin.backend.is_some() && plugin.id == requested)
        {
            return found.id;
        }
    }
    resolve()
        .into_iter()
        .find(|plugin| plugin.backend.is_some())
        .map(|plugin| plugin.id)
        .unwrap_or("none")
}

/// Enable plugins whose requirements are satisfied, then build the backend,
/// its pages and the groups contributed by page plugins.
pub fn compose(requested: &str, ctx: &egui::Context) -> Composition {
    let enabled = resolve();

    let backend_plugin = enabled
        .iter()
        .find(|plugin| plugin.backend.is_some() && plugin.id == requested)
        .or_else(|| enabled.iter().find(|plugin| plugin.backend.is_some()));

    let Some(backend_plugin) = backend_plugin else {
        return Composition {
            backend: Backend::empty(),
            modules: Vec::new(),
        };
    };

    let mut backend = (backend_plugin.backend.expect("checked above"))();
    for group in enabled.iter().flat_map(|plugin| plugin.groups) {
        if !backend
            .groups
            .iter()
            .any(|existing| existing.id == group.id)
        {
            backend.groups.push(*group);
        }
    }

    let mut modules = Vec::new();
    for plugin in &enabled {
        if let Some(pages) = plugin.pages {
            modules.extend(pages(ctx));
        }
    }

    Composition { backend, modules }
}

fn resolve() -> Vec<&'static Plugin> {
    let mut enabled: Vec<&'static Plugin> = Vec::new();
    let mut provided: Vec<Capability> = Vec::new();

    loop {
        let mut changed = false;
        for plugin in PLUGINS {
            if enabled.iter().any(|item| item.id == plugin.id) {
                continue;
            }
            if plugin.requires.iter().all(|need| provided.contains(need)) {
                enabled.push(plugin);
                provided.extend_from_slice(plugin.provides);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    enabled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requirements_resolve_in_dependency_order() {
        let enabled = resolve();
        let provided = enabled
            .iter()
            .flat_map(|plugin| plugin.provides.iter().copied())
            .collect::<Vec<_>>();
        for plugin in &enabled {
            for need in plugin.requires {
                assert!(
                    provided.contains(need),
                    "plugin {} enabled without provider for {}",
                    plugin.id,
                    need.as_str()
                );
            }
        }
    }

    #[test]
    fn every_requirement_has_a_provider_in_the_full_set() {
        let provided = PLUGINS
            .iter()
            .flat_map(|plugin| plugin.provides.iter().copied())
            .collect::<Vec<_>>();
        for plugin in PLUGINS {
            for need in plugin.requires {
                assert!(
                    provided.contains(need),
                    "plugin {} requires {} but no plugin provides it",
                    plugin.id,
                    need.as_str()
                );
            }
        }
    }

    #[test]
    fn no_capability_has_two_providers() {
        let mut seen: Vec<&'static str> = Vec::new();
        for plugin in PLUGINS {
            for capability in plugin.provides {
                assert!(
                    !seen.contains(&capability.as_str()),
                    "capability {} has more than one provider",
                    capability.as_str()
                );
                seen.push(capability.as_str());
            }
        }
    }

    #[cfg(not(feature = "labwc"))]
    #[test]
    fn shell_only_composition_has_no_pages() {
        let composition = compose("none", &eframe::egui::Context::default());
        assert!(composition.backend.files.is_empty());
        assert!(composition.backend.groups.is_empty());
        assert!(composition.modules.is_empty());
    }
}
