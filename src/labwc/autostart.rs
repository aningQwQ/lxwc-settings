const AUTOSTART_BEGIN: &str = "# BEGIN lxwc-settings default display";
const AUTOSTART_END: &str = "# END lxwc-settings default display";

pub fn load(path: &std::path::Path) -> Result<crate::services::fs::LoadedFile, String> {
    crate::services::fs::load(path)
}

pub fn managed_block(raw: &str) -> Option<String> {
    let mut block = Vec::new();
    let mut inside = false;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed == AUTOSTART_BEGIN {
            inside = true;
            block.clear();
            continue;
        }
        if trimmed == AUTOSTART_END {
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

pub fn parse_command(block: &str) -> Option<(String, String)> {
    for line in block.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("wlr-randr") {
            continue;
        }
        let tokens = trimmed.split_whitespace().collect::<Vec<_>>();
        let mut output = None;
        let mut mode = None;
        let mut index = 0;
        while index < tokens.len() {
            match tokens[index] {
                "--output" => {
                    output = tokens.get(index + 1).map(|value| (*value).to_string());
                    index += 2;
                }
                "--mode" => {
                    mode = tokens.get(index + 1).map(|value| (*value).to_string());
                    index += 2;
                }
                _ => index += 1,
            }
        }
        if let (Some(output), Some(mode)) = (output, mode) {
            return Some((output, mode));
        }
    }
    None
}

pub fn remove_block(raw: &str) -> String {
    let kept = strip_block(raw);
    let trimmed = kept.trim_end_matches('\n');
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("{trimmed}\n")
    }
}

pub fn contents(raw: &str, command: &str) -> String {
    let kept = strip_block(raw);
    let kept = kept.trim_end_matches('\n');
    let mut out = String::new();
    if !kept.is_empty() {
        out.push_str(kept);
        out.push_str("\n\n");
    }
    out.push_str(AUTOSTART_BEGIN);
    out.push('\n');
    out.push_str(command);
    out.push('\n');
    out.push_str(AUTOSTART_END);
    out.push('\n');
    out
}

fn strip_block(raw: &str) -> String {
    let mut kept = String::new();
    let mut skipping = false;

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed == AUTOSTART_BEGIN {
            skipping = true;
            continue;
        }
        if skipping {
            if trimmed == AUTOSTART_END {
                skipping = false;
            }
            continue;
        }
        kept.push_str(line);
        kept.push('\n');
    }

    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_persisted_command() {
        let block = "wlr-randr --output HDMI-A-1 --mode 1920x1080@60.000000Hz";
        assert_eq!(
            parse_command(block),
            Some(("HDMI-A-1".to_string(), "1920x1080@60.000000Hz".to_string()))
        );
        assert!(parse_command("echo hello").is_none());
    }

    #[test]
    fn reads_and_removes_managed_block() {
        let text = contents("foo\n", "wlr-randr --output A --mode 1x1@60.000000Hz");
        assert_eq!(
            managed_block(&text).as_deref(),
            Some("wlr-randr --output A --mode 1x1@60.000000Hz")
        );
        let removed = remove_block(&text);
        assert_eq!(removed, "foo\n");
        assert!(managed_block(&removed).is_none());
    }

    #[test]
    fn block_is_idempotent_and_preserves_content() {
        let first = contents("", "wlr-randr --output A --mode 1x1@60.000000Hz");
        assert!(first.contains(AUTOSTART_BEGIN));
        assert!(first.contains("1x1"));
        let second = contents(&first, "wlr-randr --output A --mode 1x1@60.000000Hz");
        assert_eq!(first, second);
        let replaced = contents(&first, "wlr-randr --output A --mode 2x2@60.000000Hz");
        assert!(replaced.contains("2x2"));
        assert!(!replaced.contains("1x1"));
        let with_other = contents("foo\n", "cmd");
        assert!(with_other.contains("foo"));
        assert!(with_other.contains("cmd"));
    }
}
