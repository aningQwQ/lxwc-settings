use eframe::egui;

#[cfg(feature = "wlr-ui")]
use crate::backend::GROUP_OUTPUT;
use crate::backend::{Backend, Deps, Group};
use crate::config::ConfigFile;
use crate::modules::SettingsModule;
use crate::plugin::{capability, Plugin};

pub mod accel;
pub mod actions;
#[cfg(feature = "wlr")]
pub mod autostart;
pub mod environment;
pub mod formats;
pub mod keycombo;
pub mod layouts;
pub mod lxqt;
pub mod pages;
pub mod paths;
pub mod rc_xml;
pub mod reconfigure;
pub mod themes;

pub const ID: &str = "labwc";
pub const GROUP: &str = "labwc";

/// File ids used by rc.xml / environment edits and apply reports.
pub const RC_XML_FILE: &str = "rc-xml";
pub const ENVIRONMENT_FILE: &str = "environment";

pub const PLUGIN: Plugin = Plugin {
    id: ID,
    provides: &[capability::BACKEND],
    requires: &[capability::OUTPUTS],
    groups: &[],
    backend: Some(backend),
    pages: Some(pages),
    outputs: None,
};

pub fn backend() -> Backend {
    Backend {
        groups: vec![
            #[cfg(feature = "wlr-ui")]
            Group {
                id: GROUP_OUTPUT,
                label_key: "wlr_module",
            },
            Group {
                id: GROUP,
                label_key: "labwc_module",
            },
        ],
        files: vec![
            ConfigFile {
                id: RC_XML_FILE,
                path: paths::rc_xml(),
                format: &formats::RC_XML,
                title_key: "rc_xml_changes",
                needs_reload: true,
            },
            ConfigFile {
                id: ENVIRONMENT_FILE,
                path: paths::environment(),
                format: &formats::ENVIRONMENT,
                title_key: "environment_changes",
                needs_reload: false,
            },
        ],
        reload: Some(reconfigure::command()),
    }
}

pub fn pages(_ctx: &egui::Context, deps: &Deps) -> Vec<Box<dyn SettingsModule>> {
    let Some(outputs) = deps.outputs.clone() else {
        return Vec::new();
    };

    vec![
        Box::new(pages::appearance::AppearanceModule::new()),
        Box::new(pages::behaviour::BehaviourModule::new()),
        Box::new(pages::mouse::MouseModule::new()),
        Box::new(pages::keyboard::KeyboardModule::new()),
        Box::new(pages::touchscreen::TouchscreenModule::new()),
        Box::new(pages::keybind::KeybindModule::new()),
        #[cfg(feature = "wlr")]
        Box::new(pages::display_persist::DisplayPersistModule::new(outputs)),
        Box::new(pages::env::EnvModule::new()),
    ]
}
