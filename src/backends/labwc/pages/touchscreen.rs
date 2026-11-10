use eframe::egui::{self, RichText};

use crate::backends::labwc::pages::scalar::{ScalarState, Spec};
use crate::backends::labwc::{GROUP, RC_XML_FILE};
use crate::core::model::ApplyReport;
use crate::i18n::t;
use crate::modules::{ModuleCtx, ModuleMeta, SettingsModule};
use crate::ui::theme;

const MODULE_ID: &str = "touchscreen";

const SPECS: &[Spec] = &[Spec {
    path: "/labwc_config/libinput/device/calibrationMatrix",
    default: "",
}];

const OPTIONS: [(&str, &str); 5] = [
    ("", "default"),
    ("1 0 0 0 1 0", "normal"),
    ("0 -1 1 1 0 0", "rotate_left"),
    ("0 1 0 -1 0 1", "rotate_right"),
    ("-1 0 1 0 -1 1", "inverted"),
];

pub struct TouchscreenModule {
    rc: ScalarState,
}

impl TouchscreenModule {
    pub fn new() -> Self {
        Self {
            rc: ScalarState::new(SPECS),
        }
    }
}

impl SettingsModule for TouchscreenModule {
    fn meta(&self) -> ModuleMeta {
        ModuleMeta {
            id: MODULE_ID,
            title_key: "touchscreen",
            subtitle_key: "touchscreen_subtitle",
            group: GROUP,
        }
    }

    fn refresh(&mut self) {
        self.rc.reload(true);
    }

    fn config_edits(&mut self) -> Vec<crate::modules::ConfigEdit> {
        self.rc.edits()
    }

    fn on_applied(&mut self, report: &ApplyReport) {
        if report.file_succeeded(RC_XML_FILE) {
            self.rc.commit();
        } else if report.file_failed(RC_XML_FILE) {
            self.rc.sync_after_apply();
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut ModuleCtx) {
        if let Some(err) = self.rc.error() {
            ui.colored_label(theme::DANGER, RichText::new(err).small());
        } else if self.rc.has_changes() {
            if let Some(err) = ctx.failures.get(RC_XML_FILE) {
                ui.colored_label(theme::DANGER, RichText::new(err).small());
            }
        }

        ui.add_space(8.0);
        ui.label(RichText::new(t("touchscreen_rotation")).strong());
        ui.label(RichText::new(t("touchscreen_rotation_help")).small().weak());
        ui.add_space(6.0);

        let mut current = self
            .rc
            .get("/labwc_config/libinput/device/calibrationMatrix")
            .to_string();
        egui::ComboBox::from_id_source("touchscreen-calibration")
            .selected_text(option_label(&current))
            .width(220.0)
            .show_ui(ui, |ui| {
                for (value, key) in OPTIONS {
                    if ui
                        .selectable_value(&mut current, value.to_string(), t(key))
                        .changed()
                    {
                        self.rc.set(
                            "/labwc_config/libinput/device/calibrationMatrix",
                            value.to_string(),
                        );
                    }
                }
            });
    }
}

fn option_label(value: &str) -> String {
    OPTIONS
        .iter()
        .find(|(option, _)| *option == value)
        .map(|(_, key)| t(key).to_string())
        .unwrap_or_else(|| value.to_string())
}
