use eframe::egui::{self, RichText};

use crate::core::model::ApplyReport;
use crate::i18n::t;
use crate::modules::scalar::{ScalarState, Spec};
use crate::modules::{Category, ModuleCtx, ModuleMeta, SettingsModule, RC_XML_TASK_ID};
use crate::ui::theme;

const MODULE_ID: &str = "behaviour";

const SPECS: &[Spec] = &[
    Spec {
        path: "/labwc_config/placement/policy",
        default: "cascade",
    },
    Spec {
        path: "/labwc_config/focus/followMouse",
        default: "no",
    },
    Spec {
        path: "/labwc_config/focus/followMouseRequiresMovement",
        default: "no",
    },
    Spec {
        path: "/labwc_config/focus/raiseOnFocus",
        default: "no",
    },
    Spec {
        path: "/labwc_config/focus/raiseOnFocusDelay",
        default: "0",
    },
    Spec {
        path: "/labwc_config/core/gap",
        default: "0",
    },
    Spec {
        path: "/labwc_config/snapping/cornerRange",
        default: "50",
    },
    Spec {
        path: "/labwc_config/snapping/overlay/enabled",
        default: "yes",
    },
    Spec {
        path: "/labwc_config/snapping/topMaximize",
        default: "yes",
    },
    Spec {
        path: "/labwc_config/snapping/notifyClient",
        default: "always",
    },
    Spec {
        path: "/labwc_config/resistance/screenEdgeStrength",
        default: "20",
    },
    Spec {
        path: "/labwc_config/resistance/windowEdgeStrength",
        default: "20",
    },
    Spec {
        path: "/labwc_config/resistance/unSnapTreshold",
        default: "20",
    },
    Spec {
        path: "/labwc_config/resistance/unMaximizeTreshold",
        default: "150",
    },
    Spec {
        path: "/labwc_config/resize/drawContents",
        default: "yes",
    },
    Spec {
        path: "/labwc_config/resize/cornerRange",
        default: "8",
    },
    Spec {
        path: "/labwc_config/resize/minimumArea",
        default: "8",
    },
    Spec {
        path: "/labwc_config/resize/popupShow",
        default: "never",
    },
    Spec {
        path: "/labwc_config/magnifier/width",
        default: "400",
    },
    Spec {
        path: "/labwc_config/magnifier/height",
        default: "400",
    },
    Spec {
        path: "/labwc_config/magnifier/initScale",
        default: "2",
    },
    Spec {
        path: "/labwc_config/magnifier/increment",
        default: "0.2",
    },
    Spec {
        path: "/labwc_config/magnifier/useFilter",
        default: "yes",
    },
];

pub struct BehaviourModule {
    rc: ScalarState,
}

impl BehaviourModule {
    pub fn new() -> Self {
        Self {
            rc: ScalarState::new(SPECS),
        }
    }
}

impl SettingsModule for BehaviourModule {
    fn meta(&self) -> ModuleMeta {
        ModuleMeta {
            id: MODULE_ID,
            title_key: "window_behaviour",
            subtitle_key: "window_behaviour_subtitle",
            category: Category::Labwc,
        }
    }

    fn refresh(&mut self) {
        self.rc.reload(true);
    }

    fn config_edits(&mut self) -> Vec<crate::modules::ConfigEdit> {
        self.rc.edits()
    }

    fn on_applied(&mut self, report: &ApplyReport) {
        if report.task_succeeded(RC_XML_TASK_ID, RC_XML_TASK_ID) {
            self.rc.commit();
        } else if report.task_failed(RC_XML_TASK_ID, RC_XML_TASK_ID) {
            self.rc.sync_after_apply();
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut ModuleCtx) {
        if let Some(err) = self.rc.error() {
            ui.colored_label(theme::DANGER, RichText::new(err).small());
        } else if self.rc.has_changes() {
            if let Some(err) = ctx.failures.get(RC_XML_TASK_ID) {
                ui.colored_label(theme::DANGER, RichText::new(err).small());
            }
        }

        section(ui, "placement");
        combo(
            ui,
            "behaviour-placement",
            "/labwc_config/placement/policy",
            &[
                ("automatic", "automatic"),
                ("cascade", "cascade"),
                ("center", "center"),
                ("cursor", "cursor"),
            ],
            &mut self.rc,
        );
        int_slider(ui, "/labwc_config/core/gap", "gap", 0..=100, &mut self.rc);

        section(ui, "focus");
        checkbox(
            ui,
            "/labwc_config/focus/followMouse",
            "focus_follows_mouse",
            &mut self.rc,
        );
        let follow = self.rc.bool("/labwc_config/focus/followMouse");
        checkbox_enabled(
            ui,
            "/labwc_config/focus/followMouseRequiresMovement",
            "focus_follows_mouse_requires_movement",
            follow,
            &mut self.rc,
        );
        checkbox_enabled(
            ui,
            "/labwc_config/focus/raiseOnFocus",
            "raise_on_focus",
            follow,
            &mut self.rc,
        );
        let raise = self.rc.bool("/labwc_config/focus/raiseOnFocus");
        int_slider_enabled(
            ui,
            "/labwc_config/focus/raiseOnFocusDelay",
            "raise_on_focus_delay",
            0..=1000,
            follow && raise,
            &mut self.rc,
        );

        section(ui, "snapping");
        int_slider(
            ui,
            "/labwc_config/snapping/cornerRange",
            "snap_corner_range",
            0..=500,
            &mut self.rc,
        );
        checkbox(
            ui,
            "/labwc_config/snapping/overlay/enabled",
            "show_snap_overlay",
            &mut self.rc,
        );
        checkbox(
            ui,
            "/labwc_config/snapping/topMaximize",
            "maximize_on_top_edge",
            &mut self.rc,
        );
        combo(
            ui,
            "behaviour-notify-client",
            "/labwc_config/snapping/notifyClient",
            &[
                ("always", "always"),
                ("region", "only_on_regions"),
                ("edge", "only_on_edges"),
                ("never", "never"),
            ],
            &mut self.rc,
        );

        section(ui, "resistance");
        int_slider(
            ui,
            "/labwc_config/resistance/screenEdgeStrength",
            "screen_edge_strength",
            0..=100,
            &mut self.rc,
        );
        int_slider(
            ui,
            "/labwc_config/resistance/windowEdgeStrength",
            "window_edge_strength",
            0..=100,
            &mut self.rc,
        );
        int_slider(
            ui,
            "/labwc_config/resistance/unSnapTreshold",
            "unsnap_threshold",
            0..=500,
            &mut self.rc,
        );
        int_slider(
            ui,
            "/labwc_config/resistance/unMaximizeTreshold",
            "unmaximize_threshold",
            0..=500,
            &mut self.rc,
        );

        section(ui, "resize");
        checkbox(
            ui,
            "/labwc_config/resize/drawContents",
            "draw_contents_while_resizing",
            &mut self.rc,
        );
        int_slider(
            ui,
            "/labwc_config/resize/cornerRange",
            "resize_corner_range",
            0..=50,
            &mut self.rc,
        );
        int_slider(
            ui,
            "/labwc_config/resize/minimumArea",
            "resize_minimum_area",
            0..=100,
            &mut self.rc,
        );
        combo(
            ui,
            "behaviour-resize-popup",
            "/labwc_config/resize/popupShow",
            &[
                ("never", "never"),
                ("always", "always"),
                ("nonpixel", "nonpixel"),
            ],
            &mut self.rc,
        );

        section(ui, "magnifier");
        int_slider(
            ui,
            "/labwc_config/magnifier/width",
            "width",
            -1..=2000,
            &mut self.rc,
        );
        int_slider(
            ui,
            "/labwc_config/magnifier/height",
            "height",
            -1..=2000,
            &mut self.rc,
        );
        float_slider(
            ui,
            "/labwc_config/magnifier/initScale",
            "initial_scale",
            0.1..=10.0,
            &mut self.rc,
        );
        float_slider(
            ui,
            "/labwc_config/magnifier/increment",
            "increment",
            0.01..=5.0,
            &mut self.rc,
        );
        checkbox(
            ui,
            "/labwc_config/magnifier/useFilter",
            "use_bilinear_filter",
            &mut self.rc,
        );
        ui.label(
            RichText::new(t("full_screen_magnifier_hint"))
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

fn int_slider_enabled(
    ui: &mut egui::Ui,
    path: &str,
    label: &'static str,
    range: std::ops::RangeInclusive<i32>,
    enabled: bool,
    rc: &mut ScalarState,
) {
    let mut value = rc.int(path);
    ui.horizontal(|ui| {
        ui.add_enabled(enabled, egui::Label::new(t(label)).selectable(false));
        if ui
            .add_enabled(enabled, egui::Slider::new(&mut value, range))
            .changed()
        {
            rc.set_int(path, value);
        }
    });
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
