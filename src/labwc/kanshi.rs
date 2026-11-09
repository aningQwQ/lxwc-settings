use crate::wlr::{Mode, OutputInfo};

const BEGIN: &str = "# BEGIN lxwc-settings dynamic display";
const END: &str = "# END lxwc-settings dynamic display";

pub fn profile(
    outputs: &[OutputInfo],
    selected_output: &str,
    selected_mode: &Mode,
) -> Option<String> {
    let mut rules = Vec::new();
    let mut next_x = 0i32;

    for output in outputs {
        let name = escape(&output.name);
        if !output.enabled && output.name != selected_output {
            rules.push(format!("    output \"{name}\" disable"));
            continue;
        }
        let mode = if output.name == selected_output {
            Some(selected_mode)
        } else {
            output.current.as_ref()
        };
        let Some(mode) = mode else {
            rules.push(format!("    output \"{name}\" enable"));
            continue;
        };
        let (x, y) = output.position.unwrap_or((next_x, 0));
        let mut rule = format!(
            "    output \"{name}\" mode {} position {x},{y}",
            mode.spec()
        );
        if let Some(scale) = output.scale {
            if (scale - 1.0).abs() > 0.001 {
                rule.push_str(&format!(" scale {scale}"));
            }
        }
        rules.push(rule);
        next_x = x + mode.width as i32;
    }

    if rules.is_empty() {
        return None;
    }

    Some(format!("profile {{\n{}\n}}", rules.join("\n")))
}

pub fn parse_profile(block: &str) -> Option<(String, String)> {
    for line in block.lines() {
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        if tokens.first().copied() != Some("output") {
            continue;
        }
        let name = tokens.get(1)?.trim_matches('"');
        let mode_index = tokens.iter().position(|token| *token == "mode")?;
        let mode = tokens.get(mode_index + 1)?;
        return Some((name.to_string(), mode.to_string()));
    }
    None
}

pub fn managed_block(raw: &str) -> Option<String> {
    let mut block = Vec::new();
    let mut inside = false;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed == BEGIN {
            inside = true;
            block.clear();
            continue;
        }
        if trimmed == END {
            if inside {
                let text = block.join("\n");
                return (!text.trim().is_empty()).then_some(text);
            }
            inside = false;
            block.clear();
            continue;
        }
        if inside {
            block.push(line.to_string());
        }
    }
    None
}

pub fn remove_block(raw: &str) -> String {
    let mut kept = String::new();
    let mut skipping = false;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed == BEGIN {
            skipping = true;
            continue;
        }
        if skipping {
            if trimmed == END {
                skipping = false;
            }
            continue;
        }
        kept.push_str(line);
        kept.push('\n');
    }
    let trimmed = kept.trim_end_matches('\n');
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("{trimmed}\n")
    }
}

pub fn contents(raw: &str, profile: &str) -> String {
    let block = format!("{BEGIN}\n{profile}\n{END}");

    if let Some(output) = replace_block(raw, BEGIN, END, &block) {
        return output;
    }

    let mut output = raw.to_string();
    if !output.is_empty() {
        if !output.ends_with('\n') {
            output.push('\n');
        }
        if !output.ends_with("\n\n") {
            output.push('\n');
        }
    }
    output.push_str(&block);
    output.push('\n');
    output
}

fn replace_block(raw: &str, begin_marker: &str, end_marker: &str, block: &str) -> Option<String> {
    let begin = raw.find(begin_marker)?;
    let end_start = begin + begin_marker.len();
    let end = end_start + raw[end_start..].find(end_marker)? + end_marker.len();
    let mut end = end;
    if raw[end..].starts_with('\n') {
        end += 1;
    }
    let mut output = String::with_capacity(raw.len() + block.len());
    output.push_str(&raw[..begin]);
    output.push_str(block);
    output.push('\n');
    output.push_str(&raw[end..]);
    Some(output)
}

fn escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output(name: &str, enabled: bool, mode: Mode, position: (i32, i32)) -> OutputInfo {
        OutputInfo {
            name: name.to_string(),
            description: String::new(),
            enabled,
            current: Some(mode),
            modes: Vec::new(),
            position: Some(position),
            scale: Some(1.5),
        }
    }

    #[test]
    fn builds_multi_output_profile() {
        let outputs = vec![
            output(
                "eDP-1",
                true,
                Mode {
                    width: 1920,
                    height: 1080,
                    refresh: 60.0,
                },
                (0, 0),
            ),
            output(
                "HDMI-A-1",
                true,
                Mode {
                    width: 2560,
                    height: 1440,
                    refresh: 60.0,
                },
                (1920, 0),
            ),
            output(
                "DP-2",
                false,
                Mode {
                    width: 1024,
                    height: 768,
                    refresh: 60.0,
                },
                (0, 0),
            ),
        ];
        let selected = Mode {
            width: 2560,
            height: 1440,
            refresh: 59.95,
        };
        let profile = profile(&outputs, "HDMI-A-1", &selected).unwrap();
        assert!(profile.contains("profile {"));
        assert!(
            profile.contains("output \"eDP-1\" mode 1920x1080@60.000000Hz position 0,0 scale 1.5")
        );
        assert!(profile.contains("output \"HDMI-A-1\" mode 2560x1440@59.950000Hz position 1920,0"));
        assert!(profile.contains("output \"DP-2\" disable"));
    }

    #[test]
    fn parses_profile_output_and_mode() {
        let profile =
            "profile {\n    output \"HDMI-A-1\" mode 1920x1080@60.000000Hz position 0,0\n}";
        assert_eq!(
            parse_profile(profile),
            Some(("HDMI-A-1".to_string(), "1920x1080@60.000000Hz".to_string()))
        );
        assert!(parse_profile("profile {\n    output \"eDP-1\" disable\n}").is_none());
    }

    #[test]
    fn reads_and_removes_managed_block() {
        let text = contents("profile \"other\" {\n}\n", "profile {\n}");
        assert_eq!(managed_block(&text).as_deref(), Some("profile {\n}"));
        let removed = remove_block(&text);
        assert_eq!(removed, "profile \"other\" {\n}\n");
        assert!(managed_block(&removed).is_none());
    }

    #[test]
    fn uses_descriptive_head_and_tail_markers() {
        let updated = contents("", "profile {\n}");
        assert!(updated.contains("# BEGIN lxwc-settings dynamic display"));
        assert!(updated.contains("# END lxwc-settings dynamic display"));
    }

    #[test]
    fn replaces_managed_block_and_preserves_other_profiles() {
        let raw =
            format!("profile \"other\" {{\n    output \"eDP-1\"\n}}\n\n{BEGIN}\nold\n{END}\n");
        let updated = contents(&raw, "profile {\n}");
        assert!(updated.contains("profile \"other\""));
        assert!(updated.contains("profile {"));
        assert!(!updated.contains("old"));
        assert_eq!(updated.matches(BEGIN).count(), 1);
        assert_eq!(updated.matches(END).count(), 1);
    }

    #[test]
    fn appends_block_to_existing_config() {
        let updated = contents("profile \"other\" {\n}\n", "profile {\n}");
        assert!(updated.starts_with("profile \"other\""));
        assert!(updated.contains(BEGIN));
        assert!(updated.ends_with(&format!("{END}\n")));
    }

    #[test]
    fn returns_none_without_outputs() {
        assert!(profile(
            &[],
            "eDP-1",
            &Mode {
                width: 1920,
                height: 1080,
                refresh: 60.0,
            }
        )
        .is_none());
    }
}
