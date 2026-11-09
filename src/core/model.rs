use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::i18n::t;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModuleKind {
    Display,
    Keybind,
    Environment,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Wlr,
    Lxqt,
    Environment,
}

impl Category {
    pub const ALL: [Category; 3] = [Category::Wlr, Category::Lxqt, Category::Environment];

    pub fn label(self) -> &'static str {
        match self {
            Category::Wlr => t("wlr_module"),
            Category::Lxqt => t("lxqt_module"),
            Category::Environment => t("environment_system"),
        }
    }
}

impl ModuleKind {
    pub const ALL: [ModuleKind; 3] = [
        ModuleKind::Display,
        ModuleKind::Keybind,
        ModuleKind::Environment,
    ];

    pub fn title(self) -> &'static str {
        match self {
            ModuleKind::Display => t("display_output"),
            ModuleKind::Keybind => t("key_bindings"),
            ModuleKind::Environment => t("environment_tweaks"),
        }
    }

    pub fn category(self) -> Category {
        match self {
            ModuleKind::Display => Category::Wlr,
            ModuleKind::Keybind => Category::Lxqt,
            ModuleKind::Environment => Category::Environment,
        }
    }

    pub fn subtitle(self) -> &'static str {
        match self {
            ModuleKind::Display => {
                t("manage_outputs_directly_via_wlr_randr_with_a_15_second_rollback_watchdog")
            }
            ModuleKind::Keybind => {
                t("type_any_keysym_combination_and_action_and_edit_rc_xml_keybindings_directly")
            }
            ModuleKind::Environment => {
                t("edit_labwc_environment_for_streaming_and_compatibility_switches")
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Mode {
    pub width: u32,
    pub height: u32,
    pub refresh: f64,
}

impl Mode {
    pub fn spec(&self) -> String {
        format!("{}x{}@{:.6}Hz", self.width, self.height, self.refresh)
    }

    pub fn label(&self) -> String {
        format!("{} × {} @ {:.2} Hz", self.width, self.height, self.refresh)
    }

    pub fn same_mode(&self, other: &Mode) -> bool {
        self.width == other.width
            && self.height == other.height
            && (self.refresh - other.refresh).abs() < 0.001
    }
}

#[derive(Clone, Debug)]
pub struct OutputInfo {
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub current: Option<Mode>,
    pub modes: Vec<Mode>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeybindAction {
    pub name: String,
    pub command: String,
    pub attrs: Vec<(String, String)>,
}

impl KeybindAction {
    pub fn summary(&self) -> String {
        if self.name == "Execute" && !self.command.is_empty() {
            return format!("Execute: {}", self.command);
        }
        if self.attrs.is_empty() {
            self.name.clone()
        } else {
            let attrs = self
                .attrs
                .iter()
                .map(|(key, value)| format!("{key}=\"{value}\""))
                .collect::<Vec<_>>()
                .join(" ");
            format!("{} {}", self.name, attrs)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeybindEntry {
    pub key: String,
    pub attrs: Vec<(String, String)>,
    pub actions: Vec<KeybindAction>,
}

impl KeybindEntry {
    pub fn summary(&self) -> String {
        match self.actions.len() {
            0 => t("no_action").to_string(),
            1 => self.actions[0].summary(),
            count => crate::trf!("actions", self.actions[0].summary(), count),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeybindEdit {
    pub original_key: Option<String>,
    pub key: String,
    pub attrs: Vec<(String, String)>,
    pub actions: Vec<KeybindAction>,
}

impl KeybindEdit {
    pub fn summary(&self) -> String {
        match self.actions.len() {
            0 => t("no_action").to_string(),
            1 => self.actions[0].summary(),
            count => crate::trf!("actions", self.actions[0].summary(), count),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditStatus {
    New,
    Overwrite,
    Update,
    Rename,
    NoChange,
}

impl EditStatus {
    pub fn label(self) -> &'static str {
        match self {
            EditStatus::New => t("new"),
            EditStatus::Overwrite => t("overwrite"),
            EditStatus::Update => t("update"),
            EditStatus::Rename => t("rename"),
            EditStatus::NoChange => t("no_change"),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Baseline {
    pub rc_path: PathBuf,
    pub env_path: PathBuf,
    pub rc_exists: bool,
    pub env_exists: bool,
    pub environment: BTreeMap<String, String>,
    pub keybinds: Vec<KeybindEntry>,
}

#[derive(Clone, Debug)]
pub enum TaskAction {
    EnvSet {
        key: String,
        value: String,
    },
    EnvUnset {
        key: String,
    },
    KeybindAdd {
        key: String,
        attrs: Vec<(String, String)>,
        actions: Vec<KeybindAction>,
        replace: bool,
    },
    KeybindRemove {
        key: String,
    },
}

#[derive(Clone, Debug)]
pub struct Task {
    pub id: String,
    pub module: ModuleKind,
    pub title: String,
    pub detail: String,
    pub action: TaskAction,
}

#[derive(Clone, Debug)]
pub struct TaskResult {
    pub id: String,
    pub module: ModuleKind,
    pub title: String,
    pub ok: bool,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct ApplyReport {
    pub results: Vec<TaskResult>,
    pub files: Vec<String>,
    pub reconfigure_error: Option<String>,
    pub elapsed_ms: u128,
}

impl ApplyReport {
    pub fn success_count(&self) -> usize {
        self.results.iter().filter(|result| result.ok).count()
    }

    pub fn failure_count(&self) -> usize {
        self.results.iter().filter(|result| !result.ok).count()
    }
}

pub struct ManagedEnv {
    pub key: &'static str,
    pub label: &'static str,
    pub help: &'static str,
}

impl ManagedEnv {
    pub fn label(&self) -> &'static str {
        t(self.label)
    }

    pub fn help(&self) -> &'static str {
        t(self.help)
    }
}

pub const MANAGED_ENV: [ManagedEnv; 2] = [
    ManagedEnv {
        key: "WLR_DRM_NO_MODIFIERS",
        label: "force_linear_buffers",
        help: "avoid_black_screens_or_corruption_with_some_streaming_and_virtual_outputs_caused_by_gpu_modifiers",
    },
    ManagedEnv {
        key: "WLR_NO_HARDWARE_CURSORS",
        label: "disable_hardware_cursors",
        help: "use_a_software_cursor_to_fix_disappearing_or_ghosting_cursors_during_remote_streaming",
    },
];

pub const ACTION_SUGGESTIONS: &[&str] = &[
    "Execute",
    "Reconfigure",
    "Close",
    "ToggleMaximize",
    "ToggleFullscreen",
    "ToggleShowDesktop",
    "ShowMenu",
    "SnapToEdge",
    "NextWindow",
    "PreviousWindow",
    "GoToDesktop",
    "Move",
    "Resize",
    "Exit",
];
