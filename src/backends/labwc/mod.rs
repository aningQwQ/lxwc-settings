use eframe::egui;

use crate::backend::{Backend, Group, GROUP_OUTPUT};
use crate::config::ConfigFile;
use crate::modules::SettingsModule;

pub mod accel;
pub mod actions;
pub mod autostart;
pub mod environment;
pub mod formats;
pub mod keycombo;
pub mod layouts;
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

pub fn backend() -> Backend {
    Backend {
        id: ID,
        groups: vec![
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

pub fn pages(ctx: &egui::Context) -> Vec<Box<dyn SettingsModule>> {
    vec![
        Box::new(crate::pages::display::DisplayModule::new(ctx)),
        Box::new(pages::appearance::AppearanceModule::new()),
        Box::new(pages::behaviour::BehaviourModule::new()),
        Box::new(pages::mouse::MouseModule::new()),
        Box::new(pages::keyboard::KeyboardModule::new()),
        Box::new(pages::touchscreen::TouchscreenModule::new()),
        Box::new(pages::keybind::KeybindModule::new()),
        Box::new(pages::display_persist::DisplayPersistModule::new()),
        Box::new(pages::env::EnvModule::new()),
    ]
}
