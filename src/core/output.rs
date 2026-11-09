use std::process::Command as ProcessCommand;

use super::model::{Mode, OutputInfo};

pub fn scan() -> Result<Vec<OutputInfo>, String> {
    let output = ProcessCommand::new("wlr-randr")
        .output()
        .map_err(|err| format!("无法执行 wlr-randr：{err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("wlr-randr 执行失败：{}", stderr.trim()));
    }
    Ok(parse(&String::from_utf8_lossy(&output.stdout)))
}

pub fn parse(text: &str) -> Vec<OutputInfo> {
    let mut outputs: Vec<OutputInfo> = Vec::new();
    let mut current: Option<OutputInfo> = None;

    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let indented = line.starts_with(' ') || line.starts_with('\t');

        if !indented {
            if let Some(item) = current.take() {
                outputs.push(item);
            }
            let name = line.split_whitespace().next().unwrap_or_default().to_string();
            if name.is_empty() {
                continue;
            }
            let description = line
                .find('"')
                .and_then(|start| {
                    line.rfind('"')
                        .filter(|end| *end > start)
                        .map(|end| line[start + 1..end].to_string())
                })
                .unwrap_or_default();
            current = Some(OutputInfo {
                name,
                description,
                enabled: true,
                current: None,
                modes: Vec::new(),
            });
            continue;
        }

        let Some(item) = current.as_mut() else {
            continue;
        };
        let trimmed = line.trim();

        if let Some(rest) = trimmed.strip_prefix("Enabled:") {
            item.enabled = rest.trim().eq_ignore_ascii_case("yes");
            continue;
        }

        if let Some((mode, flags)) = parse_mode_line(trimmed) {
            let is_current = flags.contains("current");
            if is_current {
                item.current = Some(mode.clone());
            }
            let duplicate = item
                .modes
                .iter()
                .any(|existing| existing.same_mode(&mode));
            if !duplicate {
                item.modes.push(mode);
            }
        }
    }

    if let Some(item) = current.take() {
        outputs.push(item);
    }

    for item in &mut outputs {
        if let Some(current) = &item.current {
            if !item.modes.iter().any(|mode| mode.same_mode(current)) {
                item.modes.push(current.clone());
            }
        }
        item.modes
            .sort_by(|a, b| b.width.cmp(&a.width).then(b.height.cmp(&a.height)).then(b.refresh.total_cmp(&a.refresh)));
    }

    outputs
}

fn parse_mode_line(line: &str) -> Option<(Mode, String)> {
    let (dims, rest) = line.split_once(" px,")?;
    let (width, height) = dims.split_once('x')?;
    let rest = rest.trim();
    let hz_index = rest.find(" Hz")?;
    let refresh: f64 = rest[..hz_index].trim().parse().ok()?;
    let flags = rest[hz_index + 3..].trim().to_string();
    Some((
        Mode {
            width: width.trim().parse().ok()?,
            height: height.trim().parse().ok()?,
            refresh,
        },
        flags,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"HDMI-A-1 "Lenovo T2224r (HDMI-A-1)"
  Make: Lenovo
  Enabled: yes
  Modes:
    1920x1080 px, 60.000000 Hz (preferred, current)
    1920x1080 px, 60.000000 Hz
    1280x720 px, 59.939999 Hz
  Position: 0,0
"#;

    #[test]
    fn parses_outputs_and_modes() {
        let outputs = parse(SAMPLE);
        assert_eq!(outputs.len(), 1);
        let output = &outputs[0];
        assert_eq!(output.name, "HDMI-A-1");
        assert!(output.enabled);
        assert_eq!(output.modes.len(), 2);
        assert!(output.current.as_ref().unwrap().same_mode(&Mode {
            width: 1920,
            height: 1080,
            refresh: 60.0,
        }));
    }

    #[test]
    fn mode_spec_matches_wlr_randr() {
        let mode = Mode {
            width: 1920,
            height: 1080,
            refresh: 59.939999,
        };
        assert_eq!(mode.spec(), "1920x1080@59.939999Hz");
    }
}
