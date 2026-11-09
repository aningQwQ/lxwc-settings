use std::collections::{BTreeMap, BTreeSet};

pub fn parse_active(raw: &str) -> BTreeMap<String, String> {
    let mut vars = BTreeMap::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=') {
            let key = key.trim();
            if is_valid_key(key) {
                vars.insert(key.to_string(), value.trim().to_string());
            }
        }
    }
    vars
}

pub fn patch(raw: &str, sets: &BTreeMap<String, String>, unsets: &[String]) -> String {
    let mut out = String::with_capacity(raw.len() + 128);
    let mut written: BTreeSet<String> = BTreeSet::new();

    for line in raw.split_inclusive('\n') {
        let (body, ending) = split_ending(line);
        let trimmed = body.trim();
        let active = !trimmed.is_empty() && !trimmed.starts_with('#');
        let mut replaced = false;

        if active {
            if let Some((key, _)) = trimmed.split_once('=') {
                let key = key.trim();
                if let Some(value) = sets.get(key) {
                    out.push_str(key);
                    out.push('=');
                    out.push_str(value);
                    out.push_str(ending);
                    written.insert(key.to_string());
                    replaced = true;
                } else if unsets.iter().any(|item| item == key) {
                    replaced = true;
                }
            }
        }

        if !replaced {
            out.push_str(line);
        }
    }

    let mut appended = false;
    for (key, value) in sets {
        if written.contains(key) {
            continue;
        }
        if !appended {
            if !out.is_empty() && !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("\n# added by lxwc-settings\n");
            appended = true;
        }
        out.push_str(key);
        out.push('=');
        out.push_str(value);
        out.push('\n');
    }

    out
}

fn split_ending(line: &str) -> (&str, &str) {
    if let Some(body) = line.strip_suffix("\r\n") {
        (body, "\r\n")
    } else if let Some(body) = line.strip_suffix('\n') {
        (body, "\n")
    } else {
        (line, "")
    }
}

fn is_valid_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        && !key.chars().next().map(|ch| ch.is_ascii_digit()).unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sets(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect()
    }

    #[test]
    fn parses_only_active_lines() {
        let raw = "# WLR_DRM_NO_MODIFIERS=0\nWLR_NO_HARDWARE_CURSORS=1\nFOO = bar\n";
        let parsed = parse_active(raw);
        assert_eq!(parsed.get("WLR_NO_HARDWARE_CURSORS").unwrap(), "1");
        assert_eq!(parsed.get("FOO").unwrap(), "bar");
        assert!(!parsed.contains_key("WLR_DRM_NO_MODIFIERS"));
    }

    #[test]
    fn replaces_existing_line_in_place() {
        let raw = "A=1\nWLR_DRM_NO_MODIFIERS=0\nB=2";
        let patched = patch(raw, &sets(&[("WLR_DRM_NO_MODIFIERS", "1")]), &[]);
        assert_eq!(patched, "A=1\nWLR_DRM_NO_MODIFIERS=1\nB=2");
    }

    #[test]
    fn removes_active_line() {
        let raw = "A=1\nWLR_DRM_NO_MODIFIERS=1\nB=2\n";
        let patched = patch(raw, &BTreeMap::new(), &["WLR_DRM_NO_MODIFIERS".to_string()]);
        assert_eq!(patched, "A=1\nB=2\n");
    }

    #[test]
    fn appends_missing_variable() {
        let raw = "A=1\n";
        let patched = patch(raw, &sets(&[("WLR_DRM_NO_MODIFIERS", "1")]), &[]);
        assert!(patched.starts_with("A=1\n"));
        assert!(patched.contains("WLR_DRM_NO_MODIFIERS=1\n"));
    }
}
