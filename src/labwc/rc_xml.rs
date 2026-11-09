use quick_xml::escape::unescape;
use quick_xml::events::{BytesStart, BytesText, Event};
use quick_xml::Reader;

use crate::i18n::t;
use crate::labwc::actions::known_action_label;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeybindAction {
    pub name: String,
    pub command: String,
    pub attrs: Vec<(String, String)>,
}

impl KeybindAction {
    pub fn summary(&self) -> String {
        let label = known_action_label(&self.name).unwrap_or(&self.name);
        if self.name == "Execute" && !self.command.is_empty() {
            return format!("{label}: {}", self.command);
        }
        if self.attrs.is_empty() {
            label.to_string()
        } else {
            let attrs = self
                .attrs
                .iter()
                .map(|(key, value)| format!("{key}=\"{value}\""))
                .collect::<Vec<_>>()
                .join(" ");
            format!("{label} {attrs}")
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeybindEntry {
    pub key: String,
    pub attrs: Vec<(String, String)>,
    pub actions: Vec<KeybindAction>,
}

impl KeybindEntry {
    pub fn summary(&self) -> String {
        match self.actions.len() {
            0 => t("no_action").to_string(),
            1 => self.actions[0].summary(),
            count => crate::trf!("actions", self.actions[0].summary(), count),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Addition {
    pub key: String,
    pub attrs: Vec<(String, String)>,
    pub actions: Vec<KeybindAction>,
}

pub fn normalize_key(key: &str) -> String {
    let mut sup = false;
    let mut ctrl = false;
    let mut alt = false;
    let mut shift = false;
    let mut base = Vec::new();

    for part in key.split('-') {
        match part {
            "W" => sup = true,
            "C" => ctrl = true,
            "A" => alt = true,
            "S" => shift = true,
            _ => base.push(part),
        }
    }

    let mut out = String::new();
    for (enabled, label) in [(sup, 'W'), (ctrl, 'C'), (alt, 'A'), (shift, 'S')] {
        if enabled {
            out.push(label);
            out.push('-');
        }
    }
    out.push_str(&base.join("-"));
    out
}

pub fn validate_combo(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(crate::i18n::t("enter_a_key_combination_e_g_w_c_s_t").to_string());
    }
    if trimmed.chars().any(char::is_whitespace) {
        return Err(crate::i18n::t("the_key_combination_cannot_contain_spaces").to_string());
    }

    let mut sup = false;
    let mut ctrl = false;
    let mut alt = false;
    let mut shift = false;
    let mut bases = Vec::new();

    for part in trimmed.split('-') {
        if part.is_empty() {
            return Err(crate::i18n::t("the_combination_contains_an_empty_segment").to_string());
        }
        match part {
            "W" => sup = true,
            "C" => ctrl = true,
            "A" => alt = true,
            "S" => shift = true,
            _ => bases.push(part),
        }
    }

    if bases.len() != 1 {
        return Err(crate::i18n::t(
            "exactly_one_main_key_is_required_use_uppercase_w_c_a_s_for_modifiers",
        )
        .to_string());
    }

    let base = bases[0];
    if !base
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        return Err(crate::i18n::t(
            "the_main_key_must_be_an_xkb_keysym_name_letters_digits_underscores",
        )
        .to_string());
    }

    let mut out = String::new();
    for (enabled, label) in [(sup, 'W'), (ctrl, 'C'), (alt, 'A'), (shift, 'S')] {
        if enabled {
            out.push(label);
            out.push('-');
        }
    }
    out.push_str(base);
    Ok(out)
}

pub fn validate_action_name(name: &str) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(crate::i18n::t("action_name_cannot_be_empty").to_string());
    }
    if !trimmed
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        return Err(
            crate::i18n::t("action_name_may_only_contain_letters_digits_and_underscores")
                .to_string(),
        );
    }
    Ok(())
}

pub fn parse_attrs(input: &str) -> Result<Vec<(String, String)>, String> {
    let mut rest = input.trim();
    let mut attrs: Vec<(String, String)> = Vec::new();

    while !rest.is_empty() {
        rest = rest
            .trim_start_matches(|character: char| character.is_whitespace() || character == ',');
        if rest.is_empty() {
            break;
        }

        let name_end = rest
            .find(|character: char| {
                character == '=' || character.is_whitespace() || character == ','
            })
            .ok_or_else(|| crate::i18n::t("attributes_must_use_key_value").to_string())?;
        let name = rest[..name_end].trim();
        if !is_valid_attr_name(name) {
            return Err(crate::trf!("invalid_attribute_name", name));
        }
        if name == "name" || name == "command" {
            return Err(crate::i18n::t("enter_name_command_in_their_dedicated_fields").to_string());
        }

        rest = rest[name_end..].trim_start();
        let Some(after_eq) = rest.strip_prefix('=') else {
            return Err(crate::trf!("attribute_is_missing", name));
        };
        rest = after_eq.trim_start();

        let value;
        if rest.starts_with('"') || rest.starts_with('\'') {
            let quote = rest.chars().next().unwrap();
            let body = &rest[1..];
            let end = body
                .find(quote)
                .ok_or_else(|| crate::trf!("attribute_has_an_unclosed_quote", name))?;
            value = body[..end].to_string();
            rest = &body[end + 1..];
        } else {
            let end = rest
                .find(|character: char| character.is_whitespace() || character == ',')
                .unwrap_or(rest.len());
            value = rest[..end].to_string();
            rest = &rest[end..];
        }

        if attrs.iter().any(|(existing, _)| existing == name) {
            return Err(crate::trf!("attribute_is_duplicated", name));
        }
        attrs.push((name.to_string(), value));
    }

    Ok(attrs)
}

pub fn parse_actions_fragment(fragment: &str) -> Result<Vec<KeybindAction>, String> {
    let wrapped = format!("<keyboard><keybind key=\"W-F24\">{fragment}</keybind></keyboard>");
    let entries = parse(&wrapped).map_err(|err| crate::trf!("failed_to_parse_action_xml", err))?;
    if entries.len() != 1 {
        return Err(crate::i18n::t(
            "only_action_elements_are_allowed_extra_keybind_keyboard_elements_are_not",
        )
        .to_string());
    }
    let actions = entries[0].actions.clone();
    if actions.is_empty() {
        return Err(crate::i18n::t("at_least_one_action_is_required").to_string());
    }
    for action in &actions {
        validate_action_name(&action.name)?;
    }
    Ok(actions)
}

pub fn render_action(action: &KeybindAction) -> String {
    let mut out = String::from("<action name=\"");
    out.push_str(&escape_xml(&action.name));
    out.push('"');
    for (key, value) in &action.attrs {
        if key == "name" || key == "command" {
            continue;
        }
        out.push(' ');
        out.push_str(key);
        out.push_str("=\"");
        out.push_str(&escape_xml(value));
        out.push('"');
    }
    if !action.command.is_empty() {
        out.push_str(" command=\"");
        out.push_str(&escape_xml(&action.command));
        out.push('"');
    }
    out.push_str("/>");
    out
}

pub fn render_actions(actions: &[KeybindAction]) -> String {
    actions
        .iter()
        .map(render_action)
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn render_keybind_preview(
    key: &str,
    attrs: &[(String, String)],
    actions: &[KeybindAction],
) -> String {
    let mut out = String::from("<keybind key=\"");
    out.push_str(&escape_xml(key));
    out.push('"');
    for (name, value) in attrs {
        if name == "key" {
            continue;
        }
        out.push(' ');
        out.push_str(name);
        out.push_str("=\"");
        out.push_str(&escape_xml(value));
        out.push('"');
    }
    out.push_str(">\n");
    for action in actions {
        out.push_str("  ");
        out.push_str(&render_action(action));
        out.push('\n');
    }
    out.push_str("</keybind>");
    out
}

pub fn parse(raw: &str) -> Result<Vec<KeybindEntry>, String> {
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }

    let mut reader = Reader::from_str(raw);
    reader.config_mut().trim_text(false);

    let mut depth = 0usize;
    let mut keyboard_depth: Option<usize> = None;
    let mut current: Option<PendingKeybind> = None;
    let mut current_action: Option<PendingAction> = None;
    let mut entries = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) => {
                depth += 1;
                let name = event.local_name().as_ref().to_vec();
                if name == b"keyboard" && keyboard_depth.is_none() {
                    keyboard_depth = Some(depth);
                    continue;
                }
                if keyboard_depth.is_none() {
                    continue;
                }
                match name.as_slice() {
                    b"keybind" => {
                        let (key, attrs) = keybind_attrs(&event);
                        current = Some(PendingKeybind {
                            key,
                            attrs,
                            actions: Vec::new(),
                        });
                    }
                    b"action" if current.is_some() => {
                        let (name, command, attrs) = action_attrs(&event);
                        current_action = Some(PendingAction {
                            name,
                            command,
                            attrs,
                            in_command: false,
                            command_text: String::new(),
                        });
                    }
                    b"command" if current_action.is_some() => {
                        if let Some(action) = current_action.as_mut() {
                            action.in_command = true;
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(event)) => {
                let name = event.local_name().as_ref().to_vec();
                if keyboard_depth.is_none() {
                    continue;
                }
                match name.as_slice() {
                    b"keybind" => {
                        let (key, attrs) = keybind_attrs(&event);
                        entries.push(KeybindEntry {
                            key,
                            attrs,
                            actions: Vec::new(),
                        });
                    }
                    b"action" if current.is_some() => {
                        let (name, command, attrs) = action_attrs(&event);
                        if let Some(keybind) = current.as_mut() {
                            keybind.actions.push(KeybindAction {
                                name,
                                command,
                                attrs,
                            });
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(text)) => {
                if let Some(action) = current_action.as_mut() {
                    if action.in_command {
                        action.command_text.push_str(&text_content(&text));
                    }
                }
            }
            Ok(Event::CData(data)) => {
                if let Some(action) = current_action.as_mut() {
                    if action.in_command {
                        action
                            .command_text
                            .push_str(&String::from_utf8_lossy(&data));
                    }
                }
            }
            Ok(Event::End(event)) => {
                let name = event.local_name().as_ref().to_vec();
                if keyboard_depth.is_some() {
                    match name.as_slice() {
                        b"command" => {
                            if let Some(action) = current_action.as_mut() {
                                action.in_command = false;
                            }
                        }
                        b"action" => {
                            if let Some(mut action) = current_action.take() {
                                if action.command.is_empty()
                                    && !action.command_text.trim().is_empty()
                                {
                                    action.command = action.command_text.trim().to_string();
                                }
                                if let Some(keybind) = current.as_mut() {
                                    keybind.actions.push(KeybindAction {
                                        name: action.name,
                                        command: action.command,
                                        attrs: action.attrs,
                                    });
                                }
                            }
                        }
                        b"keybind" => {
                            if let Some(keybind) = current.take() {
                                entries.push(KeybindEntry {
                                    key: keybind.key,
                                    attrs: keybind.attrs,
                                    actions: keybind.actions,
                                });
                            }
                        }
                        b"keyboard" if Some(depth) == keyboard_depth => {
                            keyboard_depth = None;
                        }
                        _ => {}
                    }
                }
                depth = depth.saturating_sub(1);
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(err) => return Err(crate::trf!("failed_to_parse_rc_xml", err)),
        }
    }

    Ok(entries)
}

pub fn patch(raw: &str, additions: &[Addition], remove_keys: &[String]) -> Result<String, String> {
    if raw.trim().is_empty() {
        return Err(crate::i18n::t("rc_xml_is_empty_cannot_write_key_bindings").to_string());
    }
    if additions.is_empty() && remove_keys.is_empty() {
        return Ok(raw.to_string());
    }
    if !raw.contains("</keyboard>") {
        return Err(crate::i18n::t("no_keyboard_node_found_in_rc_xml").to_string());
    }
    for addition in additions {
        if addition.actions.is_empty() {
            return Err(crate::trf!("shortcut_has_no_action", addition.key));
        }
    }

    let mut reader = Reader::from_str(raw);
    reader.config_mut().trim_text(false);

    let mut depth = 0usize;
    let mut keyboard_depth: Option<usize> = None;
    let mut skip_depth: Option<usize> = None;
    let mut removals: Vec<(usize, usize)> = Vec::new();

    loop {
        let start = reader.buffer_position() as usize;
        match reader.read_event() {
            Ok(Event::Start(event)) => {
                depth += 1;
                let name = event.local_name().as_ref().to_vec();
                if name == b"keyboard" && keyboard_depth.is_none() {
                    keyboard_depth = Some(depth);
                } else if keyboard_depth.is_some() && skip_depth.is_none() && name == b"keybind" {
                    if let Some(key) = attr(&event, b"key") {
                        if remove_keys
                            .iter()
                            .any(|target| normalize_key(target) == normalize_key(&key))
                        {
                            skip_depth = Some(depth);
                            let end = reader.buffer_position() as usize;
                            removals.push((extend_start(raw, start), end));
                        }
                    }
                }
            }
            Ok(Event::Empty(event)) => {
                let name = event.local_name().as_ref().to_vec();
                if keyboard_depth.is_some() && name == b"keybind" {
                    if let Some(key) = attr(&event, b"key") {
                        if remove_keys
                            .iter()
                            .any(|target| normalize_key(target) == normalize_key(&key))
                        {
                            let end = reader.buffer_position() as usize;
                            removals.push((extend_start(raw, start), end));
                        }
                    }
                }
            }
            Ok(Event::End(event)) => {
                let name = event.local_name().as_ref().to_vec();
                if skip_depth == Some(depth) && name == b"keybind" {
                    let end = reader.buffer_position() as usize;
                    if let Some(last) = removals.last_mut() {
                        last.1 = end;
                    }
                    skip_depth = None;
                }
                if Some(depth) == keyboard_depth && name == b"keyboard" {
                    keyboard_depth = None;
                }
                depth = depth.saturating_sub(1);
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(err) => return Err(crate::trf!("failed_to_parse_rc_xml", err)),
        }
    }

    let mut out = String::with_capacity(raw.len() + additions.len() * 128);
    let mut cursor = 0usize;
    for (start, end) in removals {
        if start < cursor || end > raw.len() || start >= end {
            continue;
        }
        out.push_str(&raw[cursor..start]);
        cursor = end;
    }
    out.push_str(&raw[cursor..]);

    if !additions.is_empty() {
        let close_pos = out
            .rfind("</keyboard>")
            .ok_or_else(|| crate::i18n::t("no_keyboard_node_found_in_rc_xml").to_string())?;
        let indent = detect_indent(raw);
        let close_indent = detect_close_indent(raw);
        let (insert_at, leading_newline) = match out[..close_pos].rfind('\n') {
            Some(index) => (index + 1, false),
            None => (close_pos, true),
        };

        let mut block = String::new();
        if leading_newline {
            block.push('\n');
        }
        for addition in additions {
            block.push_str(&indent);
            block.push_str("<keybind key=\"");
            block.push_str(&escape_xml(&addition.key));
            block.push('"');
            for (name, value) in &addition.attrs {
                if name == "key" {
                    continue;
                }
                block.push(' ');
                block.push_str(name);
                block.push_str("=\"");
                block.push_str(&escape_xml(value));
                block.push('"');
            }
            block.push_str(">\n");
            for action in &addition.actions {
                block.push_str(&indent);
                block.push_str("  ");
                block.push_str(&render_action(action));
                block.push('\n');
            }
            block.push_str(&indent);
            block.push_str("</keybind>\n");
        }
        block.push_str(&close_indent);
        out.replace_range(insert_at..close_pos, &block);
    }

    parse(&out).map_err(|err| {
        crate::trf!(
            "the_patched_rc_xml_could_not_be_parsed_changes_were_cancelled",
            err
        )
    })?;
    Ok(out)
}

struct PendingKeybind {
    key: String,
    attrs: Vec<(String, String)>,
    actions: Vec<KeybindAction>,
}

struct PendingAction {
    name: String,
    command: String,
    attrs: Vec<(String, String)>,
    in_command: bool,
    command_text: String,
}

fn keybind_attrs(event: &BytesStart) -> (String, Vec<(String, String)>) {
    let mut key = String::new();
    let mut attrs = Vec::new();
    for attribute in event.attributes().flatten() {
        let local = attribute.key.local_name().as_ref().to_vec();
        let Ok(name) = String::from_utf8(local) else {
            continue;
        };
        let Ok(value) = attribute.normalized_value(quick_xml::XmlVersion::Implicit1_0) else {
            continue;
        };
        if name == "key" {
            key = value.into_owned();
        } else {
            attrs.push((name, value.into_owned()));
        }
    }
    (key, attrs)
}

fn action_attrs(event: &BytesStart) -> (String, String, Vec<(String, String)>) {
    let mut name = String::new();
    let mut command = String::new();
    let mut attrs = Vec::new();
    for attribute in event.attributes().flatten() {
        let local = attribute.key.local_name().as_ref().to_vec();
        let Ok(key) = String::from_utf8(local) else {
            continue;
        };
        let Ok(value) = attribute.normalized_value(quick_xml::XmlVersion::Implicit1_0) else {
            continue;
        };
        let value = value.into_owned();
        if key == "name" {
            name = value;
        } else if key == "command" {
            command = value;
        } else {
            attrs.push((key, value));
        }
    }
    (name, command, attrs)
}

fn attr(event: &BytesStart, name: &[u8]) -> Option<String> {
    event
        .attributes()
        .flatten()
        .find(|attribute| {
            attribute.key.as_ref() == name || attribute.key.local_name().as_ref() == name
        })
        .and_then(|attribute| {
            attribute
                .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                .ok()
                .map(|value| value.into_owned())
        })
}

fn text_content(text: &BytesText) -> String {
    let decoded = text
        .xml10_content()
        .map(|content| content.into_owned())
        .unwrap_or_default();
    unescape(&decoded)
        .map(|content| content.into_owned())
        .unwrap_or(decoded)
}

fn extend_start(raw: &str, mut start: usize) -> usize {
    let bytes = raw.as_bytes();
    while start > 0 && matches!(bytes[start - 1], b' ' | b'\t') {
        start -= 1;
    }
    if start > 0 && bytes[start - 1] == b'\n' {
        start -= 1;
        if start > 0 && bytes[start - 1] == b'\r' {
            start -= 1;
        }
    }
    start
}

fn detect_indent(raw: &str) -> String {
    for line in raw.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("<keybind") {
            return " ".repeat(line.len() - trimmed.len());
        }
    }
    "    ".to_string()
}

fn detect_close_indent(raw: &str) -> String {
    for line in raw.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("</keyboard>") {
            return " ".repeat(line.len() - trimmed.len());
        }
    }
    "  ".to_string()
}

fn is_valid_attr_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_' || first == ':') {
        return false;
    }
    chars.all(|character| {
        character.is_ascii_alphanumeric()
            || character == '_'
            || character == '-'
            || character == ':'
            || character == '.'
    })
}

fn escape_xml(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(character),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use quick_xml::events::Event as XmlEvent;
    use quick_xml::Reader as XmlReader;

    const RC: &str = r#"<?xml version="1.0"?>
<labwc_config>
  <core>
    <decoration>server</decoration>
  </core>
  <keyboard>
    <keybind key="W-p">
      <action name="Execute" command="pcmanfm-qt" />
      <action name="SnapToEdge" direction="left" combine="yes" />
    </keybind>
    <keybind key="A-F2" layout="us">
      <action name="Execute">
        <command>lxqt-runner</command>
      </action>
    </keybind>
  </keyboard>
</labwc_config>
"#;

    fn action(name: &str, command: &str) -> KeybindAction {
        KeybindAction {
            name: name.to_string(),
            command: command.to_string(),
            attrs: Vec::new(),
        }
    }

    #[test]
    fn parses_actions_attributes_and_commands() {
        let entries = parse(RC).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].key, "W-p");
        assert_eq!(entries[0].actions.len(), 2);
        assert_eq!(entries[0].actions[0].command, "pcmanfm-qt");
        assert_eq!(
            entries[0].actions[1].attrs,
            vec![
                ("direction".to_string(), "left".to_string()),
                ("combine".to_string(), "yes".to_string())
            ]
        );
        assert_eq!(entries[1].key, "A-F2");
        assert_eq!(
            entries[1].attrs,
            vec![("layout".to_string(), "us".to_string())]
        );
        assert_eq!(entries[1].actions[0].command, "lxqt-runner");
    }

    #[test]
    fn appends_keybind_with_actions() {
        let patched = patch(
            RC,
            &[Addition {
                key: "W-C-S-t".to_string(),
                attrs: Vec::new(),
                actions: vec![
                    action("Execute", "qterminal"),
                    KeybindAction {
                        name: "Reconfigure".to_string(),
                        command: String::new(),
                        attrs: Vec::new(),
                    },
                ],
            }],
            &[],
        )
        .unwrap();
        assert!(patched.contains("<keybind key=\"W-C-S-t\">"));
        assert!(patched.contains("<action name=\"Execute\" command=\"qterminal\"/>"));
        assert!(patched.contains("<action name=\"Reconfigure\"/>"));
        assert!(patched.ends_with("</labwc_config>\n"));
        assert_eq!(parse(&patched).unwrap().len(), 3);
    }

    #[test]
    fn replaces_keybind_and_preserves_others() {
        let patched = patch(
            RC,
            &[Addition {
                key: "W-p".to_string(),
                attrs: Vec::new(),
                actions: vec![KeybindAction {
                    name: "Execute".to_string(),
                    command: "qterminal".to_string(),
                    attrs: vec![("foo".to_string(), "bar".to_string())],
                }],
            }],
            &["W-p".to_string()],
        )
        .unwrap();
        assert_eq!(patched.matches("key=\"W-p\"").count(), 1);
        assert!(patched.contains("foo=\"bar\""));
        assert!(!patched.contains("pcmanfm-qt"));
        assert!(patched.contains("A-F2"));
    }

    #[test]
    fn removes_keybind_only() {
        let patched = patch(RC, &[], &["A-F2".to_string()]).unwrap();
        assert!(!patched.contains("A-F2"));
        assert!(patched.contains("W-p"));
        assert_eq!(parse(&patched).unwrap().len(), 1);
    }

    #[test]
    fn validates_combinations() {
        assert_eq!(validate_combo("S-W-p").unwrap(), "W-S-p");
        assert_eq!(validate_combo("W-C-A-S-t").unwrap(), "W-C-A-S-t");
        assert_eq!(validate_combo("Print").unwrap(), "Print");
        assert_eq!(validate_combo("W-Return").unwrap(), "W-Return");
        assert!(validate_combo("W-C-t-X").is_err());
        assert!(validate_combo("W-").is_err());
        assert!(validate_combo("W-Left-x").is_err());
        assert!(validate_combo("W-C-").is_err());
    }

    #[test]
    fn parses_extra_attributes() {
        let attrs = parse_attrs("direction=\"left\" combine=yes, foo='a b'").unwrap();
        assert_eq!(attrs[0], ("direction".to_string(), "left".to_string()));
        assert_eq!(attrs[1], ("combine".to_string(), "yes".to_string()));
        assert_eq!(attrs[2], ("foo".to_string(), "a b".to_string()));
        assert!(parse_attrs("name=x").is_err());
        assert!(parse_attrs("bad-name=x").is_ok());
    }

    #[test]
    fn parses_action_fragments() {
        let actions = parse_actions_fragment(
            "<action name=\"Execute\" command=\"qterminal\"/>\n<action name=\"Close\"/>",
        )
        .unwrap();
        assert_eq!(actions.len(), 2);
        assert!(parse_actions_fragment(
            "<action name=\"Execute\" command=\"x\"/></keybind><keybind key=\"W-q\">"
        )
        .is_err());
        assert!(parse_actions_fragment("").is_err());
    }

    #[test]
    fn renders_round_trip() {
        let rendered = render_actions(&[
            action("Execute", "lxqt-runner"),
            KeybindAction {
                name: "SnapToEdge".to_string(),
                command: String::new(),
                attrs: vec![("direction".to_string(), "left".to_string())],
            },
        ]);
        let parsed = parse_actions_fragment(&rendered).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[1].attrs[0].0, "direction");
    }

    #[test]
    fn patch_output_is_well_formed_xml() {
        let patched = patch(
            RC,
            &[Addition {
                key: "W-Q".to_string(),
                attrs: Vec::new(),
                actions: vec![action("Execute", "qterminal")],
            }],
            &["W-p".to_string()],
        )
        .unwrap();
        let mut reader = XmlReader::from_str(&patched);
        loop {
            match reader.read_event().unwrap() {
                XmlEvent::Eof => break,
                XmlEvent::Start(event) if event.local_name().as_ref() == b"keyboard" => {
                    break;
                }
                _ => {}
            }
        }
    }

    #[test]
    fn real_config_patch_when_requested() {
        let Ok(path) = std::env::var("LXWC_TEST_RC") else {
            return;
        };
        let raw = std::fs::read_to_string(path).unwrap();
        let before = parse(&raw).unwrap();

        let added = patch(
            &raw,
            &[Addition {
                key: "W-C-S-t".to_string(),
                attrs: Vec::new(),
                actions: vec![action("Execute", "qterminal")],
            }],
            &[],
        )
        .unwrap();
        let after_add = parse(&added).unwrap();
        assert_eq!(after_add.len(), before.len() + 1);

        let replaced = patch(
            &raw,
            &[Addition {
                key: "A-F2".to_string(),
                attrs: Vec::new(),
                actions: vec![action("Execute", "qterminal")],
            }],
            &["A-F2".to_string()],
        )
        .unwrap();
        let after_replace = parse(&replaced).unwrap();
        assert_eq!(after_replace.len(), before.len());
        assert_eq!(
            after_replace
                .iter()
                .filter(|entry| entry.key == "A-F2")
                .count(),
            1
        );

        let removed = patch(&raw, &[], &["A-F2".to_string()]).unwrap();
        let after_remove = parse(&removed).unwrap();
        assert_eq!(after_remove.len(), before.len() - 1);
    }
}
