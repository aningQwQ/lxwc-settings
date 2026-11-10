use eframe::egui::{self, RichText};

use crate::backends::labwc::layouts::{self, Layout, LayoutEntry};
use crate::backends::labwc::pages::scalar::{EnvState, ScalarState, Spec};
use crate::backends::labwc::{ENVIRONMENT_FILE, GROUP, RC_XML_FILE};
use crate::core::model::ApplyReport;
use crate::i18n::t;
use crate::modules::{ConfigEdit, ModuleCtx, ModuleMeta, SettingsModule};
use crate::ui::theme;

const MODULE_ID: &str = "keyboard";
const ENV_KEYS: &[&str] = &[
    "XKB_DEFAULT_LAYOUT",
    "XKB_DEFAULT_VARIANT",
    "XKB_DEFAULT_OPTIONS",
];

const SPECS: &[Spec] = &[
    Spec {
        path: "/labwc_config/keyboard/repeatRate",
        default: "25",
    },
    Spec {
        path: "/labwc_config/keyboard/repeatDelay",
        default: "600",
    },
    Spec {
        path: "/labwc_config/keyboard/numlock",
        default: "no",
    },
];

const OPTION_PRESETS: [(&str, &str); 9] = [
    ("", "none"),
    ("grp:alt_shift_toggle", "alt_shift"),
    ("grp:ctrl_shift_toggle", "ctrl_shift"),
    ("grp:win_space_toggle", "win_space"),
    ("grp:alt_space_toggle", "alt_space"),
    ("grp:caps_toggle", "caps_lock"),
    ("grp:alt_caps_toggle", "alt_caps_lock"),
    ("grp:shift_caps_toggle", "shift_caps_lock"),
    ("grp:ctrl_alt_toggle", "ctrl_alt"),
];

pub struct KeyboardModule {
    rc: ScalarState,
    env: EnvState,
    catalog: Vec<Layout>,
    layouts: Vec<LayoutEntry>,
    options: String,
    selected_layout: String,
    filter: String,
    dirty_env: bool,
}

impl KeyboardModule {
    pub fn new() -> Self {
        let mut module = Self {
            rc: ScalarState::new(SPECS),
            env: EnvState::new(ENV_KEYS),
            catalog: layouts::layouts(),
            layouts: Vec::new(),
            options: String::new(),
            selected_layout: String::new(),
            filter: String::new(),
            dirty_env: false,
        };
        module.load_fields();
        module
    }

    fn load_fields(&mut self) {
        let layout = self.env.get("XKB_DEFAULT_LAYOUT").unwrap_or_default();
        let variant = self.env.get("XKB_DEFAULT_VARIANT").unwrap_or_default();
        self.layouts = layouts::parse_entries(&layout, &variant);
        self.options = self.env.get("XKB_DEFAULT_OPTIONS").unwrap_or_default();
        self.dirty_env = false;
    }

    fn sync_env(&mut self) {
        self.env.set_text(
            "XKB_DEFAULT_LAYOUT",
            &layouts::format_entries(&self.layouts),
        );
        self.env.set_text("XKB_DEFAULT_VARIANT", "");
        self.env.set_text("XKB_DEFAULT_OPTIONS", &self.options);
    }

    fn filtered_layouts(&self) -> Vec<&Layout> {
        let filter = self.filter.trim().to_lowercase();
        self.catalog
            .iter()
            .filter(|layout| {
                filter.is_empty()
                    || layout.code.to_lowercase().contains(&filter)
                    || layout.description.to_lowercase().contains(&filter)
            })
            .collect()
    }
}

impl SettingsModule for KeyboardModule {
    fn meta(&self) -> ModuleMeta {
        ModuleMeta {
            id: MODULE_ID,
            title_key: "keyboard",
            subtitle_key: "keyboard_subtitle",
            group: GROUP,
        }
    }

    fn refresh(&mut self) {
        self.rc.reload(true);
        self.env.reload(true);
        self.load_fields();
    }

    fn config_edits(&mut self) -> Vec<ConfigEdit> {
        let mut edits = self.rc.edits();
        edits.extend(self.env.edits());
        edits
    }

    fn on_applied(&mut self, report: &ApplyReport) {
        if report.file_succeeded(RC_XML_FILE) {
            self.rc.commit();
        } else if report.file_failed(RC_XML_FILE) {
            self.rc.sync_after_apply();
        }
        if report.file_succeeded(ENVIRONMENT_FILE) {
            self.env.commit();
            self.load_fields();
        } else if report.file_failed(ENVIRONMENT_FILE) {
            self.env.sync_after_apply();
            self.load_fields();
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut ModuleCtx) {
        if let Some(err) = self.rc.error().or_else(|| self.env.error()) {
            ui.colored_label(theme::DANGER, RichText::new(err).small());
        } else if self.rc.has_changes() || self.env.has_changes() {
            if let Some(err) = ctx
                .failures
                .get(RC_XML_FILE)
                .or_else(|| ctx.failures.get(ENVIRONMENT_FILE))
            {
                ui.colored_label(theme::DANGER, RichText::new(err).small());
            }
        }

        section(ui, "keyboard_layouts");
        let mut remove = None;
        let mut variant_changed = false;
        for (index, entry) in self.layouts.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.label(RichText::new(&entry.code).monospace());
                ui.label(t("layout_variant"));
                if ui
                    .add(egui::TextEdit::singleline(&mut entry.variant).desired_width(120.0))
                    .changed()
                {
                    variant_changed = true;
                }
                if ui.button(t("remove")).clicked() {
                    remove = Some(index);
                }
            });
        }
        if variant_changed {
            self.dirty_env = true;
        }
        if let Some(index) = remove {
            self.layouts.remove(index);
            self.dirty_env = true;
        }

        ui.horizontal(|ui| {
            ui.label(t("add_layout"));
            ui.text_edit_singleline(&mut self.filter)
                .on_hover_text(t("search"));
            let filtered = self
                .filtered_layouts()
                .into_iter()
                .map(|layout| layout.code.clone())
                .collect::<Vec<_>>();
            let selected = if self.selected_layout.is_empty() {
                t("select_layout").to_string()
            } else {
                self.selected_layout.clone()
            };
            egui::ComboBox::from_id_source("keyboard-layout-add")
                .selected_text(selected)
                .width(220.0)
                .show_ui(ui, |ui| {
                    for code in filtered {
                        ui.selectable_value(&mut self.selected_layout, code.clone(), code);
                    }
                });
            if ui.button(t("add")).clicked()
                && !self.selected_layout.is_empty()
                && !self
                    .layouts
                    .iter()
                    .any(|entry| entry.code == self.selected_layout)
            {
                self.layouts.push(LayoutEntry {
                    code: self.selected_layout.clone(),
                    variant: String::new(),
                });
                self.dirty_env = true;
            }
        });

        ui.horizontal(|ui| {
            ui.label(t("layout_switch_options"));
            if ui.text_edit_singleline(&mut self.options).changed() {
                self.dirty_env = true;
            }
        });
        let mut preset = self.options.clone();
        egui::ComboBox::from_id_source("keyboard-option-presets")
            .selected_text(t("presets"))
            .width(180.0)
            .show_ui(ui, |ui| {
                for (value, label) in OPTION_PRESETS {
                    if ui
                        .selectable_value(&mut preset, value.to_string(), t(label))
                        .changed()
                    {
                        self.options = value.to_string();
                        self.dirty_env = true;
                    }
                }
            });

        section(ui, "keyboard_repeat");
        int_slider(
            ui,
            "/labwc_config/keyboard/repeatRate",
            "repeat_rate",
            1..=100,
            &mut self.rc,
        );
        int_slider(
            ui,
            "/labwc_config/keyboard/repeatDelay",
            "repeat_delay",
            100..=2000,
            &mut self.rc,
        );
        let mut numlock = self.rc.bool("/labwc_config/keyboard/numlock");
        if ui.checkbox(&mut numlock, t("numlock")).changed() {
            self.rc.set_bool("/labwc_config/keyboard/numlock", numlock);
        }

        if self.dirty_env {
            self.sync_env();
            self.dirty_env = false;
        }

        ui.add_space(8.0);
        ui.label(
            RichText::new(t("environment_variables_need_a_new_session"))
                .small()
                .weak(),
        );
    }
}

fn section(ui: &mut egui::Ui, title: &'static str) {
    ui.add_space(8.0);
    ui.separator();
    ui.label(RichText::new(t(title)).strong());
}

fn int_slider(
    ui: &mut egui::Ui,
    path: &str,
    label: &'static str,
    range: std::ops::RangeInclusive<i32>,
    rc: &mut ScalarState,
) {
    let mut value = rc.int(path);
    ui.horizontal(|ui| {
        ui.label(t(label));
        if ui.add(egui::Slider::new(&mut value, range)).changed() {
            rc.set_int(path, value);
        }
    });
}
