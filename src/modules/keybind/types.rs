use crate::i18n::t;
use crate::labwc::rc_xml::KeybindAction;

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
