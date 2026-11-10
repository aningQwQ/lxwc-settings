use eframe::egui;

use crate::backend::Backend;
use crate::modules::SettingsModule;

pub mod labwc;

/// Known backend ids. A backend is selectable through `LXWC_BACKEND` or, once
/// there is more than one, autodetected from its session socket.
pub const BACKENDS: &[&str] = &[labwc::ID];

type BackendFactory = fn() -> Backend;
type PageBuilder = fn(&egui::Context) -> Vec<Box<dyn SettingsModule>>;

const FACTORIES: &[(&str, BackendFactory)] = &[(labwc::ID, labwc::backend)];
const PAGE_BUILDERS: &[(&str, PageBuilder)] = &[(labwc::ID, labwc::pages)];

/// Detect the running compositor.
///
/// `LXWC_BACKEND` always wins so users and tests can force a backend. Future
/// backends hook their session socket variable in here, for example
/// `HYPRLAND_INSTANCE_SIGNATURE`, `SWAYSOCK`, `NIRI_SOCKET` or
/// `WAYFIRE_SOCKET`. Labwc exposes no socket, so it is the fallback.
pub fn detect() -> &'static str {
    if let Some(requested) = std::env::var_os("LXWC_BACKEND") {
        let requested = requested.to_string_lossy();
        if let Some(found) = BACKENDS.iter().find(|id| **id == requested) {
            return found;
        }
    }
    labwc::ID
}

pub fn create(id: &str) -> Backend {
    for (name, factory) in FACTORIES {
        if *name == id {
            return factory();
        }
    }
    labwc::backend()
}

pub fn modules(id: &str, ctx: &egui::Context) -> Vec<Box<dyn SettingsModule>> {
    for (name, builder) in PAGE_BUILDERS {
        if *name == id {
            return builder(ctx);
        }
    }
    labwc::pages(ctx)
}
