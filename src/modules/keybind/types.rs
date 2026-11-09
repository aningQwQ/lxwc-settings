use crate::i18n::t;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeybindAction {
    pub name: String,
    pub command: String,
    pub attrs: Vec<(String, String)>,
}

pub fn known_action_label(action: &str) -> Option<&'static str> {
    let key = match action {
        "Execute" => "action_execute",
        "Reconfigure" => "action_reconfigure",
        "Close" => "action_close",
        "ToggleMaximize" => "action_toggle_maximize",
        "ToggleFullscreen" => "action_toggle_fullscreen",
        "ToggleShowDesktop" => "action_toggle_show_desktop",
        "ShowMenu" => "action_show_menu",
        "SnapToEdge" => "action_snap_to_edge",
        "SnapToRegion" => "action_snap_to_region",
        "NextWindow" => "action_next_window",
        "PreviousWindow" => "action_previous_window",
        "GoToDesktop" => "action_go_to_desktop",
        "Move" => "action_move",
        "MoveTo" => "action_move_to",
        "Resize" => "action_resize",
        "ResizeTo" => "action_resize_to",
        "Exit" => "action_exit",
        "Focus" => "action_focus",
        "Raise" => "action_raise",
        "Lower" => "action_lower",
        "Unfocus" => "action_unfocus",
        "Maximize" => "action_maximize",
        "Unmaximize" => "action_unmaximize",
        "Iconify" => "action_iconify",
        "Shade" => "action_shade",
        "Unshade" => "action_unshade",
        "ToggleShade" => "action_toggle_shade",
        "ToggleOmnipresent" => "action_toggle_omnipresent",
        "ToggleAlwaysOnTop" => "action_toggle_always_on_top",
        "ToggleAlwaysOnBottom" => "action_toggle_always_on_bottom",
        "AutoPlace" => "action_auto_place",
        "ToggleDecorations" => "action_toggle_decorations",
        "ToggleKeybinds" => "action_toggle_keybinds",
        "FocusOutput" => "action_focus_output",
        "MoveToOutput" => "action_move_to_output",
        "ResizeOutput" => "action_resize_output",
        _ => return None,
    };
    Some(t(key))
}

impl KeybindAction {
    pub fn summary(&self) -> String {
        let label = known_action_label(&self.name).unwrap_or(&self.name);
        if self.name == "Execute" && !self.command.is_empty() {
            return format!("{label}: {}", self.command);
        }
        if self.attrs.is_empty() {
            label.to_string()
        } else {
            let attrs = self
                .attrs
                .iter()
                .map(|(key, value)| format!("{key}=\"{value}\""))
                .collect::<Vec<_>>()
                .join(" ");
            format!("{label} {attrs}")
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeybindView {
    Edit,
    Existing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyPickTab {
    Letters,
    Digits,
    Function,
    Navigation,
    Symbols,
    Media,
}

#[derive(Clone, Debug)]
pub struct KeybindForm {
    pub key: String,
    pub action: String,
    pub command: String,
    pub attrs: String,
    pub advanced: bool,
    pub advanced_xml: String,
    pub editing_original: Option<String>,
    pub error: Option<String>,
}

impl Default for KeybindForm {
    fn default() -> Self {
        Self {
            key: String::new(),
            action: "Execute".to_string(),
            command: String::new(),
            attrs: String::new(),
            advanced: false,
            advanced_xml: String::new(),
            editing_original: None,
            error: None,
        }
    }
}

pub fn attrs_to_input(attrs: &[(String, String)]) -> String {
    attrs
        .iter()
        .map(|(key, value)| format!("{key}=\"{value}\""))
        .collect::<Vec<_>>()
        .join(" ")
}

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
