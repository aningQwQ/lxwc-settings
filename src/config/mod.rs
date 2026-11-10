use std::path::PathBuf;

/// A single key/value intent produced by a page.
///
/// `file_id` names the [`ConfigFile`] the edit belongs to, so pages never
/// mention concrete paths and adding a backend file never requires a new enum
/// variant.
#[derive(Clone, Debug)]
pub struct ConfigEdit {
    pub file_id: &'static str,
    pub key: String,
    pub value: Option<String>,
    pub summary: String,
}

/// Result of a page-specific patch step such as keybinds or pointer curves.
#[derive(Clone, Debug)]
pub struct PatchOutcome {
    pub contents: String,
    pub summary: String,
    pub count: usize,
}

pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
}

/// Outcome of applying a batch of [`ConfigEdit`]s to one file.
///
/// `changed` is aligned with the input edit list so the caller can attribute
/// each applied edit back to the page that produced it.
pub struct Applied {
    pub contents: String,
    pub changed: Vec<bool>,
}

/// Interprets one configuration file format.
///
/// Implementations own parsing and serialization; the application layer only
/// moves text. A backend supplies the concrete format for each [`ConfigFile`].
pub trait ConfigFormat: Send + Sync {
    fn apply(&self, raw: &str, edits: &[ConfigEdit]) -> Result<Applied, String>;
}

/// A configuration file the active backend owns.
pub struct ConfigFile {
    pub id: &'static str,
    pub path: PathBuf,
    pub format: &'static dyn ConfigFormat,
    /// Translation key for the task title; receives one `{}` for the change count.
    pub title_key: &'static str,
    /// Whether the backend must reload after this file changes.
    pub needs_reload: bool,
}
