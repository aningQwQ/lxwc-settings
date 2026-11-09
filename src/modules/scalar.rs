use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::labwc::{environment, paths, rc_xml};
use crate::modules::{ConfigEdit, ConfigTarget};

#[derive(Clone, Copy)]
pub struct Spec {
    pub path: &'static str,
    pub default: &'static str,
}

pub struct ScalarState {
    path: PathBuf,
    raw: String,
    specs: &'static [Spec],
    baseline: BTreeMap<&'static str, String>,
    desired: BTreeMap<&'static str, String>,
    error: Option<String>,
}

impl ScalarState {
    pub fn new(specs: &'static [Spec]) -> Self {
        Self::with_path(paths::rc_xml(), specs)
    }

    pub fn with_path(path: PathBuf, specs: &'static [Spec]) -> Self {
        let mut state = Self {
            path,
            raw: String::new(),
            specs,
            baseline: BTreeMap::new(),
            desired: BTreeMap::new(),
            error: None,
        };
        state.reload(true);
        state
    }

    pub fn reload(&mut self, reset_desired: bool) {
        match rc_xml::load(&self.path) {
            Ok(file) => {
                self.raw = file.raw;
                self.baseline.clear();
                for spec in self.specs {
                    let value = rc_xml::get(&self.raw, spec.path)
                        .unwrap_or_else(|| spec.default.to_string());
                    self.baseline.insert(spec.path, value);
                }
                if reset_desired || self.desired.is_empty() {
                    self.desired = self.baseline.clone();
                } else {
                    for spec in self.specs {
                        let baseline = self.baseline[spec.path].clone();
                        self.desired.entry(spec.path).or_insert(baseline);
                    }
                }
                self.error = None;
            }
            Err(err) => self.error = Some(err),
        }
    }

    pub fn commit(&mut self) {
        self.reload(true);
    }

    pub fn sync_after_apply(&mut self) {
        self.reload(false);
    }

    pub fn raw(&self) -> &str {
        &self.raw
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn get(&self, path: &str) -> &str {
        self.desired
            .get(path)
            .map(String::as_str)
            .unwrap_or_default()
    }

    pub fn bool(&self, path: &str) -> bool {
        parse_bool(self.get(path))
    }

    pub fn int(&self, path: &str) -> i32 {
        self.get(path).trim().parse().unwrap_or_default()
    }

    pub fn float(&self, path: &str) -> f32 {
        self.get(path).trim().parse().unwrap_or_default()
    }

    pub fn set(&mut self, path: &str, value: impl Into<String>) {
        if let Some(slot) = self.desired.get_mut(path) {
            *slot = value.into();
        }
    }

    pub fn set_bool(&mut self, path: &str, value: bool) {
        self.set(path, if value { "yes" } else { "no" });
    }

    pub fn set_int(&mut self, path: &str, value: i32) {
        self.set(path, value.to_string());
    }

    pub fn set_float(&mut self, path: &str, value: f32) {
        self.set(path, format_float(value));
    }

    pub fn has_changes(&self) -> bool {
        self.specs
            .iter()
            .any(|spec| self.desired.get(spec.path) != self.baseline.get(spec.path))
    }

    pub fn edits(&self) -> Vec<ConfigEdit> {
        let mut edits = Vec::new();
        for spec in self.specs {
            let Some(value) = self.desired.get(spec.path) else {
                continue;
            };
            if self.baseline.get(spec.path) == Some(value) {
                continue;
            }
            edits.push(ConfigEdit {
                target: ConfigTarget::RcXml,
                key: spec.path.to_string(),
                value: Some(value.clone()),
                summary: format!("{} = {}", short_path(spec.path), value),
            });
        }
        edits
    }
}

pub struct EnvState {
    path: PathBuf,
    keys: &'static [&'static str],
    baseline: BTreeMap<String, Option<String>>,
    desired: BTreeMap<String, Option<String>>,
    error: Option<String>,
}

impl EnvState {
    pub fn new(keys: &'static [&'static str]) -> Self {
        Self::with_path(paths::environment(), keys)
    }

    pub fn with_path(path: PathBuf, keys: &'static [&'static str]) -> Self {
        let mut state = Self {
            path,
            keys,
            baseline: BTreeMap::new(),
            desired: BTreeMap::new(),
            error: None,
        };
        state.reload(true);
        state
    }

    pub fn reload(&mut self, reset_desired: bool) {
        match environment::load(&self.path) {
            Ok(file) => {
                let active = environment::parse_active(&file.raw);
                self.baseline.clear();
                for key in self.keys {
                    self.baseline
                        .insert((*key).to_string(), active.get(*key).cloned());
                }
                if reset_desired || self.desired.is_empty() {
                    self.desired = self.baseline.clone();
                } else {
                    for key in self.keys {
                        let baseline = self.baseline.get(*key).cloned().flatten();
                        self.desired.entry((*key).to_string()).or_insert(baseline);
                    }
                }
                self.error = None;
            }
            Err(err) => self.error = Some(err),
        }
    }

    pub fn commit(&mut self) {
        self.reload(true);
    }

    pub fn sync_after_apply(&mut self) {
        self.reload(false);
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.desired.get(key).cloned().flatten()
    }

    pub fn set(&mut self, key: &str, value: Option<String>) {
        if let Some(slot) = self.desired.get_mut(key) {
            *slot = value.and_then(|value| {
                if value.trim().is_empty() {
                    None
                } else {
                    Some(value)
                }
            });
        }
    }

    pub fn set_text(&mut self, key: &str, value: &str) {
        self.set(key, Some(value.to_string()));
    }

    pub fn set_int(&mut self, key: &str, value: i32) {
        self.set(key, Some(value.to_string()));
    }

    pub fn has_changes(&self) -> bool {
        self.keys
            .iter()
            .any(|key| self.desired.get(*key) != self.baseline.get(*key))
    }

    pub fn edits(&self) -> Vec<ConfigEdit> {
        let mut edits = Vec::new();
        for key in self.keys {
            let desired = self.desired.get(*key).cloned().flatten();
            if desired == self.baseline.get(*key).cloned().flatten() {
                continue;
            }
            let summary = match &desired {
                Some(value) => format!("{}={}", key, value),
                None => format!("-{}", key),
            };
            edits.push(ConfigEdit {
                target: ConfigTarget::Environment,
                key: (*key).to_string(),
                value: desired,
                summary,
            });
        }
        edits
    }
}

pub fn parse_bool(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "yes" | "true" | "on" | "1"
    )
}

pub fn format_float(value: f32) -> String {
    let mut text = format!("{value:.4}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}

fn short_path(path: &str) -> &str {
    path.strip_prefix("/labwc_config/").unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    const SPECS: &[Spec] = &[
        Spec {
            path: "/labwc_config/theme/cornerRadius",
            default: "8",
        },
        Spec {
            path: "/labwc_config/theme/dropShadows",
            default: "no",
        },
    ];

    fn temp_dir() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("lxwc-scalar-test-{unique}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn scalar_reads_defaults_and_queues_changes() {
        let dir = temp_dir();
        let path = dir.join("rc.xml");
        std::fs::write(
            &path,
            "<labwc_config>\n  <theme>\n    <cornerRadius>12</cornerRadius>\n  </theme>\n</labwc_config>\n",
        )
        .unwrap();

        let mut state = ScalarState::with_path(path, SPECS);
        assert_eq!(state.int("/labwc_config/theme/cornerRadius"), 12);
        assert!(!state.bool("/labwc_config/theme/dropShadows"));
        assert!(!state.has_changes());

        state.set_int("/labwc_config/theme/cornerRadius", 16);
        state.set_bool("/labwc_config/theme/dropShadows", true);
        let edits = state.edits();
        assert_eq!(edits.len(), 2);
        assert_eq!(edits[0].value.as_deref(), Some("16"));
        assert_eq!(edits[1].value.as_deref(), Some("yes"));

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn environment_state_unsets_empty_values() {
        let dir = temp_dir();
        let path = dir.join("environment");
        std::fs::write(&path, "XCURSOR_SIZE=32\nXCURSOR_THEME=Adwaita\n").unwrap();

        let mut state = EnvState::with_path(path, &["XCURSOR_SIZE", "XCURSOR_THEME"]);
        assert_eq!(state.get("XCURSOR_SIZE").as_deref(), Some("32"));
        state.set_text("XCURSOR_THEME", "");
        let edits = state.edits();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].key, "XCURSOR_THEME");
        assert_eq!(edits[0].value, None);

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn float_formatting_trims_trailing_zeroes() {
        assert_eq!(format_float(2.0), "2");
        assert_eq!(format_float(0.2), "0.2");
        assert_eq!(format_float(1.25), "1.25");
    }
}
