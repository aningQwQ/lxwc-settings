use eframe::egui::{self, RichText};

use crate::core::model::ApplyReport;
use crate::i18n::t;
use crate::labwc::layouts::{self, Layout};
use crate::modules::scalar::{EnvState, ScalarState, Spec};
use crate::modules::{
    Category, ConfigEdit, ModuleCtx, ModuleMeta, SettingsModule, ENVIRONMENT_TASK_ID,
    RC_XML_TASK_ID,
};
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct LayoutEntry {
    code: String,
    variant: String,
}

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
        self.layouts = parse_layouts(&layout, &variant);
        self.options = self.env.get("XKB_DEFAULT_OPTIONS").unwrap_or_default();
        self.dirty_env = false;
    }

    fn sync_env(&mut self) {
        self.env
            .set_text("XKB_DEFAULT_LAYOUT", &format_layouts(&self.layouts));
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
            category: Category::Labwc,
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
        if report.task_succeeded(RC_XML_TASK_ID, RC_XML_TASK_ID) {
            self.rc.commit();
        } else if report.task_failed(RC_XML_TASK_ID, RC_XML_TASK_ID) {
            self.rc.sync_after_apply();
        }
        if report.task_succeeded(ENVIRONMENT_TASK_ID, ENVIRONMENT_TASK_ID) {
            self.env.commit();
            self.load_fields();
        } else if report.task_failed(ENVIRONMENT_TASK_ID, ENVIRONMENT_TASK_ID) {
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
                .get(RC_XML_TASK_ID)
                .or_else(|| ctx.failures.get(ENVIRONMENT_TASK_ID))
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

fn parse_layouts(layout: &str, variants: &str) -> Vec<LayoutEntry> {
    let variants = variants.split(',').map(str::trim).collect::<Vec<_>>();
    layout
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .enumerate()
        .map(|(index, item)| {
            if let Some((code, variant)) = parse_layout_item(item) {
                LayoutEntry {
                    code: code.to_string(),
                    variant: variant.to_string(),
                }
            } else {
                LayoutEntry {
                    code: item.to_string(),
                    variant: variants
                        .get(index)
                        .copied()
                        .filter(|variant| !variant.is_empty())
                        .unwrap_or_default()
                        .to_string(),
                }
            }
        })
        .collect()
}

fn parse_layout_item(item: &str) -> Option<(&str, &str)> {
    let inner = item.strip_suffix(')')?;
    let (code, variant) = inner.rsplit_once('(')?;
    Some((code.trim(), variant.trim()))
}

fn format_layouts(entries: &[LayoutEntry]) -> String {
    entries
        .iter()
        .map(|entry| {
            let variant = entry.variant.trim();
            if variant.is_empty() {
                entry.code.clone()
            } else {
                format!("{}({})", entry.code, variant)
            }
        })
        .collect::<Vec<_>>()
        .join(",")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_variants_by_index() {
        let entries = parse_layouts("us,de", "intl,neo");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].code, "us");
        assert_eq!(entries[0].variant, "intl");
        assert_eq!(entries[1].code, "de");
        assert_eq!(entries[1].variant, "neo");
    }

    #[test]
    fn parses_existing_variant_syntax() {
        let entries = parse_layouts("us(intl),de", "");
        assert_eq!(entries[0].code, "us");
        assert_eq!(entries[0].variant, "intl");
        assert_eq!(entries[1].code, "de");
        assert!(entries[1].variant.is_empty());
    }

    #[test]
    fn formats_layouts_and_omits_empty_variants() {
        let entries = vec![
            LayoutEntry {
                code: "us".to_string(),
                variant: "intl".to_string(),
            },
            LayoutEntry {
                code: "de".to_string(),
                variant: String::new(),
            },
        ];
        assert_eq!(format_layouts(&entries), "us(intl),de");
    }
}
