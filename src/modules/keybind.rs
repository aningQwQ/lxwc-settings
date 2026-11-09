use eframe::egui::{self, RichText};

use crate::app::{attrs_to_input, keybind_remove_id, App, KeyPickTab, KeybindForm, KeybindView};
use crate::core::keybinds;
use crate::core::model::{
    EditStatus, KeybindAction, KeybindEntry, ACTION_SUGGESTIONS,
};
use crate::i18n::t;
use crate::ui::theme;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let count = app
        .baseline
        .as_ref()
        .map(|baseline| baseline.keybinds.len())
        .unwrap_or(0);

    ui.horizontal(|ui| {
        ui.selectable_value(
            &mut app.keybind_view,
            KeybindView::Edit,
            t("编辑 / 新增", "Edit / Add"),
        );
        ui.selectable_value(
            &mut app.keybind_view,
            KeybindView::Existing,
            crate::trf!("现有快捷键（{count}）", "Existing shortcuts ({count})"),
        );
    });
    ui.separator();

    match app.keybind_view {
        KeybindView::Edit => {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| edit_view(app, ui));
        }
        KeybindView::Existing => existing_view(app, ui),
    }
}

fn edit_view(app: &mut App, ui: &mut egui::Ui) {
    ui.label(
        RichText::new(t(
            "直接填写 labwc keysym 组合与动作；也可以到“现有快捷键”页点编辑载入。",
            "Type a labwc keysym combination and action, or load one from the Existing shortcuts tab.",
        ))
        .small()
        .weak(),
    );
    if let Some(baseline) = &app.baseline {
        if !baseline.rc_exists {
            ui.colored_label(
                theme::DANGER,
                crate::trf!(
                    "未找到 {}，快捷键无法写入。",
                    "{} was not found; key bindings cannot be written.",
                    baseline.rc_path.display()
                ),
            );
        }
    }

    ui.add_space(8.0);
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(t("按键组合", "Key combination"));
            ui.add(
                egui::TextEdit::singleline(&mut app.keybind_form.key)
                    .hint_text(t(
                        "例如 W-C-S-t、W-Return、Print、XF86AudioMute",
                        "e.g. W-C-S-t, W-Return, Print, XF86AudioMute",
                    ))
                    .desired_width(320.0)
                    .font(egui::TextStyle::Monospace),
            );
        });
        ui.label(
            RichText::new(t(
                "修饰键用 W / C / A / S；主键写 XKB keysym 名称。常见：Return、space、Left、Print、F12、XF86AudioMute。labwc 不支持多段按键序列。",
                "Use W / C / A / S for modifiers and an XKB keysym name for the key. Common keysyms: Return, space, Left, Print, F12, XF86AudioMute. labwc does not support multi-step key sequences.",
            ))
            .small()
            .weak(),
        );

        ui.add_space(8.0);
        soft_keyboard(app, ui);
        ui.add_space(6.0);
        let mut advanced = app.keybind_form.advanced;
        ui.horizontal(|ui| {
            ui.label(t("动作模式", "Action mode"));
            ui.selectable_value(&mut advanced, false, t("常用", "Common"));
            ui.selectable_value(&mut advanced, true, t("高级 XML", "Advanced XML"));
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
            ui.label(t("动作 XML（可写多个 action）", "Action XML (multiple actions allowed)"));
            ui.add(
                egui::TextEdit::multiline(&mut app.keybind_form.advanced_xml)
                    .code_editor()
                    .desired_rows(4)
                    .desired_width(f32::INFINITY),
            );
            if ui.button(t("从常用字段重新生成", "Regenerate from common fields")).clicked() {
                match build_common_actions(&app.keybind_form) {
                    Ok(actions) => {
                        app.keybind_form.advanced_xml = keybinds::render_actions(&actions)
                    }
                    Err(err) => app.keybind_form.error = Some(err),
                }
            }
        } else {
            ui.horizontal(|ui| {
                ui.label(t("动作名称", "Action name"));
                ui.add(
                    egui::TextEdit::singleline(&mut app.keybind_form.action)
                        .desired_width(180.0),
                );
                egui::ComboBox::from_id_source("action-suggest")
                    .selected_text(t("常用动作…", "Common actions…"))
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
                ui.label(t("命令", "Command"));
                ui.add(
                    egui::TextEdit::singleline(&mut app.keybind_form.command)
                        .hint_text(t(
                            "Execute 必填，例如 lxqt-qdbus run qterminal",
                            "Required for Execute, e.g. lxqt-qdbus run qterminal",
                        ))
                        .desired_width(440.0),
                );
            });
            ui.horizontal(|ui| {
                ui.label(t("附加属性", "Extra attributes"));
                ui.add(
                    egui::TextEdit::singleline(&mut app.keybind_form.attrs)
                        .hint_text(t(
                            "例如 direction=\"left\" combine=\"yes\"",
                            "e.g. direction=\"left\" combine=\"yes\"",
                        ))
                        .desired_width(440.0),
                );
            });
            if app.keybind_form.editing_original.is_some()
                && app.keybind_form.advanced_xml.matches("<action").count() > 1
            {
                ui.label(
                    RichText::new(t(
                        "注意：该绑定原本包含多个动作，用常用模式提交只会保留第一个动作；建议切到高级 XML。",
                        "Note: this binding contains multiple actions. Submitting in Common mode keeps only the first one; use Advanced XML instead.",
                    ))
                    .small()
                    .color(theme::WARN),
                );
            }
        }

        if let Ok(preview) = preview_for_form(&app.keybind_form) {
            egui::CollapsingHeader::new(t("XML 预览", "XML preview")).show(ui, |ui| {
                ui.label(RichText::new(preview).monospace().small());
            });
        }

        if let Some(err) = &app.keybind_form.error {
            ui.colored_label(theme::DANGER, RichText::new(err).small());
        }

        if let Some(original) = &app.keybind_form.editing_original {
            ui.label(
                RichText::new(crate::trf!(
                    "正在编辑：{original}（键位上的其他属性会保留）",
                    "Editing: {original} (other keybind attributes are preserved)",
                ))
                    .small()
                    .weak(),
            );
        }

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let submit_label = if app.keybind_form.editing_original.is_some() {
                t("加入/更新队列", "Add / update queue")
            } else {
                t("加入队列", "Add to queue")
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
            if ui.button(t("清空表单", "Clear form")).clicked() {
                app.reset_keybind_form();
            }
            if app.keybind_form.editing_original.is_some() && ui.button(t("取消编辑", "Cancel editing")).clicked() {
                app.reset_keybind_form();
            }
        });
    });

    ui.add_space(14.0);
    ui.label(RichText::new(t("待执行快捷键", "Pending shortcuts")).strong());
    if app.keybind_edits.is_empty() && app.keybind_deletions.is_empty() {
        ui.label(
            RichText::new(t(
                "队列为空。填写组合键和动作后加入，或到“现有快捷键”页编辑/删除。",
                "The queue is empty. Fill in a key combination and action, or edit/delete from the Existing shortcuts tab.",
            ))
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
                        RichText::new(crate::trf!(
                            "原键 {}",
                            "From {}",
                            edit.original_key.clone().unwrap_or_default()
                        ))
                        .small()
                        .weak(),
                    );
                }
                if ui.small_button(t("移除", "Remove")).clicked() {
                    discard = Some(edit.clone());
                }
            });
            if let Some(err) = failure {
                ui.label(
                    RichText::new(crate::trf!("失败：{err}", "Failed: {err}"))
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
                ui.colored_label(theme::DANGER, t("待删除", "Pending delete"));
                if ui.small_button(t("撤销", "Undo")).clicked() {
                    undo = Some(key.clone());
                }
            });
            if let Some(err) = failure {
                ui.label(
                    RichText::new(crate::trf!("失败：{err}", "Failed: {err}"))
                        .small()
                        .color(theme::DANGER),
                );
            }
        }
        if let Some(key) = undo {
            app.undo_delete_keybind(&key);
        }
    }
}

fn existing_view(app: &mut App, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut app.keybind_filter)
                .hint_text(t("搜索按键 / 动作 / 命令", "Search key / action / command"))
                .desired_width(320.0),
        );
        if !app.keybind_filter.is_empty() && ui.button(t("清除", "Clear")).clicked() {
            app.keybind_filter.clear();
        }
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
        RichText::new(crate::trf!(
            "显示 {} / {} 条 · 拖动表头分隔线调整列宽 · 点击“编辑”载入编辑页",
            "Showing {} / {} · Drag header dividers to resize · Click Edit to load",
            filtered.len(),
            binds.len()
        ))
        .small()
        .weak(),
    );

    if binds.is_empty() {
        ui.label(RichText::new(t("没有读取到快捷键。", "No key bindings found.")).weak());
        return;
    }
    if filtered.is_empty() {
        ui.label(RichText::new(t("没有匹配的快捷键。", "No matching key bindings.")).weak());
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
                ui.strong(t("按键", "Key"));
            });
            header.col(|ui| {
                ui.strong(t("动作 / 命令", "Action / Command"));
            });
            header.col(|ui| {
                ui.strong(t("状态", "Status"));
            });
            header.col(|ui| {
                ui.strong(t("操作", "Actions"));
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
                    ui.add(
                        egui::Label::new(RichText::new(&key_label).monospace()).truncate(),
                    )
                    .on_hover_text(key_label);
                });

                row.col(|ui| {
                    let summary = entry.summary();
                    ui.add(egui::Label::new(summary.clone()).truncate())
                        .on_hover_text(summary);
                });

                row.col(|ui| {
                    let pending_delete = app.is_pending_deletion(&entry.key);
                    let pending_edit = app.keybind_edits.iter().any(|edit| {
                        edit.original_key
                            .as_deref()
                            .map(keybinds::normalize_key)
                            .as_deref()
                            == Some(keybinds::normalize_key(&entry.key).as_str())
                    });
                    if pending_delete {
                        ui.colored_label(theme::DANGER, t("待删除", "Pending delete"));
                    } else if pending_edit {
                        ui.colored_label(theme::WARN, t("待更新", "Pending update"));
                    }
                });

                row.col(|ui| {
                    if app.is_pending_deletion(&entry.key) {
                        if ui.small_button(t("撤销删除", "Undo delete")).clicked() {
                            pending_action =
                                Some((BindAction::UndoDelete, entry.key.clone()));
                        }
                    } else {
                        if ui.small_button(t("编辑", "Edit")).clicked() {
                            pending_action = Some((BindAction::Edit, entry.key.clone()));
                        }
                        if ui.small_button(t("删除", "Delete")).clicked() {
                            pending_action =
                                Some((BindAction::Delete, entry.key.clone()));
                        }
                    }
                });
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
        return Err(t("Execute 动作必须填写命令", "The Execute action requires a command").to_string());
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

    ui.label(RichText::new(t("软键盘", "Soft keyboard")).strong());
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
        ui.label(t("主键", "Key"));
        ui.selectable_value(&mut app.keybind_pick_tab, KeyPickTab::Letters, t("字母", "Letters"));
        ui.selectable_value(&mut app.keybind_pick_tab, KeyPickTab::Digits, t("数字", "Digits"));
        ui.selectable_value(&mut app.keybind_pick_tab, KeyPickTab::Function, t("F 键", "F keys"));
        ui.selectable_value(&mut app.keybind_pick_tab, KeyPickTab::Navigation, t("导航", "Navigation"));
        ui.selectable_value(&mut app.keybind_pick_tab, KeyPickTab::Symbols, t("符号", "Symbols"));
        ui.selectable_value(&mut app.keybind_pick_tab, KeyPickTab::Media, t("多媒体", "Media"));
    });

    let keys = pick_keys(app.keybind_pick_tab);
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
        app.keybind_form.key = key_value;
    }

    ui.label(
        RichText::new(t(
            "点击修饰键和主键即可组合；上方输入框仍可手输任意 keysym。",
            "Click modifiers and a key to build the combination; the field above still accepts any keysym.",
        ))
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
                .map(|(zh, en, value)| (t(zh, en).to_string(), *value))
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

const MEDIA_KEYS: [(&str, &str, &str); 22] = [
    ("打印", "Print", "Print"),
    ("菜单", "Menu", "Menu"),
    ("静音", "Mute", "XF86AudioMute"),
    ("音量-", "Vol-", "XF86AudioLowerVolume"),
    ("音量+", "Vol+", "XF86AudioRaiseVolume"),
    ("播放", "Play", "XF86AudioPlay"),
    ("下一曲", "Next", "XF86AudioNext"),
    ("上一曲", "Prev", "XF86AudioPrev"),
    ("亮度-", "Bri-", "XF86MonBrightnessDown"),
    ("亮度+", "Bri+", "XF86MonBrightnessUp"),
    ("电源", "Power", "XF86PowerOff"),
    ("睡眠", "Sleep", "XF86Sleep"),
    ("休眠", "Susp", "XF86Suspend"),
    ("搜索", "Search", "XF86Search"),
    ("主页", "Home", "XF86HomePage"),
    ("邮件", "Mail", "XF86Mail"),
    ("计算器", "Calc", "XF86Calculator"),
    ("显示", "Disp", "XF86Display"),
    ("触摸板", "Touch", "XF86TouchpadToggle"),
    ("无线", "WiFi", "XF86WLAN"),
    ("电量", "Batt", "XF86Battery"),
    ("录音", "Rec", "XF86AudioRecord"),
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
