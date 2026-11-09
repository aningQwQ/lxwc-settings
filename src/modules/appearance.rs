use eframe::egui::{self, RichText};

use crate::core::model::ApplyReport;
use crate::i18n::t;
use crate::labwc::themes;
use crate::modules::scalar::{ScalarState, Spec};
use crate::modules::{Category, ModuleCtx, ModuleMeta, SettingsModule, RC_XML_TASK_ID};
use crate::ui::theme;

const MODULE_ID: &str = "appearance";

const SPECS: &[Spec] = &[
    Spec {
        path: "/labwc_config/theme/name",
        default: "",
    },
    Spec {
        path: "/labwc_config/theme/cornerRadius",
        default: "8",
    },
    Spec {
        path: "/labwc_config/theme/dropShadows",
        default: "no",
    },
    Spec {
        path: "/labwc_config/theme/dropShadowsOnTiled",
        default: "no",
    },
    Spec {
        path: "/labwc_config/theme/icon",
        default: "",
    },
    Spec {
        path: "/labwc_config/core/decoration",
        default: "server",
    },
    Spec {
        path: "/labwc_config/theme/maximizedDecoration",
        default: "titlebar",
    },
];

pub struct AppearanceModule {
    rc: ScalarState,
    window_themes: Vec<String>,
    icon_themes: Vec<String>,
}

impl AppearanceModule {
    pub fn new() -> Self {
        Self {
            rc: ScalarState::new(SPECS),
            window_themes: themes::window_themes(),
            icon_themes: themes::icon_themes(),
        }
    }

    fn refresh_themes(&mut self) {
        self.window_themes = themes::window_themes();
        self.icon_themes = themes::icon_themes();
    }
}

impl SettingsModule for AppearanceModule {
    fn meta(&self) -> ModuleMeta {
        ModuleMeta {
            id: MODULE_ID,
            title_key: "appearance",
            subtitle_key: "appearance_subtitle",
            category: Category::Labwc,
        }
    }

    fn refresh(&mut self) {
        self.rc.reload(true);
        self.refresh_themes();
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

        section(ui, "window_theme");
        let themes = std::iter::once(String::new())
            .chain(self.window_themes.iter().cloned())
            .collect::<Vec<_>>();
        let mut current = self.rc.get("/labwc_config/theme/name").to_string();
        combo(
            ui,
            "appearance-window-theme",
            &mut current,
            &themes,
            |value| {
                if value.is_empty() {
                    t("default").to_string()
                } else {
                    value.to_string()
                }
            },
            &mut |value| self.rc.set("/labwc_config/theme/name", value),
        );

        section(ui, "icon_theme");
        let icons = std::iter::once(String::new())
            .chain(self.icon_themes.iter().cloned())
            .collect::<Vec<_>>();
        let mut current = self.rc.get("/labwc_config/theme/icon").to_string();
        combo(
            ui,
            "appearance-icon-theme",
            &mut current,
            &icons,
            |value| {
                if value.is_empty() {
                    t("default").to_string()
                } else {
                    value.to_string()
                }
            },
            &mut |value| self.rc.set("/labwc_config/theme/icon", value),
        );

        section(ui, "window_decoration");
        let mut radius = self.rc.int("/labwc_config/theme/cornerRadius");
        ui.horizontal(|ui| {
            ui.label(t("corner_radius"));
            if ui.add(egui::Slider::new(&mut radius, 0..=24)).changed() {
                self.rc.set_int("/labwc_config/theme/cornerRadius", radius);
            }
        });

        let mut shadows = self.rc.bool("/labwc_config/theme/dropShadows");
        if ui.checkbox(&mut shadows, t("drop_shadows")).changed() {
            self.rc.set_bool("/labwc_config/theme/dropShadows", shadows);
        }
        let mut tiled = self.rc.bool("/labwc_config/theme/dropShadowsOnTiled");
        if ui
            .add_enabled(
                shadows,
                egui::Checkbox::new(&mut tiled, t("drop_shadows_on_tiled")),
            )
            .changed()
        {
            self.rc
                .set_bool("/labwc_config/theme/dropShadowsOnTiled", tiled);
        }

        section(ui, "decoration");
        let mut current = self.rc.get("/labwc_config/core/decoration").to_string();
        combo(
            ui,
            "appearance-decoration",
            &mut current,
            &["server".to_string(), "client".to_string()],
            |value| {
                if value == "client" {
                    t("client_side_decoration").to_string()
                } else {
                    t("server_side_decoration").to_string()
                }
            },
            &mut |value| self.rc.set("/labwc_config/core/decoration", value),
        );

        section(ui, "maximized_decoration");
        let mut current = self
            .rc
            .get("/labwc_config/theme/maximizedDecoration")
            .to_string();
        combo(
            ui,
            "appearance-maximized-decoration",
            &mut current,
            &["titlebar".to_string(), "none".to_string()],
            |value| {
                if value == "none" {
                    t("none").to_string()
                } else {
                    t("titlebar").to_string()
                }
            },
            &mut |value| {
                self.rc
                    .set("/labwc_config/theme/maximizedDecoration", value)
            },
        );
    }
}

fn section(ui: &mut egui::Ui, title: &'static str) {
    ui.add_space(8.0);
    ui.label(RichText::new(t(title)).strong());
}

fn combo(
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
        .width(260.0)
        .show_ui(ui, |ui| {
            for option in options {
                if ui
                    .selectable_value(current, option.clone(), label(option))
                    .changed()
                {
                    on_change(option.clone());
                }
            }
        });
}
