mod file;

use std::collections::BTreeMap;
use std::path::PathBuf;

use eframe::egui::{self, RichText};

use crate::core::config;
use crate::core::model::{ApplyReport, Effect, Task};
use crate::i18n::t;
use crate::modules::{Category, ModuleCtx, ModuleMeta, SettingsModule};
use crate::ui::theme;

const ENV_TASK_ID: &str = "env";

fn environment_path() -> PathBuf {
    std::env::var_os("LXWC_ENVIRONMENT")
        .map(PathBuf::from)
        .unwrap_or_else(|| config::config_dir().join("environment"))
}

struct ManagedEnv {
    key: &'static str,
    label: &'static str,
    help: &'static str,
}

const MANAGED_ENV: [ManagedEnv; 2] = [
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

pub struct EnvModule {
    path: PathBuf,
    raw: String,
    exists: bool,
    baseline: BTreeMap<String, String>,
    desired: BTreeMap<&'static str, Option<String>>,
    error: Option<String>,
}

impl EnvModule {
    pub fn new() -> Self {
        Self::with_path(environment_path())
    }

    fn with_path(path: PathBuf) -> Self {
        Self {
            path,
            raw: String::new(),
            exists: false,
            baseline: BTreeMap::new(),
            desired: BTreeMap::new(),
            error: None,
        }
    }

    fn reload(&mut self, reset_desired: bool) {
        match config::read_optional(&self.path) {
            Ok((raw, exists)) => {
                self.raw = raw;
                self.exists = exists;
                self.baseline = file::parse_active(&self.raw);
                if reset_desired || self.desired.is_empty() {
                    self.desired.clear();
                    for item in MANAGED_ENV {
                        self.desired
                            .insert(item.key, self.baseline.get(item.key).cloned());
                    }
                } else {
                    for item in MANAGED_ENV {
                        self.desired
                            .entry(item.key)
                            .or_insert_with(|| self.baseline.get(item.key).cloned());
                    }
                }
                self.error = None;
            }
            Err(err) => {
                self.error = Some(err);
            }
        }
    }

    fn pending_task(&self) -> Option<Task> {
        let mut sets = BTreeMap::new();
        let mut unsets = Vec::new();
        let mut changes = Vec::new();

        for item in MANAGED_ENV {
            let desired = self.desired.get(item.key).cloned().flatten();
            let current = self.baseline.get(item.key).cloned();
            if desired == current {
                continue;
            }
            match desired {
                Some(value) => {
                    changes.push(format!("{}={}", item.key, value));
                    sets.insert(item.key.to_string(), value);
                }
                None => {
                    changes.push(format!("-{}", item.key));
                    unsets.push(item.key.to_string());
                }
            }
        }

        if changes.is_empty() {
            return None;
        }

        Some(Task {
            id: ENV_TASK_ID.to_string(),
            module_id: "env",
            title: crate::trf!("environment_changes", changes.len()),
            detail: changes.join(", "),
            change_count: changes.len(),
            effects: vec![Effect::WriteFile {
                path: self.path.clone(),
                contents: file::patch(&self.raw, &sets, &unsets),
                expected: if self.exists {
                    Some(self.raw.clone())
                } else {
                    None
                },
            }],
        })
    }
}

impl SettingsModule for EnvModule {
    fn meta(&self) -> ModuleMeta {
        ModuleMeta {
            id: "env",
            title_key: "environment_tweaks",
            subtitle_key: "edit_labwc_environment_for_streaming_and_compatibility_switches",
            category: Category::Environment,
        }
    }

    fn refresh(&mut self) {
        self.reload(true);
    }

    fn pending(&mut self) -> Vec<Task> {
        self.pending_task().into_iter().collect()
    }

    fn on_applied(&mut self, _report: &ApplyReport) {
        self.reload(false);
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut ModuleCtx) {
        ui.label(t(
            "these_switches_are_written_to_config_labwc_environment_and_only_take_effect_the_next_time_the_labwc_session_starts",
        ));

        if let Some(err) = &self.error {
            ui.colored_label(theme::DANGER, RichText::new(err).small());
        } else if !self.exists {
            ui.label(
                RichText::new(crate::trf!(
                    "does_not_exist_and_will_be_created_when_applying",
                    self.path.display()
                ))
                .small()
                .weak(),
            );
        }

        if self.pending_task().is_some() {
            if let Some(err) = ctx.failures.get(ENV_TASK_ID) {
                ui.colored_label(theme::DANGER, RichText::new(err).small());
            }
        }

        ui.add_space(10.0);

        for item in MANAGED_ENV {
            let mut checked = self.desired.get(item.key).cloned().flatten().is_some();
            let current_value = self.baseline.get(item.key).cloned();
            let mut changed = false;

            ui.group(|ui| {
                ui.horizontal(|ui| {
                    if ui
                        .checkbox(&mut checked, RichText::new(t(item.label)).strong())
                        .changed()
                    {
                        changed = true;
                    }
                    ui.label(RichText::new(item.key).monospace().weak());
                });
                ui.label(RichText::new(t(item.help)).small().weak());
                ui.label(
                    RichText::new(crate::trf!(
                        "current_file",
                        current_value
                            .map(|value| format!("{}={}", item.key, value))
                            .unwrap_or_else(|| t("not_set").to_string())
                    ))
                    .small(),
                );
            });

            if changed {
                self.desired
                    .insert(item.key, if checked { Some("1".to_string()) } else { None });
            }

            ui.add_space(6.0);
        }

        ui.separator();
        ui.label(
            RichText::new(t(
                "these_variables_are_not_hot_reloaded_log_out_and_back_in_to_verify_the_effect",
            ))
            .small()
            .weak(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_env_path() -> (PathBuf, PathBuf) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("lxwc-env-test-{unique}"));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("environment");
        (dir, path)
    }

    #[test]
    fn diff_and_reload_round_trip() {
        let (dir, path) = temp_env_path();
        std::fs::write(&path, "WLR_DRM_NO_MODIFIERS=0\n").unwrap();

        let mut module = EnvModule::with_path(path.clone());
        module.refresh();
        assert_eq!(
            module
                .desired
                .get("WLR_DRM_NO_MODIFIERS")
                .cloned()
                .flatten(),
            Some("0".to_string())
        );

        module
            .desired
            .insert("WLR_DRM_NO_MODIFIERS", Some("1".to_string()));
        module
            .desired
            .insert("WLR_NO_HARDWARE_CURSORS", Some("1".to_string()));

        let tasks = module.pending();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].change_count, 2);

        let Effect::WriteFile { path, contents, .. } = &tasks[0].effects[0] else {
            panic!("expected file write");
        };
        assert!(contents.contains("WLR_DRM_NO_MODIFIERS=1"));
        assert!(contents.contains("WLR_NO_HARDWARE_CURSORS=1"));

        module.on_applied(&ApplyReport::default());
        assert_eq!(module.pending().len(), 1);

        std::fs::write(path, contents).unwrap();
        module.on_applied(&ApplyReport::default());
        assert!(module.pending().is_empty());

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn refresh_restores_baseline_selection() {
        let (dir, path) = temp_env_path();
        std::fs::write(&path, "WLR_NO_HARDWARE_CURSORS=1\n").unwrap();

        let mut module = EnvModule::with_path(path);
        module.refresh();
        module.desired.insert("WLR_NO_HARDWARE_CURSORS", None);
        assert!(module.pending().len() == 1);

        module.refresh();
        assert!(module.pending().is_empty());

        std::fs::remove_dir_all(dir).ok();
    }
}
