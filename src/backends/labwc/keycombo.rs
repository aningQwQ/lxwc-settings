#[derive(Clone, Copy, Default)]
pub struct ComboParts {
    pub sup: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

pub fn parse_parts(input: &str) -> (ComboParts, Option<String>) {
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
            if !out.is_empty() {
                out.push('-');
            }
            out.push(label);
        }
    }
    if let Some(base) = base {
        if !out.is_empty() {
            out.push('-');
        }
        out.push_str(base);
    }
    out
}

pub fn toggle_modifier(input: &str, which: char) -> String {
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

pub fn toggle_base_key(input: &str, base: &str) -> String {
    let (parts, current) = parse_parts(input);
    if current.as_deref() == Some(base) {
        compose_combo(parts, None)
    } else {
        compose_combo(parts, Some(base))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggles_modifiers_and_keeps_base() {
        assert_eq!(toggle_modifier("W-t", 'C'), "W-C-t");
        assert_eq!(toggle_modifier("W-C-t", 'C'), "W-t");
        assert_eq!(toggle_modifier("", 'W'), "W");
        assert_eq!(toggle_modifier("W", 'W'), "");
        assert_eq!(toggle_modifier("W", 'C'), "W-C");
        assert_eq!(toggle_modifier("W-C", 'A'), "W-C-A");
        assert_eq!(toggle_modifier("W-C", 'C'), "W");
    }

    #[test]
    fn toggles_base_key() {
        assert_eq!(toggle_base_key("W-C", "p"), "W-C-p");
        assert_eq!(toggle_base_key("W-C-t", "Return"), "W-C-Return");
        assert_eq!(toggle_base_key("", "Print"), "Print");
        assert_eq!(toggle_base_key("W-x-y", "p"), "W-p");
        assert_eq!(toggle_base_key("W-t", "t"), "W");
        assert_eq!(toggle_base_key("W", "t"), "W-t");
        assert_eq!(toggle_base_key("W-t", "p"), "W-p");
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
