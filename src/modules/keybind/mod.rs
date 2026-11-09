mod file;
mod types;
mod ui;

use std::path::PathBuf;

use eframe::egui;

use crate::core::model::{ApplyReport, Effect, Task};
use crate::i18n::t;
use crate::modules::{Category, ModuleCtx, ModuleMeta, SettingsModule};
use crate::services::config;

use types::*;

const MODULE_ID: &str = "keybind";
const TASK_ID: &str = "keybind";

fn rc_path() -> PathBuf {
    std::env::var_os("LXWC_RC_XML")
        .map(PathBuf::from)
        .unwrap_or_else(|| config::config_dir().join("rc.xml"))
}

pub struct KeybindModule {
    path: PathBuf,
    raw: String,
    exists: bool,
    baseline: Vec<KeybindEntry>,
    form: KeybindForm,
    edits: Vec<KeybindEdit>,
    deletions: Vec<String>,
    filter: String,
    view: KeybindView,
    pick_tab: KeyPickTab,
    error: Option<String>,
}

impl KeybindModule {
    pub fn new() -> Self {
        Self::with_path(rc_path())
    }

    fn with_path(path: PathBuf) -> Self {
        let mut module = Self {
            path,
            raw: String::new(),
            exists: false,
            baseline: Vec::new(),
            form: KeybindForm::default(),
            edits: Vec::new(),
            deletions: Vec::new(),
            filter: String::new(),
            view: KeybindView::Existing,
            pick_tab: KeyPickTab::Letters,
            error: None,
        };
        module.reload_rc(true);
        module
    }

    fn reload_rc(&mut self, reset: bool) {
        match config::read_optional(&self.path) {
            Ok((raw, exists)) => {
                self.raw = raw;
                self.exists = exists;
                match file::parse(&self.raw) {
                    Ok(keybinds) => {
                        self.baseline = keybinds;
                        self.error = None;
                    }
                    Err(err) => {
                        self.baseline.clear();
                        self.error = Some(err);
                    }
                }
                if reset {
                    self.edits.clear();
                    self.deletions.clear();
                    self.form = KeybindForm::default();
                }
            }
            Err(err) => {
                self.error = Some(err);
            }
        }
    }

    fn reset_form(&mut self) {
        self.form = KeybindForm::default();
    }

    fn load_for_edit(&mut self, key: &str) {
        self.view = KeybindView::Edit;
        let normalized = file::normalize_key(key);

        let pending_edit = self
            .edits
            .iter()
            .find(|edit| {
                edit.original_key
                    .as_deref()
                    .map(file::normalize_key)
                    .as_deref()
                    == Some(normalized.as_str())
            })
            .cloned();
        if let Some(edit) = pending_edit {
            self.fill_form_from_edit(&edit);
            return;
        }

        let Some(entry) = self
            .baseline
            .iter()
            .find(|entry| file::normalize_key(&entry.key) == normalized)
        else {
            return;
        };

        let form = &mut self.form;
        form.editing_original = Some(entry.key.clone());
        form.key = entry.key.clone();
        form.advanced_xml = file::render_actions(&entry.actions);
        form.advanced = entry.actions.len() > 1;
        form.error = None;

        if let Some(action) = entry.actions.first() {
            form.action = action.name.clone();
            form.command = action.command.clone();
            form.attrs = attrs_to_input(&action.attrs);
        } else {
            form.action = "Execute".to_string();
            form.command.clear();
            form.attrs.clear();
            form.advanced = true;
        }
    }

    fn fill_form_from_edit(&mut self, edit: &KeybindEdit) {
        let form = &mut self.form;
        form.editing_original = edit.original_key.clone();
        form.key = edit.key.clone();
        form.advanced_xml = file::render_actions(&edit.actions);
        form.advanced = edit.actions.len() > 1;
        form.error = None;
        if let Some(action) = edit.actions.first() {
            form.action = action.name.clone();
            form.command = action.command.clone();
            form.attrs = attrs_to_input(&action.attrs);
        } else {
            form.action = "Execute".to_string();
            form.command.clear();
            form.attrs.clear();
            form.advanced = true;
        }
    }

    fn submit(&mut self) -> Result<String, String> {
        self.form.error = None;
        if !self.exists {
            return Err(crate::trf!(
                "was_not_found_key_bindings_cannot_be_written",
                self.path.display()
            ));
        }
        let form = self.form.clone();

        let key = file::validate_combo(&form.key)?;

        let actions = if form.advanced {
            file::parse_actions_fragment(&form.advanced_xml)?
        } else {
            file::validate_action_name(&form.action)?;
            let attrs = file::parse_attrs(&form.attrs)?;
            let command = form.command.trim().to_string();
            if form.action.trim() == "Execute" && command.is_empty() {
                return Err(t("the_execute_action_requires_a_command").to_string());
            }
            vec![KeybindAction {
                name: form.action.trim().to_string(),
                command,
                attrs,
            }]
        };

        let keybind_attrs = form
            .editing_original
            .as_ref()
            .and_then(|original| {
                let normalized = file::normalize_key(original);
                self.baseline
                    .iter()
                    .find(|entry| file::normalize_key(&entry.key) == normalized)
                    .map(|entry| entry.attrs.clone())
            })
            .unwrap_or_default();

        let edit = KeybindEdit {
            original_key: form.editing_original.clone(),
            key: key.clone(),
            attrs: keybind_attrs,
            actions,
        };

        let replace_index = edit.original_key.as_ref().and_then(|original| {
            self.edits
                .iter()
                .position(|existing| existing.original_key.as_deref() == Some(original.as_str()))
        });

        let target = file::normalize_key(&edit.key);
        for (index, other) in self.edits.iter().enumerate() {
            if Some(index) == replace_index {
                continue;
            }
            let other_target = file::normalize_key(&other.key);
            let other_original = other.original_key.as_deref().map(file::normalize_key);
            if other_target == target || other_original.as_deref() == Some(target.as_str()) {
                return Err(t("the_target_key_conflicts_with_another_queued_change").to_string());
            }
        }

        if let Some(index) = replace_index {
            self.edits.remove(index);
        }

        let original_norm = edit.original_key.as_deref().map(file::normalize_key);
        self.deletions.retain(|key| {
            let normalized = file::normalize_key(key);
            normalized != target && Some(normalized) != original_norm
        });

        let status = self.edit_status(&edit);
        let message = if status == EditStatus::NoChange {
            t("no_change_from_the_current_state_not_queued").to_string()
        } else {
            let message = crate::trf!("queued", edit.key, edit.summary());
            self.edits.push(edit);
            message
        };

        self.form.key.clear();
        self.form.editing_original = None;
        self.form.error = None;
        Ok(message)
    }

    fn edit_status(&self, edit: &KeybindEdit) -> EditStatus {
        let target = file::normalize_key(&edit.key);
        let target_entry = self
            .baseline
            .iter()
            .find(|entry| file::normalize_key(&entry.key) == target);

        if let Some(original) = &edit.original_key {
            let original_norm = file::normalize_key(original);
            let original_entry = self
                .baseline
                .iter()
                .find(|entry| file::normalize_key(&entry.key) == original_norm);
            if original_norm == target {
                match original_entry {
                    Some(entry) if entry.actions == edit.actions && entry.attrs == edit.attrs => {
                        EditStatus::NoChange
                    }
                    Some(_) => EditStatus::Update,
                    None => EditStatus::New,
                }
            } else {
                EditStatus::Rename
            }
        } else if let Some(entry) = target_entry {
            if entry.actions == edit.actions && entry.attrs == edit.attrs {
                EditStatus::NoChange
            } else {
                EditStatus::Overwrite
            }
        } else {
            EditStatus::New
        }
    }

    fn discard_edit(&mut self, edit: &KeybindEdit) {
        self.edits.retain(|existing| existing != edit);
        if self.form.editing_original == edit.original_key {
            self.form.editing_original = None;
        }
    }

    fn delete(&mut self, key: &str) -> String {
        let normalized = file::normalize_key(key);
        self.edits.retain(|edit| {
            let target = file::normalize_key(&edit.key);
            let original = edit.original_key.as_deref().map(file::normalize_key);
            target != normalized && original.as_deref() != Some(normalized.as_str())
        });
        self.deletions
            .retain(|item| file::normalize_key(item) != normalized);
        if self
            .form
            .editing_original
            .as_deref()
            .map(file::normalize_key)
            .as_deref()
            == Some(normalized.as_str())
        {
            self.form.editing_original = None;
        }
        if let Some(entry) = self
            .baseline
            .iter()
            .find(|entry| file::normalize_key(&entry.key) == normalized)
        {
            self.deletions.push(entry.key.clone());
        }
        crate::trf!("queued_for_deletion", key)
    }

    fn undo_delete(&mut self, key: &str) {
        let normalized = file::normalize_key(key);
        self.deletions
            .retain(|item| file::normalize_key(item) != normalized);
    }

    fn is_pending_deletion(&self, key: &str) -> bool {
        let normalized = file::normalize_key(key);
        self.deletions
            .iter()
            .any(|item| file::normalize_key(item) == normalized)
    }

    fn push_unique(keys: &mut Vec<String>, key: &str) {
        if !keys.iter().any(|existing| existing == key) {
            keys.push(key.to_string());
        }
    }
}

impl SettingsModule for KeybindModule {
    fn meta(&self) -> ModuleMeta {
        ModuleMeta {
            id: MODULE_ID,
            title_key: "global_shortcuts",
            subtitle_key:
                "type_any_keysym_combination_and_action_and_edit_rc_xml_keybindings_directly",
            category: Category::Labwc,
        }
    }

    fn refresh(&mut self) {
        self.reload_rc(true);
    }

    fn pending(&mut self) -> Vec<Task> {
        let mut tasks = Vec::new();

        if self.exists {
            let mut additions = Vec::new();
            let mut remove_keys = Vec::new();
            let mut changes = Vec::new();

            for deletion in &self.deletions {
                let normalized = file::normalize_key(deletion);
                let Some(entry) = self
                    .baseline
                    .iter()
                    .find(|entry| file::normalize_key(&entry.key) == normalized)
                else {
                    continue;
                };
                changes.push(format!("-{}", entry.key));
                Self::push_unique(&mut remove_keys, &entry.key);
            }

            for edit in &self.edits {
                if self.edit_status(edit) == EditStatus::NoChange {
                    continue;
                }

                let target = file::normalize_key(&edit.key);
                let target_entry = self
                    .baseline
                    .iter()
                    .find(|entry| file::normalize_key(&entry.key) == target);

                if let Some(original) = &edit.original_key {
                    let original_norm = file::normalize_key(original);
                    if let Some(entry) = self
                        .baseline
                        .iter()
                        .find(|entry| file::normalize_key(&entry.key) == original_norm)
                    {
                        if original_norm != target {
                            changes.push(format!("{} → {}", entry.key, edit.key));
                            Self::push_unique(&mut remove_keys, &entry.key);
                        } else {
                            changes.push(edit.key.clone());
                        }
                    } else {
                        changes.push(edit.key.clone());
                    }
                } else {
                    changes.push(format!("+{}", edit.key));
                }

                if target_entry.is_some() {
                    Self::push_unique(&mut remove_keys, &edit.key);
                }
                additions.push(file::Addition {
                    key: edit.key.clone(),
                    attrs: edit.attrs.clone(),
                    actions: edit.actions.clone(),
                });
            }

            if !changes.is_empty() {
                match file::patch(&self.raw, &additions, &remove_keys) {
                    Ok(patched) => {
                        self.error = None;
                        tasks.push(Task {
                            id: TASK_ID.to_string(),
                            module_id: MODULE_ID,
                            title: crate::trf!("keybind_changes", changes.len()),
                            detail: changes.join("; "),
                            change_count: changes.len(),
                            effects: vec![
                                Effect::WriteFile {
                                    path: self.path.clone(),
                                    contents: patched,
                                    expected: if self.exists {
                                        Some(self.raw.clone())
                                    } else {
                                        None
                                    },
                                },
                                Effect::RunCommand {
                                    program: "sh".to_string(),
                                    args: vec![
                                        "-c".to_string(),
                                        "labwc --reconfigure || pkill -HUP -x labwc".to_string(),
                                    ],
                                    best_effort: true,
                                },
                            ],
                        });
                    }
                    Err(err) => {
                        self.error = Some(err);
                    }
                }
            }
        }

        tasks
    }

    fn on_applied(&mut self, report: &ApplyReport) {
        let keybind_ok = report.task_succeeded(MODULE_ID, TASK_ID);
        let keybind_failed = report.task_failed(MODULE_ID, TASK_ID);
        if keybind_ok || keybind_failed {
            self.reload_rc(false);
            if keybind_ok {
                self.edits.clear();
                self.deletions.clear();
                self.form.editing_original = None;
                self.form.key.clear();
                self.form.error = None;
            }
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut ModuleCtx) {
        ui::show(self, ui, ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::model::TaskResult;
    use std::time::{SystemTime, UNIX_EPOCH};

    const RC: &str = r#"<?xml version="1.0"?>
<labwc_config>
  <keyboard>
    <keybind key="W-p">
      <action name="Execute" command="pcmanfm-qt" />
    </keybind>
    <keybind key="A-F2">
      <action name="Execute">
        <command>lxqt-runner</command>
      </action>
    </keybind>
  </keyboard>
</labwc_config>
"#;

    fn temp_rc() -> (std::path::PathBuf, std::path::PathBuf) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("lxwc-keybind-test-{unique}"));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("rc.xml");
        std::fs::write(&path, RC).unwrap();
        (dir, path)
    }

    fn edit(key: &str) -> KeybindEdit {
        KeybindEdit {
            original_key: None,
            key: key.to_string(),
            attrs: Vec::new(),
            actions: vec![KeybindAction {
                name: "Execute".to_string(),
                command: "qterminal".to_string(),
                attrs: Vec::new(),
            }],
        }
    }

    fn report(ok: bool) -> ApplyReport {
        ApplyReport {
            results: vec![TaskResult {
                id: TASK_ID.to_string(),
                module_id: MODULE_ID,
                title: "keybind".to_string(),
                change_count: 1,
                ok,
                error: if ok {
                    None
                } else {
                    Some("write failed".to_string())
                },
            }],
            warnings: Vec::new(),
            elapsed_ms: 0,
        }
    }

    #[test]
    fn pending_write_and_successful_apply() {
        let (dir, path) = temp_rc();
        let mut module = KeybindModule::with_path(path);
        module.edits.push(edit("W-t"));

        let tasks = module.pending();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].change_count, 1);
        assert!(matches!(tasks[0].effects[1], Effect::RunCommand { .. }));

        let Effect::WriteFile { path, contents, .. } = &tasks[0].effects[0] else {
            panic!("expected file write");
        };
        assert!(contents.contains("key=\"W-t\""));
        std::fs::write(path, contents).unwrap();

        module.on_applied(&report(true));
        assert!(module.edits.is_empty());
        assert!(module.pending().is_empty());

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn failed_apply_keeps_edits() {
        let (dir, path) = temp_rc();
        let mut module = KeybindModule::with_path(path);
        module.edits.push(edit("W-t"));

        module.on_applied(&report(false));
        assert_eq!(module.edits.len(), 1);
        assert_eq!(module.pending().len(), 1);

        std::fs::remove_dir_all(dir).ok();
    }
}
