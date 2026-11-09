use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, Instant};

use eframe::egui;

use crate::core::keybinds;
use crate::core::model::*;
use crate::core::worker::{WorkerEvent, WorkerHandle, WorkerRequest};
use crate::modules;
use crate::ui::{sidebar, status_bar, theme};

pub const WATCHDOG_SECONDS: u64 = 15;

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

#[derive(Clone, Debug)]
pub struct StatusMessage {
    pub text: String,
    pub error: bool,
}

impl StatusMessage {
    pub fn info(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            error: false,
        }
    }

    pub fn error(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            error: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PendingModeChange {
    pub mode: Mode,
    pub previous: Option<Mode>,
}

#[derive(Clone, Debug)]
pub struct Watchdog {
    pub output: String,
    pub mode: Mode,
    pub previous: Mode,
    pub deadline: Instant,
}

impl Watchdog {
    pub fn remaining_secs(&self) -> u32 {
        self.deadline
            .saturating_duration_since(Instant::now())
            .as_secs_f32()
            .ceil() as u32
    }

    pub fn fraction(&self) -> f32 {
        (self.remaining_secs() as f32 / WATCHDOG_SECONDS as f32).clamp(0.0, 1.0)
    }
}

pub fn keybind_remove_id(key: &str) -> String {
    format!("kb:remove:{}", keybinds::normalize_key(key))
}

pub fn keybind_add_id(edit: &KeybindEdit) -> String {
    format!(
        "kb:add:{}:{}",
        keybinds::normalize_key(&edit.key),
        keybinds::render_actions(&edit.actions)
    )
}

pub fn attrs_to_input(attrs: &[(String, String)]) -> String {
    attrs
        .iter()
        .map(|(key, value)| format!("{key}=\"{value}\""))
        .collect::<Vec<_>>()
        .join(" ")
}

pub struct App {
    worker: WorkerHandle,
    pub baseline: Option<Baseline>,
    pub load_error: Option<String>,
    pub outputs: Vec<OutputInfo>,
    pub outputs_error: Option<String>,
    pub scanning: bool,
    pub current: ModuleKind,
    pub env_desired: BTreeMap<&'static str, Option<String>>,
    pub keybind_form: KeybindForm,
    pub keybind_edits: Vec<KeybindEdit>,
    pub keybind_deletions: Vec<String>,
    pub keybind_filter: String,
    pub keybind_pick_tab: KeyPickTab,
    pub selected_output: Option<String>,
    pub selected_mode: Option<Mode>,
    pub mode_inflight: Option<PendingModeChange>,
    pub watchdog: Option<Watchdog>,
    pub confirm_open: bool,
    pub applying: bool,
    pub failures: HashMap<String, String>,
    pub pending: Vec<Task>,
    pub status: Option<StatusMessage>,
    pub report: Option<ApplyReport>,
    pub report_open: bool,
    reset_on_baseline: bool,
}

impl App {
    pub fn new(ctx: &egui::Context) -> Self {
        let worker = WorkerHandle::spawn(ctx.clone());
        let app = Self {
            worker,
            baseline: None,
            load_error: None,
            outputs: Vec::new(),
            outputs_error: None,
            scanning: true,
            current: ModuleKind::Display,
            env_desired: BTreeMap::new(),
            keybind_form: KeybindForm::default(),
            keybind_edits: Vec::new(),
            keybind_deletions: Vec::new(),
            keybind_filter: String::new(),
            keybind_pick_tab: KeyPickTab::Letters,
            selected_output: None,
            selected_mode: None,
            mode_inflight: None,
            watchdog: None,
            confirm_open: false,
            applying: false,
            failures: HashMap::new(),
            pending: Vec::new(),
            status: None,
            report: None,
            report_open: false,
            reset_on_baseline: true,
        };
        app.worker.send(WorkerRequest::Refresh);
        app.worker.send(WorkerRequest::ScanOutputs);
        app
    }

    fn poll_events(&mut self) {
        while let Some(event) = self.worker.try_recv() {
            match event {
                WorkerEvent::Baseline(result) => self.on_baseline(result),
                WorkerEvent::Outputs(result) => self.on_outputs(result),
                WorkerEvent::Applied { report, baseline } => self.on_applied(report, baseline),
                WorkerEvent::ModeApplied { output, result } => self.on_mode_applied(output, result),
                WorkerEvent::ModeReverted { output, result } => {
                    self.status = Some(match result {
                        Ok(()) => StatusMessage::info(format!("{output} 已还原到之前的模式")),
                        Err(err) => StatusMessage::error(format!("还原失败：{err}")),
                    });
                    self.worker.send(WorkerRequest::ScanOutputs);
                }
            }
        }
    }

    fn on_baseline(&mut self, result: Result<Baseline, String>) {
        match result {
            Ok(baseline) => {
                let reset = self.reset_on_baseline || self.baseline.is_none();
                self.reset_on_baseline = false;
                self.load_error = None;
                self.adopt_baseline(baseline, reset);
                self.status = Some(StatusMessage::info("配置已重新载入"));
            }
            Err(err) => {
                self.load_error = Some(err.clone());
                self.status = Some(StatusMessage::error(format!("读取配置失败：{err}")));
            }
        }
    }

    fn on_outputs(&mut self, result: Result<Vec<OutputInfo>, String>) {
        self.scanning = false;
        match result {
            Ok(outputs) => {
                self.outputs_error = None;
                self.outputs = outputs;
                let still_exists = self
                    .selected_output
                    .as_ref()
                    .map(|name| self.outputs.iter().any(|item| &item.name == name))
                    .unwrap_or(false);
                if !still_exists {
                    self.selected_output = self
                        .outputs
                        .iter()
                        .find(|item| item.enabled)
                        .or_else(|| self.outputs.first())
                        .map(|item| item.name.clone());
                    self.selected_mode = None;
                }
                self.sync_selected_mode();
            }
            Err(err) => {
                self.outputs_error = Some(err.clone());
                self.status = Some(StatusMessage::error(format!("扫描显示器失败：{err}")));
            }
        }
    }

    fn on_applied(&mut self, report: ApplyReport, baseline: Result<Baseline, String>) {
        self.applying = false;
        self.failures.clear();
        for result in &report.results {
            if !result.ok {
                if let Some(err) = &result.error {
                    self.failures.insert(result.id.clone(), err.clone());
                }
            }
        }

        let keybind_results: Vec<&TaskResult> = report
            .results
            .iter()
            .filter(|result| result.module == ModuleKind::Keybind)
            .collect();
        if !keybind_results.is_empty() && keybind_results.iter().all(|result| result.ok) {
            self.keybind_edits.clear();
            self.keybind_deletions.clear();
            self.keybind_form.editing_original = None;
            self.keybind_form.key.clear();
            self.keybind_form.error = None;
        }

        match baseline {
            Ok(baseline) => self.adopt_baseline(baseline, false),
            Err(err) => {
                self.load_error = Some(err.clone());
            }
        }

        let success = report.success_count();
        let failure = report.failure_count();
        let mut text = format!("执行完成：成功 {success} 项");
        if failure > 0 {
            text.push_str(&format!("，失败 {failure} 项（已保留在队列）"));
        }
        if let Some(err) = &report.reconfigure_error {
            text.push_str(&format!("；labwc 重载失败：{err}"));
        }
        self.status = Some(StatusMessage {
            text,
            error: failure > 0 || report.reconfigure_error.is_some(),
        });
        self.report_open = failure > 0 || report.reconfigure_error.is_some();
        self.report = Some(report);
    }

    fn on_mode_applied(&mut self, output: String, result: Result<(), String>) {
        let pending = self.mode_inflight.take();
        match result {
            Ok(()) => {
                self.selected_output = Some(output.clone());
                self.sync_selected_mode();
                let previous = pending.as_ref().and_then(|item| item.previous.clone());
                let requested = pending.as_ref().map(|item| item.mode.clone());
                match (previous, requested) {
                    (Some(previous), Some(mode)) => {
                        self.watchdog = Some(Watchdog {
                            output: output.clone(),
                            mode: mode.clone(),
                            previous,
                            deadline: Instant::now() + Duration::from_secs(WATCHDOG_SECONDS),
                        });
                        self.status = Some(StatusMessage::info(format!(
                            "已切换到 {}，{} 秒内确认否则自动回滚",
                            mode.label(),
                            WATCHDOG_SECONDS
                        )));
                    }
                    _ => {
                        self.status = Some(StatusMessage::error(
                            "已切换模式，但无法获取原模式，自动回滚未启用",
                        ));
                    }
                }
                self.worker.send(WorkerRequest::ScanOutputs);
            }
            Err(err) => {
                self.status = Some(StatusMessage::error(format!("切换失败：{err}")));
                self.worker.send(WorkerRequest::ScanOutputs);
            }
        }
    }

    fn adopt_baseline(&mut self, baseline: Baseline, reset: bool) {
        if reset || self.baseline.is_none() {
            self.env_desired.clear();
            for item in MANAGED_ENV {
                self.env_desired
                    .insert(item.key, baseline.environment.get(item.key).cloned());
            }
            self.keybind_edits.clear();
            self.keybind_deletions.clear();
            self.keybind_form = KeybindForm::default();
            self.failures.clear();
        } else {
            for item in MANAGED_ENV {
                self.env_desired
                    .entry(item.key)
                    .or_insert_with(|| baseline.environment.get(item.key).cloned());
            }
        }
        self.baseline = Some(baseline);
        self.sync_selected_mode();
    }

    fn update_watchdog(&mut self, ctx: &egui::Context) {
        let deadline = match &self.watchdog {
            Some(watchdog) => watchdog.deadline,
            None => return,
        };
        let now = Instant::now();
        if now >= deadline {
            self.revert_mode();
        } else {
            ctx.request_repaint_after((deadline - now).min(Duration::from_millis(200)));
        }
    }

    pub fn build_tasks(&self) -> Vec<Task> {
        let Some(baseline) = &self.baseline else {
            return Vec::new();
        };
        let mut tasks = Vec::new();

        for item in MANAGED_ENV {
            let desired = self.env_desired.get(item.key).cloned().flatten();
            let current = baseline.environment.get(item.key).cloned();
            if desired == current {
                continue;
            }
            match desired {
                Some(value) => tasks.push(Task {
                    id: format!("env:{}={}", item.key, value),
                    module: ModuleKind::Environment,
                    title: format!("设置 {}={}", item.key, value),
                    detail: format!("{}：{}（重新登录后生效）", item.label, item.help),
                    action: TaskAction::EnvSet {
                        key: item.key.to_string(),
                        value,
                    },
                }),
                None => tasks.push(Task {
                    id: format!("env:{}", item.key),
                    module: ModuleKind::Environment,
                    title: format!("移除环境变量 {}", item.key),
                    detail: format!("从 environment 文件中删除 {}", item.key),
                    action: TaskAction::EnvUnset {
                        key: item.key.to_string(),
                    },
                }),
            }
        }

        for deletion in &self.keybind_deletions {
            let normalized = keybinds::normalize_key(deletion);
            let Some(entry) = baseline
                .keybinds
                .iter()
                .find(|entry| keybinds::normalize_key(&entry.key) == normalized)
            else {
                continue;
            };
            tasks.push(Task {
                id: keybind_remove_id(&entry.key),
                module: ModuleKind::Keybind,
                title: format!("删除快捷键 {}", entry.key),
                detail: entry.summary(),
                action: TaskAction::KeybindRemove {
                    key: entry.key.clone(),
                },
            });
        }

        for edit in &self.keybind_edits {
            let target = keybinds::normalize_key(&edit.key);
            let target_entry = baseline
                .keybinds
                .iter()
                .find(|entry| keybinds::normalize_key(&entry.key) == target);

            if let Some(original) = &edit.original_key {
                let original_norm = keybinds::normalize_key(original);
                if let Some(entry) = baseline
                    .keybinds
                    .iter()
                    .find(|entry| keybinds::normalize_key(&entry.key) == original_norm)
                {
                    let unchanged = original_norm == target
                        && entry.actions == edit.actions
                        && entry.attrs == edit.attrs;
                    if unchanged {
                        continue;
                    }
                    if original_norm != target {
                        tasks.push(Task {
                            id: keybind_remove_id(&entry.key),
                            module: ModuleKind::Keybind,
                            title: format!("删除快捷键 {}", entry.key),
                            detail: entry.summary(),
                            action: TaskAction::KeybindRemove {
                                key: entry.key.clone(),
                            },
                        });
                    }
                }
            } else if let Some(entry) = target_entry {
                if entry.actions == edit.actions && entry.attrs == edit.attrs {
                    continue;
                }
            }

            let replace = target_entry.is_some();
            tasks.push(Task {
                id: keybind_add_id(edit),
                module: ModuleKind::Keybind,
                title: format!("快捷键 {} → {}", edit.key, edit.summary()),
                detail: if replace {
                    "覆盖 rc.xml 中已有的同名按键".to_string()
                } else {
                    "写入新的快捷键绑定".to_string()
                },
                action: TaskAction::KeybindAdd {
                    key: edit.key.clone(),
                    attrs: edit.attrs.clone(),
                    actions: edit.actions.clone(),
                    replace,
                },
            });
        }

        tasks
    }

    pub fn refresh(&mut self) {
        if self.applying {
            return;
        }
        self.confirm_open = false;
        self.reset_on_baseline = true;
        self.status = Some(StatusMessage::info("正在重新读取配置文件…"));
        self.worker.send(WorkerRequest::Refresh);
        if !self.scanning {
            self.scanning = true;
            self.worker.send(WorkerRequest::ScanOutputs);
        }
    }

    pub fn rescan_outputs(&mut self) {
        self.scanning = true;
        self.outputs_error = None;
        self.worker.send(WorkerRequest::ScanOutputs);
    }

    pub fn execute_pending(&mut self) {
        self.confirm_open = false;
        if self.pending.is_empty() || self.applying {
            return;
        }
        self.applying = true;
        self.status = Some(StatusMessage::info("正在执行队列…"));
        self.worker.send(WorkerRequest::ApplyTasks(self.pending.clone()));
    }

    pub fn reset_keybind_form(&mut self) {
        self.keybind_form = KeybindForm::default();
    }

    pub fn load_keybind_for_edit(&mut self, key: &str) {
        let normalized = keybinds::normalize_key(key);

        let pending_edit = self
            .keybind_edits
            .iter()
            .find(|edit| {
                edit.original_key
                    .as_deref()
                    .map(keybinds::normalize_key)
                    .as_deref()
                    == Some(normalized.as_str())
            })
            .cloned();
        if let Some(edit) = pending_edit {
            self.fill_form_from_edit(&edit);
            return;
        }

        let Some(baseline) = &self.baseline else {
            return;
        };
        let Some(entry) = baseline
            .keybinds
            .iter()
            .find(|entry| keybinds::normalize_key(&entry.key) == normalized)
        else {
            return;
        };

        let form = &mut self.keybind_form;
        form.editing_original = Some(entry.key.clone());
        form.key = entry.key.clone();
        form.advanced_xml = keybinds::render_actions(&entry.actions);
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
        let form = &mut self.keybind_form;
        form.editing_original = edit.original_key.clone();
        form.key = edit.key.clone();
        form.advanced_xml = keybinds::render_actions(&edit.actions);
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

    pub fn submit_keybind(&mut self) {
        let form = self.keybind_form.clone();

        let key = match keybinds::validate_combo(&form.key) {
            Ok(key) => key,
            Err(err) => {
                self.keybind_form.error = Some(err);
                return;
            }
        };

        let actions = if form.advanced {
            match keybinds::parse_actions_fragment(&form.advanced_xml) {
                Ok(actions) => actions,
                Err(err) => {
                    self.keybind_form.error = Some(err);
                    return;
                }
            }
        } else {
            if let Err(err) = keybinds::validate_action_name(&form.action) {
                self.keybind_form.error = Some(err);
                return;
            }
            let attrs = match keybinds::parse_attrs(&form.attrs) {
                Ok(attrs) => attrs,
                Err(err) => {
                    self.keybind_form.error = Some(err);
                    return;
                }
            };
            let command = form.command.trim().to_string();
            if form.action.trim() == "Execute" && command.is_empty() {
                self.keybind_form.error = Some("Execute 动作必须填写命令".to_string());
                return;
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
                let normalized = keybinds::normalize_key(original);
                self.baseline.as_ref().and_then(|baseline| {
                    baseline
                        .keybinds
                        .iter()
                        .find(|entry| keybinds::normalize_key(&entry.key) == normalized)
                        .map(|entry| entry.attrs.clone())
                })
            })
            .unwrap_or_default();

        let edit = KeybindEdit {
            original_key: form.editing_original.clone(),
            key: key.clone(),
            attrs: keybind_attrs,
            actions,
        };

        let replace_index = edit.original_key.as_ref().and_then(|original| {
            self.keybind_edits.iter().position(|existing| {
                existing.original_key.as_deref() == Some(original.as_str())
            })
        });

        let target = keybinds::normalize_key(&edit.key);
        for (index, other) in self.keybind_edits.iter().enumerate() {
            if Some(index) == replace_index {
                continue;
            }
            let other_target = keybinds::normalize_key(&other.key);
            let other_original = other
                .original_key
                .as_deref()
                .map(keybinds::normalize_key);
            if other_target == target || other_original.as_deref() == Some(target.as_str()) {
                self.keybind_form.error =
                    Some("目标按键与队列中的另一项修改冲突".to_string());
                return;
            }
        }

        if let Some(index) = replace_index {
            self.keybind_edits.remove(index);
        }

        let original_norm = edit
            .original_key
            .as_deref()
            .map(keybinds::normalize_key);
        self.keybind_deletions.retain(|key| {
            let normalized = keybinds::normalize_key(key);
            normalized != target && Some(normalized) != original_norm
        });

        let status = self.edit_status(&edit);
        if status == EditStatus::NoChange {
            self.status = Some(StatusMessage::info("与现状一致，未加入队列"));
        } else {
            self.status = Some(StatusMessage::info(format!(
                "已加入队列：{} → {}",
                edit.key,
                edit.summary()
            )));
            self.keybind_edits.push(edit);
        }

        self.keybind_form.key.clear();
        self.keybind_form.editing_original = None;
        self.keybind_form.error = None;
    }

    pub fn edit_status(&self, edit: &KeybindEdit) -> EditStatus {
        let Some(baseline) = &self.baseline else {
            return EditStatus::New;
        };
        let target = keybinds::normalize_key(&edit.key);
        let target_entry = baseline
            .keybinds
            .iter()
            .find(|entry| keybinds::normalize_key(&entry.key) == target);

        if let Some(original) = &edit.original_key {
            let original_norm = keybinds::normalize_key(original);
            let original_entry = baseline
                .keybinds
                .iter()
                .find(|entry| keybinds::normalize_key(&entry.key) == original_norm);
            if original_norm == target {
                match original_entry {
                    Some(entry)
                        if entry.actions == edit.actions && entry.attrs == edit.attrs =>
                    {
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

    pub fn discard_keybind_edit(&mut self, edit: &KeybindEdit) {
        self.keybind_edits.retain(|existing| existing != edit);
        if self.keybind_form.editing_original == edit.original_key {
            self.keybind_form.editing_original = None;
        }
    }

    pub fn delete_keybind(&mut self, key: &str) {
        let normalized = keybinds::normalize_key(key);
        self.keybind_edits.retain(|edit| {
            let target = keybinds::normalize_key(&edit.key);
            let original = edit
                .original_key
                .as_deref()
                .map(keybinds::normalize_key);
            target != normalized && original.as_deref() != Some(normalized.as_str())
        });
        self.keybind_deletions
            .retain(|item| keybinds::normalize_key(item) != normalized);
        if self
            .keybind_form
            .editing_original
            .as_deref()
            .map(keybinds::normalize_key)
            .as_deref()
            == Some(normalized.as_str())
        {
            self.keybind_form.editing_original = None;
        }
        if let Some(baseline) = &self.baseline {
            if let Some(entry) = baseline
                .keybinds
                .iter()
                .find(|entry| keybinds::normalize_key(&entry.key) == normalized)
            {
                self.keybind_deletions.push(entry.key.clone());
            }
        }
        self.status = Some(StatusMessage::info(format!("已加入删除队列：{key}")));
    }

    pub fn undo_delete_keybind(&mut self, key: &str) {
        let normalized = keybinds::normalize_key(key);
        self.keybind_deletions
            .retain(|item| keybinds::normalize_key(item) != normalized);
    }

    pub fn is_pending_deletion(&self, key: &str) -> bool {
        let normalized = keybinds::normalize_key(key);
        self.keybind_deletions
            .iter()
            .any(|item| keybinds::normalize_key(item) == normalized)
    }

    pub fn keybind_edit_failure(&self, edit: &KeybindEdit) -> Option<String> {
        if let Some(original) = &edit.original_key {
            if let Some(err) = self.failures.get(&keybind_remove_id(original)) {
                return Some(err.clone());
            }
        }
        self.failures.get(&keybind_add_id(edit)).cloned()
    }

    pub fn selected_output_info(&self) -> Option<OutputInfo> {
        let name = self.selected_output.as_ref()?;
        self.outputs.iter().find(|item| &item.name == name).cloned()
    }

    pub fn sync_selected_mode(&mut self) {
        let Some(output) = self.selected_output_info() else {
            self.selected_mode = None;
            return;
        };
        let valid = self
            .selected_mode
            .as_ref()
            .map(|mode| output.modes.iter().any(|item| item.same_mode(mode)))
            .unwrap_or(false);
        if !valid {
            self.selected_mode = output
                .current
                .clone()
                .or_else(|| output.modes.first().cloned());
        }
    }

    pub fn request_mode_apply(&mut self) {
        if self.watchdog.is_some() || self.mode_inflight.is_some() {
            return;
        }
        let Some(output) = self.selected_output_info() else {
            return;
        };
        let Some(mode) = self.selected_mode.clone() else {
            return;
        };
        if !output.enabled || output.current.is_none() {
            self.status = Some(StatusMessage::error("输出未启用，无法安全切换"));
            return;
        }
        if output
            .current
            .as_ref()
            .map(|current| current.same_mode(&mode))
            .unwrap_or(false)
        {
            return;
        }
        self.mode_inflight = Some(PendingModeChange {
            mode: mode.clone(),
            previous: output.current.clone(),
        });
        self.status = Some(StatusMessage::info(format!("正在切换到 {}…", mode.label())));
        self.worker.send(WorkerRequest::ApplyMode {
            output: output.name,
            spec: mode.spec(),
        });
    }

    pub fn revert_mode(&mut self) {
        if let Some(watchdog) = self.watchdog.take() {
            self.worker.send(WorkerRequest::RevertMode {
                output: watchdog.output.clone(),
                spec: watchdog.previous.spec(),
            });
            self.status = Some(StatusMessage::info(format!(
                "正在还原 {} …",
                watchdog.output
            )));
        }
    }

    pub fn env_value(&self, key: &str) -> Option<String> {
        self.env_desired.get(key).cloned().flatten()
    }

    pub fn set_env(&mut self, key: &'static str, enabled: bool) {
        self.env_desired
            .insert(key, if enabled { Some("1".to_string()) } else { None });
    }

    pub fn failure_for(&self, task_id: &str) -> Option<&String> {
        self.failures.get(task_id)
    }

    fn confirm_dialog(&mut self, ctx: &egui::Context) {
        if !self.confirm_open {
            return;
        }
        let tasks = self.pending.clone();
        let mut cancel = false;
        let mut go = false;

        egui::Window::new("确认执行")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(520.0);
                ui.label(format!("即将执行 {} 项变更：", tasks.len()));
                ui.add_space(4.0);
                egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                    for (index, task) in tasks.iter().enumerate() {
                        ui.horizontal_wrapped(|ui| {
                            ui.monospace(format!("{:>2}.", index + 1));
                            ui.label(egui::RichText::new(&task.title).strong());
                        });
                        ui.label(egui::RichText::new(&task.detail).small().weak());
                        ui.add_space(4.0);
                    }
                });
                ui.separator();
                ui.label(
                    egui::RichText::new(
                        "写入前会备份为 *.lxwc.bak；失败的任务会留在队列中等待重试。",
                    )
                    .small()
                    .weak(),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("执行").clicked() {
                        go = true;
                    }
                    if ui.button("取消").clicked() {
                        cancel = true;
                    }
                });
            });

        if go {
            self.execute_pending();
        } else if cancel {
            self.confirm_open = false;
        }
    }

    fn watchdog_overlay(&mut self, ctx: &egui::Context) {
        let Some(watchdog) = self.watchdog.clone() else {
            return;
        };
        let mut keep = false;
        let mut revert = false;

        egui::Window::new("显示设置确认")
            .anchor(egui::Align2::CENTER_TOP, [0.0, 24.0])
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.set_min_width(380.0);
                ui.label(format!(
                    "{} 已切换到 {}",
                    watchdog.output,
                    watchdog.mode.label()
                ));
                ui.add(
                    egui::ProgressBar::new(watchdog.fraction())
                        .text(format!("{} 秒", watchdog.remaining_secs()))
                        .animate(false),
                );
                ui.label(format!(
                    "{} 秒后自动还原到 {}。",
                    watchdog.remaining_secs(),
                    watchdog.previous.label()
                ));
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui.button("立即还原").clicked() {
                        revert = true;
                    }
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("保留此设置").strong(),
                            )
                            .fill(theme::ACCENT),
                        )
                        .clicked()
                    {
                        keep = true;
                    }
                });
            });

        if revert {
            self.revert_mode();
        } else if keep {
            self.watchdog = None;
            self.status = Some(StatusMessage::info("已保留新的显示模式"));
        }
    }

    fn report_dialog(&mut self, ctx: &egui::Context) {
        if !self.report_open {
            return;
        }
        let Some(report) = self.report.clone() else {
            self.report_open = false;
            return;
        };
        let mut close = false;

        egui::Window::new("执行报告")
            .collapsible(false)
            .resizable(true)
            .default_width(560.0)
            .show(ctx, |ui| {
                for file in &report.files {
                    ui.label(format!("写入：{file}"));
                }
                if !report.files.is_empty() {
                    ui.separator();
                }
                for result in &report.results {
                    ui.horizontal_wrapped(|ui| {
                        if result.ok {
                            ui.colored_label(theme::OK, "成功");
                        } else {
                            ui.colored_label(theme::DANGER, "失败");
                        }
                        ui.label(&result.title);
                    });
                    if let Some(err) = &result.error {
                        ui.label(
                            egui::RichText::new(err)
                                .small()
                                .color(theme::DANGER),
                        );
                    }
                }
                if let Some(err) = &report.reconfigure_error {
                    ui.separator();
                    ui.colored_label(theme::DANGER, "labwc 重载失败");
                    ui.label(egui::RichText::new(err).small());
                }
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("耗时 {} ms", report.elapsed_ms)).small(),
                    );
                    if ui.button("关闭").clicked() {
                        close = true;
                    }
                });
            });

        if close {
            self.report_open = false;
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_events();
        self.update_watchdog(ctx);
        self.pending = self.build_tasks();

        sidebar::show(self, ctx);
        status_bar::show(self, ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            let module = self.current;
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add_space(6.0);
                    ui.heading(module.title());
                    ui.label(egui::RichText::new(module.subtitle()).weak());
                    ui.add_space(10.0);
                    match module {
                        ModuleKind::Display => modules::display::show(self, ui),
                        ModuleKind::Keybind => modules::keybind::show(self, ui),
                        ModuleKind::Environment => modules::env::show(self, ui),
                    }
                });
        });

        self.confirm_dialog(ctx);
        self.watchdog_overlay(ctx);
        self.report_dialog(ctx);
    }
}
