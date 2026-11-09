use std::collections::{BTreeMap, BTreeSet};

const ENV_BEGIN: &str = "# BEGIN lxwc-settings environment";
const ENV_END: &str = "# END lxwc-settings environment";
const LEGACY_COMMENT: &str = "# added by lxwc-settings";

pub fn load(path: &std::path::Path) -> Result<crate::services::fs::LoadedFile, String> {
    crate::services::fs::load(path)
}

pub fn parse_active(raw: &str) -> BTreeMap<String, String> {
    let mut vars = BTreeMap::new();
    for line in raw.lines() {
        if let Some((key, value)) = active_pair(line) {
            vars.insert(key.to_string(), value.to_string());
        }
    }
    vars
}

pub fn managed_vars(raw: &str) -> BTreeMap<String, String> {
    let mut vars = BTreeMap::new();
    let mut in_block = false;
    let mut legacy = false;

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed == ENV_BEGIN {
            in_block = true;
            continue;
        }
        if trimmed == ENV_END {
            in_block = false;
            continue;
        }
        if in_block {
            if let Some((key, value)) = active_pair(trimmed) {
                vars.insert(key.to_string(), value.to_string());
            }
            continue;
        }
        if trimmed == LEGACY_COMMENT {
            legacy = true;
            continue;
        }
        if legacy {
            if let Some((key, value)) = active_pair(trimmed) {
                vars.insert(key.to_string(), value.to_string());
                continue;
            }
            legacy = false;
        }
    }

    vars
}

pub fn patch(raw: &str, sets: &BTreeMap<String, String>, unsets: &[String]) -> String {
    let managed = managed_vars(raw);
    let managed_keys = managed.keys().cloned().collect::<BTreeSet<_>>();
    let unset_keys = unsets.iter().cloned().collect::<BTreeSet<_>>();
    let mut out = String::with_capacity(raw.len() + 128);
    let mut in_block = false;
    let mut legacy = false;

    for line in raw.split_inclusive('\n') {
        let (body, _) = split_ending(line);
        let trimmed = body.trim();

        if trimmed == ENV_BEGIN {
            in_block = true;
            continue;
        }
        if trimmed == ENV_END {
            in_block = false;
            continue;
        }
        if in_block {
            continue;
        }
        if trimmed == LEGACY_COMMENT {
            legacy = true;
            continue;
        }
        if legacy {
            if active_pair(trimmed).is_some() {
                continue;
            }
            legacy = false;
        }

        if let Some((key, _)) = active_pair(trimmed) {
            if managed_keys.contains(key) || sets.contains_key(key) || unset_keys.contains(key) {
                continue;
            }
        }

        out.push_str(line);
    }

    let mut managed = managed;
    for (key, value) in sets {
        managed.insert(key.clone(), value.clone());
    }
    for key in unsets {
        managed.remove(key);
    }

    if managed.is_empty() {
        let trimmed = out.trim_end_matches('\n');
        return if trimmed.is_empty() {
            String::new()
        } else {
            format!("{trimmed}\n")
        };
    }

    let kept = out.trim_end_matches('\n');
    let mut result = String::with_capacity(kept.len() + managed.len() * 32 + 128);
    if !kept.is_empty() {
        result.push_str(kept);
        result.push_str("\n\n");
    }
    result.push_str(ENV_BEGIN);
    result.push('\n');
    for (key, value) in &managed {
        result.push_str(key);
        result.push('=');
        result.push_str(value);
        result.push('\n');
    }
    result.push_str(ENV_END);
    result.push('\n');
    result
}

fn active_pair(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let (key, value) = trimmed.split_once('=')?;
    let key = key.trim();
    is_valid_key(key).then_some((key, value.trim()))
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
        && !key
            .chars()
            .next()
            .map(|ch| ch.is_ascii_digit())
            .unwrap_or(true)
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
    fn appends_managed_block() {
        let patched = patch("A=1\n", &sets(&[("WLR_DRM_NO_MODIFIERS", "1")]), &[]);
        assert!(patched.starts_with("A=1\n"));
        assert!(patched.contains(ENV_BEGIN));
        assert!(patched.contains("WLR_DRM_NO_MODIFIERS=1"));
        assert!(patched.contains(ENV_END));
    }

    #[test]
    fn replaces_existing_line_into_block() {
        let raw = "A=1\nWLR_DRM_NO_MODIFIERS=0\nB=2\n";
        let patched = patch(raw, &sets(&[("WLR_DRM_NO_MODIFIERS", "1")]), &[]);
        assert_eq!(
            patched,
            "A=1\nB=2\n\n# BEGIN lxwc-settings environment\nWLR_DRM_NO_MODIFIERS=1\n# END lxwc-settings environment\n"
        );
    }

    #[test]
    fn updates_existing_block_and_preserves_other_keys() {
        let raw = "A=1\n# BEGIN lxwc-settings environment\nB=2\n# END lxwc-settings environment\n";
        let patched = patch(raw, &sets(&[("B", "3"), ("C", "4")]), &[]);
        assert!(patched.contains("A=1"));
        assert!(patched.contains("B=3"));
        assert!(patched.contains("C=4"));
        assert_eq!(patched.matches(ENV_BEGIN).count(), 1);
        assert_eq!(patched.matches(ENV_END).count(), 1);
    }

    #[test]
    fn removes_key_and_empty_block() {
        let raw = "A=1\n# BEGIN lxwc-settings environment\nB=2\n# END lxwc-settings environment\n";
        let patched = patch(raw, &BTreeMap::new(), &["B".to_string()]);
        assert_eq!(patched, "A=1\n");
    }

    #[test]
    fn migrates_legacy_comment_block() {
        let raw = "A=1\n\n# added by lxwc-settings\nB=2\n";
        let patched = patch(raw, &sets(&[("B", "3")]), &[]);
        assert!(patched.contains("A=1"));
        assert!(patched.contains("B=3"));
        assert!(!patched.contains(LEGACY_COMMENT));
        assert!(patched.contains(ENV_BEGIN));
    }

    #[test]
    fn multiple_new_variables_share_one_block() {
        let sets = BTreeMap::from([
            ("A".to_string(), "1".to_string()),
            ("B".to_string(), "2".to_string()),
        ]);
        let patched = patch("", &sets, &[]);
        assert_eq!(patched.matches(ENV_BEGIN).count(), 1);
        assert!(patched.contains("A=1"));
        assert!(patched.contains("B=2"));
    }
}
