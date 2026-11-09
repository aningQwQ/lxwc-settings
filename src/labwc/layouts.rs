#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    pub code: String,
    pub description: String,
}

pub fn layouts() -> Vec<Layout> {
    let path = std::env::var_os("LXWC_XKB_RULES")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("/usr/share/X11/xkb/rules/evdev.lst"));
    let content = std::fs::read_to_string(path).ok();
    content
        .as_deref()
        .map(parse)
        .filter(|layouts| !layouts.is_empty())
        .unwrap_or_else(fallback)
}

pub fn parse(content: &str) -> Vec<Layout> {
    let mut layouts = Vec::new();
    let mut in_layout = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(section) = trimmed.strip_prefix('!') {
            in_layout = section.trim() == "layout";
            continue;
        }
        if !in_layout {
            continue;
        }
        let Some((code, description)) = split_entry(trimmed) else {
            continue;
        };
        layouts.push(Layout {
            code: code.to_string(),
            description: description.to_string(),
        });
    }
    layouts
}

fn split_entry(line: &str) -> Option<(&str, &str)> {
    let mut parts = line.splitn(2, char::is_whitespace);
    let code = parts.next()?.trim();
    let description = parts.next()?.trim();
    if code.is_empty() || description.is_empty() {
        return None;
    }
    Some((code, description))
}

fn fallback() -> Vec<Layout> {
    [
        ("us", "English (US)"),
        ("gb", "English (UK)"),
        ("de", "German"),
        ("fr", "French"),
        ("es", "Spanish"),
        ("it", "Italian"),
        ("ru", "Russian"),
        ("cn", "Chinese"),
        ("jp", "Japanese"),
        ("kr", "Korean"),
    ]
    .into_iter()
    .map(|(code, description)| Layout {
        code: code.to_string(),
        description: description.to_string(),
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_layout_section_only() {
        let content = "\
! model
  pc105  Generic 105-key
! layout
  us              English (US)
  gb              English (UK)
! variant
  intl            US international
";
        let layouts = parse(content);
        assert_eq!(layouts.len(), 2);
        assert_eq!(layouts[0].code, "us");
        assert_eq!(layouts[0].description, "English (US)");
        assert_eq!(layouts[1].code, "gb");
    }

    #[test]
    fn fallback_contains_common_layouts() {
        let fallback = fallback();
        assert_eq!(fallback.len(), 10);
        assert!(fallback.iter().any(|layout| layout.code == "us"));
        assert!(fallback.iter().any(|layout| layout.code == "cn"));
    }
}
