use eframe::egui::{self, RichText};

use crate::core::model::ApplyReport;
use crate::i18n::t;
use crate::labwc::themes;
use crate::modules::scalar::{EnvState, ScalarState, Spec};
use crate::modules::{
    Category, ConfigEdit, ModuleCtx, ModuleMeta, SettingsModule, ENVIRONMENT_TASK_ID,
    RC_XML_TASK_ID,
};
use crate::ui::theme;

const MODULE_ID: &str = "mouse";
const ENV_KEYS: &[&str] = &["XCURSOR_THEME", "XCURSOR_SIZE"];

const SPECS: &[Spec] = &[
    Spec {
        path: "/labwc_config/libinput/device/naturalScroll",
        default: "no",
    },
    Spec {
        path: "/labwc_config/libinput/device/leftHanded",
        default: "no",
    },
    Spec {
        path: "/labwc_config/libinput/device/pointerSpeed",
        default: "0",
    },
    Spec {
        path: "/labwc_config/libinput/device/accelProfile",
        default: "adaptive",
    },
    Spec {
        path: "/labwc_config/libinput/device/tap",
        default: "yes",
    },
    Spec {
        path: "/labwc_config/libinput/device/tapButtonMap",
        default: "lrm",
    },
    Spec {
        path: "/labwc_config/libinput/device/tapAndDrag",
        default: "yes",
    },
    Spec {
        path: "/labwc_config/libinput/device/dragLock",
        default: "no",
    },
    Spec {
        path: "/labwc_config/libinput/device/threeFingerDrag",
        default: "no",
    },
    Spec {
        path: "/labwc_config/libinput/device/middleEmulation",
        default: "no",
    },
    Spec {
        path: "/labwc_config/libinput/device/disableWhileTyping",
        default: "yes",
    },
    Spec {
        path: "/labwc_config/libinput/device/clickMethod",
        default: "none",
    },
    Spec {
        path: "/labwc_config/libinput/device/scrollMethod",
        default: "twoFinger",
    },
    Spec {
        path: "/labwc_config/libinput/device/sendEventsMode",
        default: "yes",
    },
    Spec {
        path: "/labwc_config/libinput/device/scrollFactor",
        default: "1",
    },
];

pub struct MouseModule {
    rc: ScalarState,
    env: EnvState,
    cursor_themes: Vec<String>,
}

impl MouseModule {
    pub fn new() -> Self {
        Self {
            rc: ScalarState::new(SPECS),
            env: EnvState::new(ENV_KEYS),
            cursor_themes: themes::cursor_themes(),
        }
    }
}

impl SettingsModule for MouseModule {
    fn meta(&self) -> ModuleMeta {
        ModuleMeta {
            id: MODULE_ID,
            title_key: "mouse_touchpad",
            subtitle_key: "mouse_touchpad_subtitle",
            category: Category::Labwc,
        }
    }

    fn refresh(&mut self) {
        self.rc.reload(true);
        self.env.reload(true);
        self.cursor_themes = themes::cursor_themes();
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
        } else if report.task_failed(ENVIRONMENT_TASK_ID, ENVIRONMENT_TASK_ID) {
            self.env.sync_after_apply();
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

        section(ui, "cursor");
        let cursor_themes = std::iter::once(String::new())
            .chain(self.cursor_themes.iter().cloned())
            .collect::<Vec<_>>();
        let mut current = self.env.get("XCURSOR_THEME").unwrap_or_default();
        string_combo(
            ui,
            "mouse-cursor-theme",
            &mut current,
            &cursor_themes,
            |value| {
                if value.is_empty() {
                    t("default").to_string()
                } else {
                    value.to_string()
                }
            },
            &mut |value| self.env.set_text("XCURSOR_THEME", &value),
        );
        let mut size = self
            .env
            .get("XCURSOR_SIZE")
            .and_then(|value| value.parse().ok())
            .unwrap_or(24);
        ui.horizontal(|ui| {
            ui.label(t("cursor_size"));
            if ui.add(egui::Slider::new(&mut size, 8..=96)).changed() {
                self.env.set_int("XCURSOR_SIZE", size);
            }
        });

        section(ui, "pointer");
        checkbox(
            ui,
            "/labwc_config/libinput/device/naturalScroll",
            "natural_scroll",
            &mut self.rc,
        );
        checkbox(
            ui,
            "/labwc_config/libinput/device/leftHanded",
            "left_handed",
            &mut self.rc,
        );
        combo(
            ui,
            "mouse-accel-profile",
            "/labwc_config/libinput/device/accelProfile",
            &[
                ("flat", "flat"),
                ("adaptive", "adaptive"),
                ("custom", "custom"),
            ],
            &mut self.rc,
        );
        let custom = self.rc.get("/labwc_config/libinput/device/accelProfile") == "custom";
        float_slider_enabled(
            ui,
            "/labwc_config/libinput/device/pointerSpeed",
            "pointer_speed",
            -1.0..=1.0,
            !custom,
            &mut self.rc,
        );
        if custom {
            ui.label(
                RichText::new(t("custom_acceleration_curves_not_implemented"))
                    .small()
                    .weak(),
            );
        }

        section(ui, "tapping");
        checkbox(
            ui,
            "/labwc_config/libinput/device/tap",
            "tap_to_click",
            &mut self.rc,
        );
        combo(
            ui,
            "mouse-tap-map",
            "/labwc_config/libinput/device/tapButtonMap",
            &[("lrm", "left_right_middle"), ("lmr", "left_middle_right")],
            &mut self.rc,
        );
        checkbox(
            ui,
            "/labwc_config/libinput/device/tapAndDrag",
            "tap_and_drag",
            &mut self.rc,
        );
        let tap_drag = self.rc.bool("/labwc_config/libinput/device/tapAndDrag");
        checkbox_enabled(
            ui,
            "/labwc_config/libinput/device/dragLock",
            "drag_lock",
            tap_drag,
            &mut self.rc,
        );
        checkbox(
            ui,
            "/labwc_config/libinput/device/threeFingerDrag",
            "three_finger_drag",
            &mut self.rc,
        );
        checkbox(
            ui,
            "/labwc_config/libinput/device/middleEmulation",
            "middle_button_emulation",
            &mut self.rc,
        );

        section(ui, "scrolling");
        checkbox(
            ui,
            "/labwc_config/libinput/device/disableWhileTyping",
            "disable_while_typing",
            &mut self.rc,
        );
        combo(
            ui,
            "mouse-click-method",
            "/labwc_config/libinput/device/clickMethod",
            &[
                ("none", "none"),
                ("buttonAreas", "button_areas"),
                ("clickFinger", "clickfinger"),
            ],
            &mut self.rc,
        );
        combo(
            ui,
            "mouse-scroll-method",
            "/labwc_config/libinput/device/scrollMethod",
            &[
                ("twoFinger", "two_finger"),
                ("edge", "edge"),
                ("onbutton", "on_button"),
                ("none", "none"),
            ],
            &mut self.rc,
        );
        combo(
            ui,
            "mouse-send-events",
            "/labwc_config/libinput/device/sendEventsMode",
            &[
                ("yes", "enabled"),
                ("disabledOnExternalMouse", "disable_with_external_mouse"),
            ],
            &mut self.rc,
        );
        float_slider(
            ui,
            "/labwc_config/libinput/device/scrollFactor",
            "scroll_factor",
            0.1..=5.0,
            &mut self.rc,
        );

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

fn checkbox(ui: &mut egui::Ui, path: &str, label: &'static str, rc: &mut ScalarState) {
    let mut value = rc.bool(path);
    if ui.checkbox(&mut value, t(label)).changed() {
        rc.set_bool(path, value);
    }
}

fn checkbox_enabled(
    ui: &mut egui::Ui,
    path: &str,
    label: &'static str,
    enabled: bool,
    rc: &mut ScalarState,
) {
    let mut value = rc.bool(path);
    if ui
        .add_enabled(enabled, egui::Checkbox::new(&mut value, t(label)))
        .changed()
    {
        rc.set_bool(path, value);
    }
}

fn float_slider(
    ui: &mut egui::Ui,
    path: &str,
    label: &'static str,
    range: std::ops::RangeInclusive<f32>,
    rc: &mut ScalarState,
) {
    let mut value = rc.float(path);
    ui.horizontal(|ui| {
        ui.label(t(label));
        if ui.add(egui::Slider::new(&mut value, range)).changed() {
            rc.set_float(path, value);
        }
    });
}

fn float_slider_enabled(
    ui: &mut egui::Ui,
    path: &str,
    label: &'static str,
    range: std::ops::RangeInclusive<f32>,
    enabled: bool,
    rc: &mut ScalarState,
) {
    let mut value = rc.float(path);
    ui.horizontal(|ui| {
        ui.add_enabled(enabled, egui::Label::new(t(label)));
        if ui
            .add_enabled(enabled, egui::Slider::new(&mut value, range))
            .changed()
        {
            rc.set_float(path, value);
        }
    });
}

fn combo(
    ui: &mut egui::Ui,
    id: &str,
    path: &str,
    options: &[(&str, &'static str)],
    rc: &mut ScalarState,
) {
    let mut current = rc.get(path).to_string();
    let selected = options
        .iter()
        .find(|(value, _)| *value == current)
        .map(|(_, label)| t(label).to_string())
        .unwrap_or_else(|| current.clone());
    egui::ComboBox::from_id_source(id)
        .selected_text(selected)
        .width(220.0)
        .show_ui(ui, |ui| {
            for (value, label) in options {
                if ui
                    .selectable_value(&mut current, (*value).to_string(), t(label))
                    .changed()
                {
                    rc.set(path, (*value).to_string());
                }
            }
        });
}

fn string_combo(
    ui: &mut egui::Ui,
    id: &str,
    current: &mut String,
    options: &[String],
    label: impl Fn(&str) -> String,
    on_change: &mut dyn FnMut(String),
) {
    let selected = label(current);
    egui::ComboBox::from_id_source(id)
        .selected_text(selected)
        .width(220.0)
        .show_ui(ui, |ui| {
            for option in options.iter() {
                if ui
                    .selectable_value(current, option.clone(), label(option))
                    .changed()
                {
                    on_change(option.clone());
                }
            }
        });
}
