#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurveError {
    Step,
    Points,
}

pub fn validate(step: &str, points: &str) -> Result<(String, String), CurveError> {
    let step = parse_step(step).ok_or(CurveError::Step)?;
    let points = parse_points(points).ok_or(CurveError::Points)?;
    Ok((format_number(step), points))
}

fn parse_step(text: &str) -> Option<f64> {
    if text.contains('\0') {
        return None;
    }
    let value = parse_number(text.trim())?;
    (value > 0.0).then_some(value)
}

fn parse_points(text: &str) -> Option<String> {
    if text.contains('\0') {
        return None;
    }
    let mut values = Vec::new();
    for token in text.split_whitespace() {
        let value = parse_number(token)?;
        if value < 0.0 {
            return None;
        }
        values.push(value);
    }
    if !(2..=32).contains(&values.len()) {
        return None;
    }
    Some(
        values
            .into_iter()
            .map(format_number)
            .collect::<Vec<_>>()
            .join(" "),
    )
}

fn parse_number(text: &str) -> Option<f64> {
    if text.is_empty() {
        return None;
    }
    let mut normalized = String::with_capacity(text.len());
    let mut separator = false;
    for character in text.chars() {
        match character {
            '.' | ',' => {
                if separator {
                    return None;
                }
                separator = true;
                normalized.push('.');
            }
            _ => normalized.push(character),
        }
    }
    let value = normalized.parse::<f64>().ok()?;
    value.is_finite().then_some(value)
}

fn format_number(value: f64) -> String {
    let mut text = value.to_string();
    if text == "-0" {
        text = "0".to_string();
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_curve() {
        assert_eq!(
            validate("1", "0 1 2 3").unwrap(),
            ("1".to_string(), "0 1 2 3".to_string())
        );
        assert_eq!(
            validate(" 0,5 ", "0.0 1,5\t3\n4").unwrap(),
            ("0.5".to_string(), "0 1.5 3 4".to_string())
        );
    }

    #[test]
    fn rejects_invalid_step() {
        assert_eq!(validate("", "0 1"), Err(CurveError::Step));
        assert_eq!(validate("0", "0 1"), Err(CurveError::Step));
        assert_eq!(validate("-1", "0 1"), Err(CurveError::Step));
        assert_eq!(validate("NaN", "0 1"), Err(CurveError::Step));
        assert_eq!(validate("inf", "0 1"), Err(CurveError::Step));
        assert_eq!(validate("1.2.3", "0 1"), Err(CurveError::Step));
        assert_eq!(validate("1 2", "0 1"), Err(CurveError::Step));
    }

    #[test]
    fn rejects_invalid_points() {
        assert_eq!(validate("1", ""), Err(CurveError::Points));
        assert_eq!(validate("1", "1"), Err(CurveError::Points));
        assert_eq!(validate("1", "1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32 33"), Err(CurveError::Points));
        assert_eq!(validate("1", "0 -1"), Err(CurveError::Points));
        assert_eq!(validate("1", "0 NaN"), Err(CurveError::Points));
        assert_eq!(validate("1", "0 inf"), Err(CurveError::Points));
        assert_eq!(validate("1", "0 1,2,3"), Err(CurveError::Points));
    }

    #[test]
    fn accepts_boundary_point_counts() {
        assert!(validate("1", "0 1").is_ok());
        assert!(validate(
            "1",
            "0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31"
        )
        .is_ok());
    }
}
