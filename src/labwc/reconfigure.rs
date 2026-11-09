pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
}

pub fn command() -> CommandSpec {
    CommandSpec {
        program: "sh".to_string(),
        args: vec![
            "-c".to_string(),
            "labwc --reconfigure || pkill -HUP -x labwc".to_string(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_labwc_reconfigure_command() {
        let command = command();
        assert_eq!(command.program, "sh");
        assert!(command
            .args
            .iter()
            .any(|arg| arg.contains("labwc --reconfigure")));
    }
}
