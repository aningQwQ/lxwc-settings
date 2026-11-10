use crate::config::{CommandSpec, ConfigFile};

/// Group id used by pages that do not belong to a single compositor.
#[cfg(feature = "wlr-ui")]
pub const GROUP_OUTPUT: &str = "output";

/// A sidebar section. Pages pick their section through `ModuleMeta::group`.
#[derive(Clone, Copy, Debug)]
pub struct Group {
    pub id: &'static str,
    pub label_key: &'static str,
}

/// Everything the application needs to know about the running compositor.
///
/// Pages are constructed separately by `backends::modules` so the application
/// can hold mutable page state while reading immutable backend metadata.
pub struct Backend {
    pub groups: Vec<Group>,
    pub files: Vec<ConfigFile>,
    pub reload: Option<CommandSpec>,
}

impl Backend {
    pub fn empty() -> Self {
        Self {
            groups: Vec::new(),
            files: Vec::new(),
            reload: None,
        }
    }
}
