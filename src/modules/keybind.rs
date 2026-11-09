use eframe::egui::{self, RichText};

use crate::app::{attrs_to_input, keybind_remove_id, App, KeyPickTab, KeybindForm};
use crate::core::keybinds;
use crate::core::model::{
    EditStatus, KeybindAction, KeybindEntry, ACTION_SUGGESTIONS,
};
use crate::ui::theme;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    ui.label(
        "直接填写 labwc keysym 组合与动作，不再依赖按键捕获。支持任意修饰键组合、多动作、自定义属性，以及编辑/删除现有绑定。",
    );
    if let Some(baseline) = &app.baseline {
        if !baseline.rc_exists {
            ui.colored_label(
                theme::DANGER,
                format!("未找到 {}，快捷键无法写入。", baseline.rc_path.display()),
            );
        }
    }

    ui.add_space(10.0);
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label("按键组合");
            ui.add(
                egui::TextEdit::singleline(&mut app.keybind_form.key)
                    .hint_text("例如 W-C-S-t、W-Return、Print、XF86AudioMute")
                    .desired_width(320.0)
                    .font(egui::TextStyle::Monospace),
            );
        });
        ui.label(
            RichText::new(
                "修饰键用 W / C / A / S；主键写 XKB keysym 名称。常见：Return、space、Left、Print、F12、XF86AudioMute。labwc 不支持多段按键序列。",
            )
            .small()
            .weak(),
        );

        ui.add_space(8.0);
        soft_keyboard(app, ui);
        ui.add_space(6.0);
        let mut advanced = app.keybind_form.advanced;
        ui.horizontal(|ui| {
            ui.label("动作模式");
            ui.selectable_value(&mut advanced, false, "常用");
            ui.selectable_value(&mut advanced, true, "高级 XML");
        });
        if advanced != app.keybind_form.advanced {
            app.keybind_form.advanced = advanced;
            if advanced && app.keybind_form.advanced_xml.trim().is_empty() {
                if let Ok(actions) = build_common_actions(&app.keybind_form) {
                    app.keybind_form.advanced_xml = keybinds::render_actions(&actions);
                }
            }
        }

        if app.keybind_form.advanced {
            ui.label("动作 XML（可写多个 action）");
            ui.add(
                egui::TextEdit::multiline(&mut app.keybind_form.advanced_xml)
                    .code_editor()
                    .desired_rows(4)
                    .desired_width(f32::INFINITY),
            );
            if ui.button("从常用字段重新生成").clicked() {
                match build_common_actions(&app.keybind_form) {
                    Ok(actions) => {
                        app.keybind_form.advanced_xml = keybinds::render_actions(&actions)
                    }
                    Err(err) => app.keybind_form.error = Some(err),
                }
            }
        } else {
            ui.horizontal(|ui| {
                ui.label("动作名称");
                ui.add(
                    egui::TextEdit::singleline(&mut app.keybind_form.action)
                        .desired_width(180.0),
                );
                egui::ComboBox::from_id_source("action-suggest")
                    .selected_text("常用动作…")
                    .show_ui(ui, |ui| {
                        for name in ACTION_SUGGESTIONS {
                            let selected = app.keybind_form.action == *name;
                            if ui.selectable_label(selected, *name).clicked() {
                                app.keybind_form.action = (*name).to_string();
                            }
                        }
                    });
            });
            ui.horizontal(|ui| {
                ui.label("命令");
                ui.add(
                    egui::TextEdit::singleline(&mut app.keybind_form.command)
                        .hint_text("Execute 必填，例如 lxqt-qdbus run qterminal")
                        .desired_width(440.0),
                );
            });
            ui.horizontal(|ui| {
                ui.label("附加属性");
                ui.add(
                    egui::TextEdit::singleline(&mut app.keybind_form.attrs)
                        .hint_text("例如 direction=\"left\" combine=\"yes\"")
                        .desired_width(440.0),
                );
            });
            if app.keybind_form.editing_original.is_some()
                && app.keybind_form.advanced_xml.matches("<action").count() > 1
            {
                ui.label(
                    RichText::new(
                        "注意：该绑定原本包含多个动作，用常用模式提交只会保留第一个动作；建议切到高级 XML。",
                    )
                    .small()
                    .color(theme::WARN),
                );
            }
        }

        if let Ok(preview) = preview_for_form(&app.keybind_form) {
            egui::CollapsingHeader::new("XML 预览").show(ui, |ui| {
                ui.label(RichText::new(preview).monospace().small());
            });
        }

        if let Some(err) = &app.keybind_form.error {
            ui.colored_label(theme::DANGER, RichText::new(err).small());
        }

        if let Some(original) = &app.keybind_form.editing_original {
            ui.label(
                RichText::new(format!("正在编辑：{original}（键位上的其他属性会保留）"))
                    .small()
                    .weak(),
            );
        }

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let submit_label = if app.keybind_form.editing_original.is_some() {
                "加入/更新队列"
            } else {
                "加入队列"
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
                app.submit_keybind();
            }
            if ui.button("清空表单").clicked() {
                app.reset_keybind_form();
            }
            if app.keybind_form.editing_original.is_some() && ui.button("取消编辑").clicked() {
                app.reset_keybind_form();
            }
        });
    });

    ui.add_space(14.0);
    ui.label(RichText::new("待执行快捷键").strong());
    if app.keybind_edits.is_empty() && app.keybind_deletions.is_empty() {
        ui.label(
            RichText::new("队列为空。填写组合键和动作后加入，或从下方现有绑定里编辑/删除。")
                .small()
                .weak(),
        );
    } else {
        let edits = app.keybind_edits.clone();
        let deletions = app.keybind_deletions.clone();
        let mut discard: Option<crate::core::model::KeybindEdit> = None;

        for edit in &edits {
            let status = app.edit_status(edit);
            let failure = app.keybind_edit_failure(edit);
            let renamed = edit
                .original_key
                .as_ref()
                .map(|original| {
                    keybinds::normalize_key(original) != keybinds::normalize_key(&edit.key)
                })
                .unwrap_or(false);

            ui.horizontal(|ui| {
                ui.monospace(&edit.key);
                ui.label("→");
                ui.label(edit.summary());
                ui.colored_label(status_color(status), status.label());
                if renamed {
                    ui.label(
                        RichText::new(format!(
                            "原键 {}",
                            edit.original_key.clone().unwrap_or_default()
                        ))
                        .small()
                        .weak(),
                    );
                }
                if ui.small_button("移除").clicked() {
                    discard = Some(edit.clone());
                }
            });
            if let Some(err) = failure {
                ui.label(
                    RichText::new(format!("失败：{err}"))
                        .small()
                        .color(theme::DANGER),
                );
            }
        }

        if let Some(edit) = discard {
            app.discard_keybind_edit(&edit);
        }

        let mut undo: Option<String> = None;
        for key in &deletions {
            let failure = app.failures.get(&keybind_remove_id(key)).cloned();
            ui.horizontal(|ui| {
                ui.monospace(key);
                ui.colored_label(theme::DANGER, "待删除");
                if ui.small_button("撤销").clicked() {
                    undo = Some(key.clone());
                }
            });
            if let Some(err) = failure {
                ui.label(
                    RichText::new(format!("失败：{err}"))
                        .small()
                        .color(theme::DANGER),
                );
            }
        }
        if let Some(key) = undo {
            app.undo_delete_keybind(&key);
        }
    }

    ui.add_space(14.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("现有快捷键").strong());
        ui.add(
            egui::TextEdit::singleline(&mut app.keybind_filter)
                .hint_text("搜索按键 / 动作 / 命令")
                .desired_width(240.0),
        );
    });

    let filter = app.keybind_filter.trim().to_lowercase();
    let binds = app
        .baseline
        .as_ref()
        .map(|baseline| baseline.keybinds.clone())
        .unwrap_or_default();
    let filtered: Vec<&KeybindEntry> = binds
        .iter()
        .filter(|entry| {
            filter.is_empty()
                || entry.key.to_lowercase().contains(&filter)
                || entry.summary().to_lowercase().contains(&filter)
        })
        .collect();

    ui.label(
        RichText::new(format!(
            "显示 {} / {} 条",
            filtered.len(),
            binds.len()
        ))
        .small()
        .weak(),
    );

    if binds.is_empty() {
        ui.label(RichText::new("没有读取到快捷键。").weak());
        return;
    }

    let mut pending_action: Option<(BindAction, String)> = None;

    egui::ScrollArea::vertical()
        .max_height(300.0)
        .show(ui, |ui| {
            egui::Grid::new("existing-keybinds-full")
                .num_columns(4)
                .striped(true)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    for entry in filtered.iter().take(400) {
                        let mut key_label = entry.key.clone();
                        if !entry.attrs.is_empty() {
                            key_label.push_str(&format!(" [{}]", attrs_to_input(&entry.attrs)));
                        }
                        ui.monospace(key_label);
                        ui.label(entry.summary());

                        let pending_delete = app.is_pending_deletion(&entry.key);
                        let pending_edit = app.keybind_edits.iter().any(|edit| {
                            edit.original_key
                                .as_deref()
                                .map(keybinds::normalize_key)
                                .as_deref()
                                == Some(keybinds::normalize_key(&entry.key).as_str())
                        });
                        if pending_delete {
                            ui.colored_label(theme::DANGER, "待删除");
                        } else if pending_edit {
                            ui.colored_label(theme::WARN, "待更新");
                        } else {
                            ui.label("");
                        }

                        ui.horizontal(|ui| {
                            if pending_delete {
                                if ui.small_button("撤销删除").clicked() {
                                    pending_action =
                                        Some((BindAction::UndoDelete, entry.key.clone()));
                                }
                            } else {
                                if ui.small_button("编辑").clicked() {
                                    pending_action =
                                        Some((BindAction::Edit, entry.key.clone()));
                                }
                                if ui.small_button("删除").clicked() {
                                    pending_action =
                                        Some((BindAction::Delete, entry.key.clone()));
                                }
                            }
                        });
                        ui.end_row();
                    }
                });
        });

    if let Some((action, key)) = pending_action {
        match action {
            BindAction::Edit => app.load_keybind_for_edit(&key),
            BindAction::Delete => app.delete_keybind(&key),
            BindAction::UndoDelete => app.undo_delete_keybind(&key),
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
    keybinds::validate_action_name(&form.action)?;
    let attrs = keybinds::parse_attrs(&form.attrs)?;
    let command = form.command.trim().to_string();
    if form.action.trim() == "Execute" && command.is_empty() {
        return Err("Execute 动作必须填写命令".to_string());
    }
    Ok(vec![KeybindAction {
        name: form.action.trim().to_string(),
        command,
        attrs,
    }])
}

fn form_actions(form: &KeybindForm) -> Result<Vec<KeybindAction>, String> {
    if form.advanced {
        keybinds::parse_actions_fragment(&form.advanced_xml)
    } else {
        build_common_actions(form)
    }
}

fn preview_for_form(form: &KeybindForm) -> Result<String, String> {
    let key = keybinds::validate_combo(&form.key)?;
    let actions = form_actions(form)?;
    Ok(keybinds::render_keybind_preview(&key, &[], &actions))
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

fn soft_keyboard(app: &mut App, ui: &mut egui::Ui) {
    let mut key_value = app.keybind_form.key.clone();
    let mut changed = false;
    let (mods, _) = parse_parts(&key_value);

    ui.label(RichText::new("软键盘").strong());
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
        ui.label("主键");
        ui.selectable_value(&mut app.keybind_pick_tab, KeyPickTab::Letters, "字母");
        ui.selectable_value(&mut app.keybind_pick_tab, KeyPickTab::Digits, "数字");
        ui.selectable_value(&mut app.keybind_pick_tab, KeyPickTab::Function, "F 键");
        ui.selectable_value(&mut app.keybind_pick_tab, KeyPickTab::Navigation, "导航");
        ui.selectable_value(&mut app.keybind_pick_tab, KeyPickTab::Symbols, "符号");
        ui.selectable_value(&mut app.keybind_pick_tab, KeyPickTab::Media, "多媒体");
    });

    let keys = pick_keys(app.keybind_pick_tab);
    ui.horizontal_wrapped(|ui| {
        for (label, value) in keys {
            let selected = base.as_deref() == Some(*value);
            if key_button(ui, label, value, selected) {
                key_value = set_base_key(&key_value, value);
                changed = true;
            }
        }
    });

    if changed {
        app.keybind_form.key = key_value;
    }

    ui.label(
        RichText::new("点击修饰键和主键即可组合；上方输入框仍可手输任意 keysym。")
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
        RichText::new(label)
            .monospace()
            .color(egui::Color32::WHITE)
    } else {
        RichText::new(label).monospace()
    };
    let fill = if selected {
        theme::ACCENT
    } else {
        ui.visuals().widgets.inactive.bg_fill
    };
    ui.add_sized([66.0, 30.0], egui::Button::new(text).fill(fill))
        .on_hover_text(value)
        .clicked()
}

fn pick_keys(tab: KeyPickTab) -> &'static [(&'static str, &'static str)] {
    match tab {
        KeyPickTab::Letters => &LETTER_KEYS,
        KeyPickTab::Digits => &DIGIT_KEYS,
        KeyPickTab::Function => &FUNCTION_KEYS,
        KeyPickTab::Navigation => &NAVIGATION_KEYS,
        KeyPickTab::Symbols => &SYMBOL_KEYS,
        KeyPickTab::Media => &MEDIA_KEYS,
    }
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
    ("打印", "Print"),
    ("菜单", "Menu"),
    ("静音", "XF86AudioMute"),
    ("音量-", "XF86AudioLowerVolume"),
    ("音量+", "XF86AudioRaiseVolume"),
    ("播放", "XF86AudioPlay"),
    ("下一曲", "XF86AudioNext"),
    ("上一曲", "XF86AudioPrev"),
    ("亮度-", "XF86MonBrightnessDown"),
    ("亮度+", "XF86MonBrightnessUp"),
    ("电源", "XF86PowerOff"),
    ("睡眠", "XF86Sleep"),
    ("休眠", "XF86Suspend"),
    ("搜索", "XF86Search"),
    ("主页", "XF86HomePage"),
    ("邮件", "XF86Mail"),
    ("计算器", "XF86Calculator"),
    ("显示", "XF86Display"),
    ("触摸板", "XF86TouchpadToggle"),
    ("无线", "XF86WLAN"),
    ("电量", "XF86Battery"),
    ("录音", "XF86AudioRecord"),
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
