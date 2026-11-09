const AUTOSTART_BEGIN: &str = "# BEGIN lxwc-settings default display";
const AUTOSTART_END: &str = "# END lxwc-settings default display";

pub fn contents(raw: &str, command: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

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
