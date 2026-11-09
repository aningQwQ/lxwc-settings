use eframe::egui::{self, RichText};

use super::file;
use super::types::*;
use super::KeybindModule;
use crate::i18n::t;
use crate::modules::ModuleCtx;
use crate::ui::theme;

pub(super) fn show(module: &mut KeybindModule, ui: &mut egui::Ui, ctx: &mut ModuleCtx) {
    let count = module.baseline.len();

    ui.horizontal(|ui| {
        ui.selectable_value(&mut module.view, KeybindView::Edit, t("edit_add"));
        ui.selectable_value(
            &mut module.view,
            KeybindView::Existing,
            crate::trf!("existing_shortcuts", count),
        );
    });
    ui.separator();

    match module.view {
        KeybindView::Edit => {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| edit_view(module, ui, ctx));
        }
        KeybindView::Existing => existing_view(module, ui, ctx),
    }
}

fn edit_view(module: &mut KeybindModule, ui: &mut egui::Ui, ctx: &mut ModuleCtx) {
    ui.label(
        RichText::new(t("type_a_labwc_keysym_combination_and_action_or_load_one_from_the_existing_shortcuts_tab"))
        .small()
        .weak(),
    );
    if !module.exists {
        ui.colored_label(
            theme::DANGER,
            crate::trf!(
                "was_not_found_key_bindings_cannot_be_written",
                module.path.display()
            ),
        );
    }
    if let Some(err) = &module.error {
        ui.colored_label(theme::DANGER, RichText::new(err).small());
    }

    ui.add_space(8.0);
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(t("key_combination"));
            ui.add(
                egui::TextEdit::singleline(&mut module.form.key)
                    .hint_text(t("e_g_w_c_s_t_w_return_print_xf86audiomute"))
                    .desired_width(320.0)
                    .font(egui::TextStyle::Monospace),
            );
        });
        ui.label(
            RichText::new(t("use_w_c_a_s_for_modifiers_and_an_xkb_keysym_name_for_the_key_common_keysyms_return_space_left_print_f12_xf86audiomute_labwc_does_not_support_multi_step_key_sequences"))
            .small()
            .weak(),
        );

        ui.add_space(8.0);
        soft_keyboard(module, ui);
        ui.add_space(6.0);
        let mut advanced = module.form.advanced;
        ui.horizontal(|ui| {
            ui.label(t("action_mode"));
            ui.selectable_value(&mut advanced, false, t("common"));
            ui.selectable_value(&mut advanced, true, t("advanced_xml"));
        });
        if advanced != module.form.advanced {
            module.form.advanced = advanced;
            if advanced && module.form.advanced_xml.trim().is_empty() {
                if let Ok(actions) = build_common_actions(&module.form) {
                    module.form.advanced_xml = file::render_actions(&actions);
                }
            }
        }

        if module.form.advanced {
            ui.label(t("action_xml_multiple_actions_allowed"));
            ui.add(
                egui::TextEdit::multiline(&mut module.form.advanced_xml)
                    .code_editor()
                    .desired_rows(4)
                    .desired_width(f32::INFINITY),
            );
            if ui.button(t("regenerate_from_common_fields")).clicked() {
                match build_common_actions(&module.form) {
                    Ok(actions) => {
                        module.form.advanced_xml = file::render_actions(&actions)
                    }
                    Err(err) => module.form.error = Some(err),
                }
            }
        } else {
            ui.horizontal(|ui| {
                ui.label(t("action_name"));
                ui.add(
                    egui::TextEdit::singleline(&mut module.form.action)
                        .desired_width(180.0),
                );
                egui::ComboBox::from_id_source("action-suggest")
                    .selected_text(t("common_actions"))
                    .show_ui(ui, |ui| {
                        for name in ACTION_SUGGESTIONS {
                            let selected = module.form.action == *name;
                            if ui.selectable_label(selected, *name).clicked() {
                                module.form.action = (*name).to_string();
                            }
                        }
                    });
            });
            ui.horizontal(|ui| {
                ui.label(t("command"));
                ui.add(
                    egui::TextEdit::singleline(&mut module.form.command)
                        .hint_text(t("required_for_execute_e_g_lxqt_qdbus_run_qterminal"))
                        .desired_width(440.0),
                );
            });
            ui.horizontal(|ui| {
                ui.label(t("extra_attributes"));
                ui.add(
                    egui::TextEdit::singleline(&mut module.form.attrs)
                        .hint_text(t("e_g_direction_left_combine_yes"))
                        .desired_width(440.0),
                );
            });
            if module.form.editing_original.is_some()
                && module.form.advanced_xml.matches("<action").count() > 1
            {
                ui.label(
                    RichText::new(t("note_this_binding_contains_multiple_actions_submitting_in_common_mode_keeps_only_the_first_one_use_advanced_xml_instead"))
                    .small()
                    .color(theme::WARN),
                );
            }
        }

        if let Ok(preview) = preview_for_form(&module.form) {
            egui::CollapsingHeader::new(t("xml_preview")).show(ui, |ui| {
                ui.label(RichText::new(preview).monospace().small());
            });
        }

        if let Some(err) = &module.form.error {
            ui.colored_label(theme::DANGER, RichText::new(err).small());
        }

        if let Some(original) = &module.form.editing_original {
            ui.label(
                RichText::new(crate::trf!("editing_other_keybind_attributes_are_preserved", original))
                    .small()
                    .weak(),
            );
        }

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let submit_label = if module.form.editing_original.is_some() {
                t("add_update_queue")
            } else {
                t("add_to_queue")
            };
            if ui
                .add(
                    egui::Button::new(
                        RichText::new(submit_label)
                            .strong()
                            .color(egui::Color32::WHITE),
                    )
                    .fill(theme::ACCENT),
                )
                .clicked()
            {
                match module.submit() {
                    Ok(message) => ctx.info(message),
                    Err(err) => module.form.error = Some(err),
                }
            }
            if ui.button(t("clear_form")).clicked() {
                module.reset_form();
            }
            if module.form.editing_original.is_some() && ui.button(t("cancel_editing")).clicked() {
                module.reset_form();
            }
        });
    });

    ui.add_space(14.0);
    ui.label(RichText::new(t("pending_shortcuts")).strong());
    if module.edits.is_empty() && module.deletions.is_empty() {
        ui.label(
            RichText::new(t("the_queue_is_empty_fill_in_a_key_combination_and_action_or_edit_delete_from_the_existing_shortcuts_tab"))
            .small()
            .weak(),
        );
    } else {
        let edits = module.edits.clone();
        let deletions = module.deletions.clone();
        let mut discard: Option<KeybindEdit> = None;

        for edit in &edits {
            let status = module.edit_status(edit);
            let renamed = edit
                .original_key
                .as_ref()
                .map(|original| file::normalize_key(original) != file::normalize_key(&edit.key))
                .unwrap_or(false);

            ui.horizontal(|ui| {
                ui.monospace(&edit.key);
                ui.label("→");
                ui.label(edit.summary());
                ui.colored_label(status_color(status), status.label());
                if renamed {
                    ui.label(
                        RichText::new(crate::trf!(
                            "from",
                            edit.original_key.clone().unwrap_or_default()
                        ))
                        .small()
                        .weak(),
                    );
                }
                if ui.small_button(t("remove")).clicked() {
                    discard = Some(edit.clone());
                }
            });
        }

        if let Some(edit) = discard {
            module.discard_edit(&edit);
        }

        if let Some(err) = ctx.failures.get("keybind") {
            ui.colored_label(theme::DANGER, RichText::new(err).small());
        }

        let mut undo: Option<String> = None;
        for key in &deletions {
            ui.horizontal(|ui| {
                ui.monospace(key);
                ui.colored_label(theme::DANGER, t("pending_delete"));
                if ui.small_button(t("undo")).clicked() {
                    undo = Some(key.clone());
                }
            });
        }
        if let Some(key) = undo {
            module.undo_delete(&key);
        }
    }
}

fn existing_view(module: &mut KeybindModule, ui: &mut egui::Ui, ctx: &mut ModuleCtx) {
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut module.filter)
                .hint_text(t("search_key_action_command"))
                .desired_width(320.0),
        );
        if !module.filter.is_empty() && ui.button(t("clear")).clicked() {
            module.filter.clear();
        }
    });

    let filter = module.filter.trim().to_lowercase();
    let binds = module.baseline.clone();
    let filtered: Vec<&KeybindEntry> = binds
        .iter()
        .filter(|entry| {
            filter.is_empty()
                || entry.key.to_lowercase().contains(&filter)
                || entry.summary().to_lowercase().contains(&filter)
        })
        .collect();

    ui.label(
        RichText::new(crate::trf!(
            "showing_drag_header_dividers_to_resize_click_edit_to_load",
            filtered.len(),
            binds.len()
        ))
        .small()
        .weak(),
    );

    if binds.is_empty() {
        ui.label(RichText::new(t("no_key_bindings_found")).weak());
        return;
    }
    if filtered.is_empty() {
        ui.label(RichText::new(t("no_matching_key_bindings")).weak());
        return;
    }

    let available = (ui.available_height() - 34.0).max(160.0);
    let mut pending_action: Option<(BindAction, String)> = None;

    egui_extras::TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .auto_shrink([false, false])
        .max_scroll_height(available)
        .min_scrolled_height(160.0)
        .column(
            egui_extras::Column::initial(190.0)
                .at_least(110.0)
                .resizable(true)
                .clip(true),
        )
        .column(
            egui_extras::Column::initial(340.0)
                .at_least(160.0)
                .resizable(true)
                .clip(true),
        )
        .column(
            egui_extras::Column::initial(90.0)
                .at_least(70.0)
                .resizable(true)
                .clip(true),
        )
        .column(
            egui_extras::Column::initial(160.0)
                .at_least(140.0)
                .resizable(true)
                .clip(true),
        )
        .header(30.0, |mut header| {
            header.col(|ui| {
                ui.strong(t("key_2"));
            });
            header.col(|ui| {
                ui.strong(t("action_command"));
            });
            header.col(|ui| {
                ui.strong(t("status"));
            });
            header.col(|ui| {
                ui.strong(t("actions_2"));
            });
        })
        .body(|body| {
            body.rows(34.0, filtered.len(), |mut row| {
                let index = row.index();
                let Some(entry) = filtered.get(index) else {
                    return;
                };

                row.col(|ui| {
                    let mut key_label = entry.key.clone();
                    if !entry.attrs.is_empty() {
                        key_label.push_str(&format!(" [{}]", attrs_to_input(&entry.attrs)));
                    }
                    ui.add(egui::Label::new(RichText::new(&key_label).monospace()).truncate())
                        .on_hover_text(key_label);
                });

                row.col(|ui| {
                    let summary = entry.summary();
                    ui.add(egui::Label::new(summary.clone()).truncate())
                        .on_hover_text(summary);
                });

                row.col(|ui| {
                    let pending_delete = module.is_pending_deletion(&entry.key);
                    let pending_edit = module.edits.iter().any(|edit| {
                        edit.original_key
                            .as_deref()
                            .map(file::normalize_key)
                            .as_deref()
                            == Some(file::normalize_key(&entry.key).as_str())
                    });
                    if pending_delete {
                        ui.colored_label(theme::DANGER, t("pending_delete"));
                    } else if pending_edit {
                        ui.colored_label(theme::WARN, t("pending_update"));
                    }
                });

                row.col(|ui| {
                    if module.is_pending_deletion(&entry.key) {
                        if ui.small_button(t("undo_delete")).clicked() {
                            pending_action = Some((BindAction::UndoDelete, entry.key.clone()));
                        }
                    } else {
                        if ui.small_button(t("edit")).clicked() {
                            pending_action = Some((BindAction::Edit, entry.key.clone()));
                        }
                        if ui.small_button(t("delete")).clicked() {
                            pending_action = Some((BindAction::Delete, entry.key.clone()));
                        }
                    }
                });
            });
        });

    if let Some((action, key)) = pending_action {
        match action {
            BindAction::Edit => module.load_for_edit(&key),
            BindAction::Delete => {
                let message = module.delete(&key);
                ctx.info(message);
            }
            BindAction::UndoDelete => module.undo_delete(&key),
        }
    }
}

#[derive(Clone, Copy)]
enum BindAction {
    Edit,
    Delete,
    UndoDelete,
}

fn status_color(status: EditStatus) -> egui::Color32 {
    match status {
        EditStatus::New => theme::OK,
        EditStatus::Overwrite | EditStatus::Update | EditStatus::Rename => theme::WARN,
        EditStatus::NoChange => egui::Color32::GRAY,
    }
}

fn build_common_actions(form: &KeybindForm) -> Result<Vec<KeybindAction>, String> {
    file::validate_action_name(&form.action)?;
    let attrs = file::parse_attrs(&form.attrs)?;
    let command = form.command.trim().to_string();
    if form.action.trim() == "Execute" && command.is_empty() {
        return Err(t("the_execute_action_requires_a_command").to_string());
    }
    Ok(vec![KeybindAction {
        name: form.action.trim().to_string(),
        command,
        attrs,
    }])
}

fn form_actions(form: &KeybindForm) -> Result<Vec<KeybindAction>, String> {
    if form.advanced {
        file::parse_actions_fragment(&form.advanced_xml)
    } else {
        build_common_actions(form)
    }
}

fn preview_for_form(form: &KeybindForm) -> Result<String, String> {
    let key = file::validate_combo(&form.key)?;
    let actions = form_actions(form)?;
    Ok(file::render_keybind_preview(&key, &[], &actions))
}

#[derive(Clone, Copy, Default)]
struct ComboParts {
    sup: bool,
    ctrl: bool,
    alt: bool,
    shift: bool,
}

fn parse_parts(input: &str) -> (ComboParts, Option<String>) {
    let mut parts = ComboParts::default();
    let mut bases: Vec<&str> = Vec::new();
    for token in input.split('-') {
        if token.is_empty() {
            continue;
        }
        match token {
            "W" => parts.sup = true,
            "C" => parts.ctrl = true,
            "A" => parts.alt = true,
            "S" => parts.shift = true,
            _ => bases.push(token),
        }
    }
    let base = if bases.len() == 1 {
        Some(bases[0].to_string())
    } else {
        None
    };
    (parts, base)
}

fn compose_combo(parts: ComboParts, base: Option<&str>) -> String {
    let mut out = String::new();
    for (enabled, label) in [
        (parts.sup, 'W'),
        (parts.ctrl, 'C'),
        (parts.alt, 'A'),
        (parts.shift, 'S'),
    ] {
        if enabled {
            out.push(label);
            out.push('-');
        }
    }
    if let Some(base) = base {
        out.push_str(base);
    }
    out
}

fn toggle_modifier(input: &str, which: char) -> String {
    let (mut parts, base) = parse_parts(input);
    match which {
        'W' => parts.sup = !parts.sup,
        'C' => parts.ctrl = !parts.ctrl,
        'A' => parts.alt = !parts.alt,
        'S' => parts.shift = !parts.shift,
        _ => {}
    }
    compose_combo(parts, base.as_deref())
}

fn set_base_key(input: &str, base: &str) -> String {
    let (parts, _) = parse_parts(input);
    compose_combo(parts, Some(base))
}

fn soft_keyboard(module: &mut KeybindModule, ui: &mut egui::Ui) {
    let mut key_value = module.form.key.clone();
    let mut changed = false;
    let (mods, _) = parse_parts(&key_value);

    ui.label(RichText::new(t("soft_keyboard")).strong());
    ui.horizontal_wrapped(|ui| {
        if modifier_button(ui, "Super", mods.sup) {
            key_value = toggle_modifier(&key_value, 'W');
            changed = true;
        }
        if modifier_button(ui, "Ctrl", mods.ctrl) {
            key_value = toggle_modifier(&key_value, 'C');
            changed = true;
        }
        if modifier_button(ui, "Alt", mods.alt) {
            key_value = toggle_modifier(&key_value, 'A');
            changed = true;
        }
        if modifier_button(ui, "Shift", mods.shift) {
            key_value = toggle_modifier(&key_value, 'S');
            changed = true;
        }
    });

    let (_, base) = parse_parts(&key_value);
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        ui.label(t("key"));
        ui.selectable_value(&mut module.pick_tab, KeyPickTab::Letters, t("letters"));
        ui.selectable_value(&mut module.pick_tab, KeyPickTab::Digits, t("digits"));
        ui.selectable_value(&mut module.pick_tab, KeyPickTab::Function, t("f_keys"));
        ui.selectable_value(
            &mut module.pick_tab,
            KeyPickTab::Navigation,
            t("navigation"),
        );
        ui.selectable_value(&mut module.pick_tab, KeyPickTab::Symbols, t("symbols"));
        ui.selectable_value(&mut module.pick_tab, KeyPickTab::Media, t("media"));
    });

    let keys = pick_keys(module.pick_tab);
    ui.horizontal_wrapped(|ui| {
        for (label, value) in keys {
            let selected = base.as_deref() == Some(value);
            if key_button(ui, &label, value, selected) {
                key_value = set_base_key(&key_value, value);
                changed = true;
            }
        }
    });

    if changed {
        module.form.key = key_value;
    }

    ui.label(
        RichText::new(t("click_modifiers_and_a_key_to_build_the_combination_the_field_above_still_accepts_any_keysym"))
            .small()
            .weak(),
    );
}

fn modifier_button(ui: &mut egui::Ui, label: &str, active: bool) -> bool {
    let text = if active {
        RichText::new(label).strong().color(egui::Color32::WHITE)
    } else {
        RichText::new(label).strong()
    };
    let fill = if active {
        theme::ACCENT
    } else {
        ui.visuals().widgets.inactive.bg_fill
    };
    ui.add_sized([72.0, 30.0], egui::Button::new(text).fill(fill))
        .clicked()
}

fn key_button(ui: &mut egui::Ui, label: &str, value: &str, selected: bool) -> bool {
    let text = if selected {
        RichText::new(label).monospace().color(egui::Color32::WHITE)
    } else {
        RichText::new(label).monospace()
    };
    let fill = if selected {
        theme::ACCENT
    } else {
        ui.visuals().widgets.inactive.bg_fill
    };
    ui.add_sized([72.0, 30.0], egui::Button::new(text).fill(fill))
        .on_hover_text(value)
        .clicked()
}

fn pick_keys(tab: KeyPickTab) -> Vec<(String, &'static str)> {
    let pairs: &[(&str, &str)] = match tab {
        KeyPickTab::Letters => &LETTER_KEYS,
        KeyPickTab::Digits => &DIGIT_KEYS,
        KeyPickTab::Function => &FUNCTION_KEYS,
        KeyPickTab::Navigation => &NAVIGATION_KEYS,
        KeyPickTab::Symbols => &SYMBOL_KEYS,
        KeyPickTab::Media => {
            return MEDIA_KEYS
                .iter()
                .map(|(source, value)| (t(source).to_string(), *value))
                .collect();
        }
    };
    pairs
        .iter()
        .map(|(label, value)| ((*label).to_string(), *value))
        .collect()
}

const LETTER_KEYS: [(&str, &str); 26] = [
    ("A", "a"),
    ("B", "b"),
    ("C", "c"),
    ("D", "d"),
    ("E", "e"),
    ("F", "f"),
    ("G", "g"),
    ("H", "h"),
    ("I", "i"),
    ("J", "j"),
    ("K", "k"),
    ("L", "l"),
    ("M", "m"),
    ("N", "n"),
    ("O", "o"),
    ("P", "p"),
    ("Q", "q"),
    ("R", "r"),
    ("S", "s"),
    ("T", "t"),
    ("U", "u"),
    ("V", "v"),
    ("W", "w"),
    ("X", "x"),
    ("Y", "y"),
    ("Z", "z"),
];

const DIGIT_KEYS: [(&str, &str); 10] = [
    ("0", "0"),
    ("1", "1"),
    ("2", "2"),
    ("3", "3"),
    ("4", "4"),
    ("5", "5"),
    ("6", "6"),
    ("7", "7"),
    ("8", "8"),
    ("9", "9"),
];

const FUNCTION_KEYS: [(&str, &str); 20] = [
    ("F1", "F1"),
    ("F2", "F2"),
    ("F3", "F3"),
    ("F4", "F4"),
    ("F5", "F5"),
    ("F6", "F6"),
    ("F7", "F7"),
    ("F8", "F8"),
    ("F9", "F9"),
    ("F10", "F10"),
    ("F11", "F11"),
    ("F12", "F12"),
    ("F13", "F13"),
    ("F14", "F14"),
    ("F15", "F15"),
    ("F16", "F16"),
    ("F17", "F17"),
    ("F18", "F18"),
    ("F19", "F19"),
    ("F20", "F20"),
];

const NAVIGATION_KEYS: [(&str, &str); 15] = [
    ("Left", "Left"),
    ("Right", "Right"),
    ("Up", "Up"),
    ("Down", "Down"),
    ("Home", "Home"),
    ("End", "End"),
    ("PgUp", "Page_Up"),
    ("PgDn", "Page_Down"),
    ("Ins", "Insert"),
    ("Del", "Delete"),
    ("Enter", "Return"),
    ("Tab", "Tab"),
    ("Space", "space"),
    ("Esc", "Escape"),
    ("Bksp", "BackSpace"),
];

const SYMBOL_KEYS: [(&str, &str); 11] = [
    ("-", "minus"),
    ("=", "equal"),
    (",", "comma"),
    (".", "period"),
    ("/", "slash"),
    ("\\", "backslash"),
    (";", "semicolon"),
    ("'", "apostrophe"),
    ("[", "bracketleft"),
    ("]", "bracketright"),
    ("`", "grave"),
];

const MEDIA_KEYS: [(&str, &str); 22] = [
    ("print", "Print"),
    ("menu", "Menu"),
    ("mute", "XF86AudioMute"),
    ("vol_2", "XF86AudioLowerVolume"),
    ("vol", "XF86AudioRaiseVolume"),
    ("play", "XF86AudioPlay"),
    ("next", "XF86AudioNext"),
    ("prev", "XF86AudioPrev"),
    ("bri_2", "XF86MonBrightnessDown"),
    ("bri", "XF86MonBrightnessUp"),
    ("power", "XF86PowerOff"),
    ("sleep", "XF86Sleep"),
    ("susp", "XF86Suspend"),
    ("search", "XF86Search"),
    ("home", "XF86HomePage"),
    ("mail", "XF86Mail"),
    ("calc", "XF86Calculator"),
    ("disp", "XF86Display"),
    ("touch", "XF86TouchpadToggle"),
    ("wifi", "XF86WLAN"),
    ("batt", "XF86Battery"),
    ("rec", "XF86AudioRecord"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggles_modifiers_and_keeps_base() {
        assert_eq!(toggle_modifier("W-t", 'C'), "W-C-t");
        assert_eq!(toggle_modifier("W-C-t", 'C'), "W-t");
        assert_eq!(toggle_modifier("", 'W'), "W-");
        assert_eq!(toggle_modifier("W-", 'W'), "");
    }

    #[test]
    fn picks_base_and_keeps_modifiers() {
        assert_eq!(set_base_key("W-C-", "p"), "W-C-p");
        assert_eq!(set_base_key("W-C-t", "Return"), "W-C-Return");
        assert_eq!(set_base_key("", "Print"), "Print");
        assert_eq!(set_base_key("W-x-y", "p"), "W-p");
    }

    #[test]
    fn parses_manual_input() {
        let (parts, base) = parse_parts("S-W-p");
        assert!(parts.sup && parts.shift);
        assert_eq!(base.as_deref(), Some("p"));
        let (_, base) = parse_parts("W-Left-x");
        assert_eq!(base, None);
    }
}
