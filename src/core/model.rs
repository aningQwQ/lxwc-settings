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
    pub const ALL: [Category; 3] = [
        Category::Wlr,
        Category::Lxqt,
        Category::Environment,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Category::Wlr => t("WLR 模块", "WLR Module"),
            Category::Lxqt => t("LXQt 模块", "LXQt Module"),
            Category::Environment => t("环境/系统模块", "Environment / System"),
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
            ModuleKind::Display => t("显示与输出", "Display & Output"),
            ModuleKind::Keybind => t("快捷键绑定", "Key Bindings"),
            ModuleKind::Environment => t("环境变量魔改", "Environment Tweaks"),
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
            ModuleKind::Display => t(
                "直接通过 wlr-randr 管理输出，带 15 秒防变砖回滚。",
                "Manage outputs directly via wlr-randr, with a 15-second rollback watchdog.",
            ),
            ModuleKind::Keybind => t(
                "手动填写任意 keysym 组合与动作，直接编辑 rc.xml 快捷键。",
                "Type any keysym combination and action, and edit rc.xml keybindings directly.",
            ),
            ModuleKind::Environment => t(
                "修改 labwc environment，为串流与兼容性加开关。",
                "Edit labwc environment for streaming and compatibility switches.",
            ),
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
            0 => t("无动作", "No action").to_string(),
            1 => self.actions[0].summary(),
            count => crate::trf!(
                "{}（{} 个动作）",
                "{} ({} actions)",
                self.actions[0].summary(),
                count
            ),
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
            0 => t("无动作", "No action").to_string(),
            1 => self.actions[0].summary(),
            count => crate::trf!(
                "{}（{} 个动作）",
                "{} ({} actions)",
                self.actions[0].summary(),
                count
            ),
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
            EditStatus::New => t("新增", "New"),
            EditStatus::Overwrite => t("覆盖", "Overwrite"),
            EditStatus::Update => t("更新", "Update"),
            EditStatus::Rename => t("改键", "Rename"),
            EditStatus::NoChange => t("无变化", "No change"),
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
    EnvSet { key: String, value: String },
    EnvUnset { key: String },
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
    pub label_zh: &'static str,
    pub label_en: &'static str,
    pub help_zh: &'static str,
    pub help_en: &'static str,
}

impl ManagedEnv {
    pub fn label(&self) -> &'static str {
        t(self.label_zh, self.label_en)
    }

    pub fn help(&self) -> &'static str {
        t(self.help_zh, self.help_en)
    }
}

pub const MANAGED_ENV: [ManagedEnv; 2] = [
    ManagedEnv {
        key: "WLR_DRM_NO_MODIFIERS",
        label_zh: "强制线性显存",
        label_en: "Force linear buffers",
        help_zh: "规避部分串流、虚拟显示在 GPU 修饰符下的黑屏和花屏。",
        help_en: "Avoid black screens or corruption with some streaming and virtual outputs caused by GPU modifiers.",
    },
    ManagedEnv {
        key: "WLR_NO_HARDWARE_CURSORS",
        label_zh: "禁用硬件光标",
        label_en: "Disable hardware cursors",
        help_zh: "改用软件光标，修复远程串流时鼠标消失或残影。",
        help_en: "Use a software cursor to fix disappearing or ghosting cursors during remote streaming.",
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
