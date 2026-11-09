use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModuleKind {
    Display,
    Keybind,
    Environment,
}

impl ModuleKind {
    pub const ALL: [ModuleKind; 3] = [
        ModuleKind::Display,
        ModuleKind::Keybind,
        ModuleKind::Environment,
    ];

    pub fn title(self) -> &'static str {
        match self {
            ModuleKind::Display => "显示与输出",
            ModuleKind::Keybind => "快捷键与桌面集成",
            ModuleKind::Environment => "环境变量魔改",
        }
    }

    pub fn category(self) -> &'static str {
        match self {
            ModuleKind::Display => "硬件与显示",
            ModuleKind::Keybind => "桌面集成",
            ModuleKind::Environment => "高级魔改",
        }
    }

    pub fn subtitle(self) -> &'static str {
        match self {
            ModuleKind::Display => "直接通过 wlr-randr 管理输出，带 15 秒防变砖回滚。",
            ModuleKind::Keybind => "手动填写任意 keysym 组合与动作，直接编辑 rc.xml 快捷键。",
            ModuleKind::Environment => "修改 labwc environment，为串流与兼容性加开关。",
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
            0 => "无动作".to_string(),
            1 => self.actions[0].summary(),
            count => format!("{}（{} 个动作）", self.actions[0].summary(), count),
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
            0 => "无动作".to_string(),
            1 => self.actions[0].summary(),
            count => format!("{}（{} 个动作）", self.actions[0].summary(), count),
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
            EditStatus::New => "新增",
            EditStatus::Overwrite => "覆盖",
            EditStatus::Update => "更新",
            EditStatus::Rename => "改键",
            EditStatus::NoChange => "无变化",
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
    pub label: &'static str,
    pub help: &'static str,
}

pub const MANAGED_ENV: [ManagedEnv; 2] = [
    ManagedEnv {
        key: "WLR_DRM_NO_MODIFIERS",
        label: "强制线性显存",
        help: "规避部分串流、虚拟显示在 GPU 修饰符下的黑屏和花屏。",
    },
    ManagedEnv {
        key: "WLR_NO_HARDWARE_CURSORS",
        label: "禁用硬件光标",
        help: "改用软件光标，修复远程串流时鼠标消失或残影。",
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
