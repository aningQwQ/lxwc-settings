use eframe::egui::{self, RichText};

use crate::core::model::ApplyReport;
use crate::i18n::t;
use crate::labwc::accel::{self, CurveError};
use crate::labwc::{rc_xml, themes};
use crate::modules::scalar::{EnvState, ScalarState, Spec};
use crate::modules::{
    Category, ConfigEdit, ModuleCtx, ModuleMeta, PatchOutcome, SettingsModule, ENVIRONMENT_TASK_ID,
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

const CURVES: [(&str, &str); 3] = [
    ("motionCurve", "define_motion_curve"),
    ("scrollCurve", "define_scroll_curve"),
    ("fallbackCurve", "define_fallback_curve"),
];
const CURVE_BASE: &str = "/labwc_config/libinput/device/";

#[derive(Clone, Debug, PartialEq, Eq)]
struct CurveSnapshot {
    defined: bool,
    step: String,
    points: String,
}

struct CurveEditor {
    name: &'static str,
    title: &'static str,
    path: String,
    defined: bool,
    step: String,
    points: String,
    original: CurveSnapshot,
    error: Option<CurveError>,
}

impl CurveEditor {
    fn new(name: &'static str, title: &'static str) -> Self {
        Self {
            name,
            title,
            path: format!("{CURVE_BASE}{name}"),
            defined: false,
            step: String::new(),
            points: String::new(),
            original: CurveSnapshot {
                defined: false,
                step: String::new(),
                points: String::new(),
            },
            error: None,
        }
    }

    fn reload(&mut self, raw: &str) {
        self.defined = rc_xml::exists(raw, &self.path);
        self.step = rc_xml::get(raw, &format!("{}/step", self.path))
            .or_else(|| rc_xml::get_attr(raw, &self.path, "step"))
            .unwrap_or_else(|| "1".to_string());
        self.points = rc_xml::get(raw, &format!("{}/points", self.path))
            .or_else(|| rc_xml::get_attr(raw, &self.path, "points"))
            .unwrap_or_else(|| "0 1".to_string());
        self.original = CurveSnapshot {
            defined: self.defined,
            step: self.step.clone(),
            points: self.points.clone(),
        };
        self.validate();
    }

    fn validate(&mut self) {
        self.error = if self.defined {
            accel::validate(&self.step, &self.points).err()
        } else {
            None
        };
    }

    fn is_valid(&self) -> bool {
        !self.defined || self.error.is_none()
    }

    fn is_changed(&self) -> bool {
        if self.defined != self.original.defined {
            return true;
        }
        self.defined && (self.step != self.original.step || self.points != self.original.points)
    }

    fn show(&mut self, ui: &mut egui::Ui) {
        let mut defined = self.defined;
        if ui.checkbox(&mut defined, t(self.title)).changed() {
            self.defined = defined;
            self.validate();
        }
        if self.defined {
            ui.horizontal(|ui| {
                ui.label(t("step"));
                if ui.text_edit_singleline(&mut self.step).changed() {
                    self.validate();
                }
            });
            ui.horizontal(|ui| {
                ui.label(t("points"));
                if ui.text_edit_singleline(&mut self.points).changed() {
                    self.validate();
                }
            });
            if let Some(error) = self.error {
                let key = match error {
                    CurveError::Step => "curve_step_error",
                    CurveError::Points => "curve_points_error",
                };
                ui.colored_label(theme::DANGER, RichText::new(t(key)).small());
            }
        }
        ui.add_space(4.0);
    }
}

pub struct MouseModule {
    rc: ScalarState,
    env: EnvState,
    cursor_themes: Vec<String>,
    curves: Vec<CurveEditor>,
}

impl MouseModule {
    pub fn new() -> Self {
        let mut module = Self {
            rc: ScalarState::new(SPECS),
            env: EnvState::new(ENV_KEYS),
            cursor_themes: themes::cursor_themes(),
            curves: CURVES
                .iter()
                .map(|(name, title)| CurveEditor::new(name, title))
                .collect(),
        };
        module.load_curves();
        module
    }

    fn load_curves(&mut self) {
        let raw = self.rc.raw().to_string();
        for curve in &mut self.curves {
            curve.reload(&raw);
        }
    }

    fn curves_patch(&self, raw: &str) -> Result<Option<PatchOutcome>, String> {
        if self.rc.get("/labwc_config/libinput/device/accelProfile") != "custom"
            || !self.curves.iter().all(CurveEditor::is_valid)
        {
            return Ok(None);
        }

        let mut current = raw.to_string();
        let mut changes = Vec::new();
        for curve in &self.curves {
            if !curve.is_changed() {
                continue;
            }
            if !curve.defined {
                current = rc_xml::remove(&current, &curve.path)?;
                changes.push(format!("-{}", curve.name));
                continue;
            }
            let (step, points) = accel::validate(&curve.step, &curve.points)
                .map_err(|_| crate::i18n::t("curve_points_error").to_string())?;
            let step_path = format!("{}/step", curve.path);
            let points_path = format!("{}/points", curve.path);
            current = rc_xml::remove(&current, &curve.path)?;
            current = rc_xml::set(&current, &step_path, &step)?;
            current = rc_xml::set(&current, &points_path, &points)?;
            changes.push(format!("{}: {} / {}", curve.name, step, points));
        }

        if changes.is_empty() {
            return Ok(None);
        }
        Ok(Some(PatchOutcome {
            contents: current,
            summary: changes.join("; "),
            count: changes.len(),
        }))
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
        self.load_curves();
    }

    fn is_valid(&self) -> bool {
        self.rc.get("/labwc_config/libinput/device/accelProfile") != "custom"
            || self.curves.iter().all(CurveEditor::is_valid)
    }

    fn config_edits(&mut self) -> Vec<ConfigEdit> {
        let mut edits = self.rc.edits();
        edits.extend(self.env.edits());
        edits
    }

    fn rc_xml_patch(&mut self, raw: &str) -> Result<Option<PatchOutcome>, String> {
        self.curves_patch(raw)
    }

    fn on_applied(&mut self, report: &ApplyReport) {
        if report.task_succeeded(RC_XML_TASK_ID, RC_XML_TASK_ID) {
            self.rc.commit();
            self.load_curves();
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
        section(ui, "acceleration_curves");
        if custom {
            ui.label(RichText::new(t("acceleration_curves_help")).small().weak());
        } else {
            ui.label(
                RichText::new(t("select_custom_profile_to_edit_curves"))
                    .small()
                    .weak(),
            );
        }
        ui.add_enabled_ui(custom, |ui| {
            for curve in &mut self.curves {
                curve.show(ui);
            }
        });

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    const CURVES_RC: &str = r#"<labwc_config>
  <libinput>
    <device>
      <accelProfile>custom</accelProfile>
      <motionCurve custom="yes">
        <step>1</step>
        <points>0 1</points>
      </motionCurve>
      <scrollCurve keep="yes">
        <step>1</step>
        <points>0 2</points>
      </scrollCurve>
    </device>
  </libinput>
</labwc_config>
"#;

    fn temp_module(raw: &str) -> (PathBuf, MouseModule) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("lxwc-mouse-test-{unique}"));
        std::fs::create_dir_all(&dir).unwrap();
        let rc_path = dir.join("rc.xml");
        let env_path = dir.join("environment");
        std::fs::write(&rc_path, raw).unwrap();

        let mut module = MouseModule {
            rc: ScalarState::with_path(rc_path, SPECS),
            env: EnvState::with_path(env_path, ENV_KEYS),
            cursor_themes: Vec::new(),
            curves: CURVES
                .iter()
                .map(|(name, title)| CurveEditor::new(name, title))
                .collect(),
        };
        module.load_curves();
        (dir, module)
    }

    #[test]
    fn reads_attribute_based_curve() {
        let raw = r#"<labwc_config>
  <libinput>
    <device>
      <accelProfile>custom</accelProfile>
      <fallbackCurve step="2" points="0 2 4" />
    </device>
  </libinput>
</labwc_config>
"#;
        let (dir, module) = temp_module(raw);
        assert!(module.curves[2].defined);
        assert_eq!(module.curves[2].step, "2");
        assert_eq!(module.curves[2].points, "0 2 4");
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn untouched_curves_are_not_rewritten() {
        let (dir, module) = temp_module(CURVES_RC);
        let raw = module.rc.raw().to_string();
        assert!(module.curves_patch(&raw).unwrap().is_none());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn edits_one_curve_and_preserves_the_other() {
        let (dir, mut module) = temp_module(CURVES_RC);
        module.curves[0].step = "2".to_string();
        module.curves[0].points = "0 2 4".to_string();
        module.curves[0].validate();

        let raw = module.rc.raw().to_string();
        let patch = module.curves_patch(&raw).unwrap().unwrap();
        assert!(patch.contents.contains("<step>2</step>"));
        assert!(patch.contents.contains("<points>0 2 4</points>"));
        assert!(patch.contents.contains("<scrollCurve keep=\"yes\">"));
        assert!(patch.contents.contains("<points>0 2</points>"));
        assert!(!patch.contents.contains("custom=\"yes\""));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn disabling_curve_removes_it() {
        let (dir, mut module) = temp_module(CURVES_RC);
        module.curves[0].defined = false;
        module.curves[0].validate();

        let raw = module.rc.raw().to_string();
        let patch = module.curves_patch(&raw).unwrap().unwrap();
        assert!(!patch.contents.contains("<motionCurve"));
        assert!(patch.contents.contains("<scrollCurve keep=\"yes\">"));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn invalid_curve_blocks_apply_and_patch() {
        let (dir, mut module) = temp_module(CURVES_RC);
        module.curves[0].step = "0".to_string();
        module.curves[0].validate();

        assert!(!module.is_valid());
        let raw = module.rc.raw().to_string();
        assert!(module.curves_patch(&raw).unwrap().is_none());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn curves_are_ignored_outside_custom_profile() {
        let (dir, mut module) = temp_module(CURVES_RC);
        module.curves[0].step = "2".to_string();
        module.curves[0].validate();
        module
            .rc
            .set("/labwc_config/libinput/device/accelProfile", "flat");

        let raw = module.rc.raw().to_string();
        assert!(module.curves_patch(&raw).unwrap().is_none());
        std::fs::remove_dir_all(dir).ok();
    }
}
