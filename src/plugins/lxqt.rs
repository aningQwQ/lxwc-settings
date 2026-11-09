pub struct ActionPreset {
    pub label_key: &'static str,
    pub action: &'static str,
    pub command: &'static str,
}

pub const PRESETS: &[ActionPreset] = &[
    ActionPreset {
        label_key: "lxqt_open_start_menu",
        action: "Execute",
        command: "lxqt-qdbus openmenu",
    },
    ActionPreset {
        label_key: "lxqt_run_dialog",
        action: "Execute",
        command: "lxqt-runner",
    },
    ActionPreset {
        label_key: "lxqt_open_terminal",
        action: "Execute",
        command: "lxqt-qdbus run qterminal",
    },
    ActionPreset {
        label_key: "lxqt_open_file_manager",
        action: "Execute",
        command: "lxqt-qdbus run pcmanfm-qt",
    },
    ActionPreset {
        label_key: "lxqt_show_desktop",
        action: "Execute",
        command: "lxqt-qdbus showdesktop",
    },
    ActionPreset {
        label_key: "lxqt_clipboard",
        action: "Execute",
        command: "lxqt-qdbus lxqt-clip",
    },
    ActionPreset {
        label_key: "lxqt_restart_panel",
        action: "Execute",
        command: "lxqt-qdbus restart panel",
    },
    ActionPreset {
        label_key: "lxqt_volume_up",
        action: "Execute",
        command: "lxqt-qdbus volume up",
    },
    ActionPreset {
        label_key: "lxqt_volume_down",
        action: "Execute",
        command: "lxqt-qdbus volume down",
    },
    ActionPreset {
        label_key: "lxqt_volume_mute",
        action: "Execute",
        command: "lxqt-qdbus volume mute",
    },
    ActionPreset {
        label_key: "lxqt_lock_screen",
        action: "Execute",
        command: "lxqt-leave --lockscreen",
    },
    ActionPreset {
        label_key: "lxqt_log_out",
        action: "Execute",
        command: "lxqt-leave --logout",
    },
    ActionPreset {
        label_key: "lxqt_suspend",
        action: "Execute",
        command: "lxqt-leave --suspend",
    },
    ActionPreset {
        label_key: "lxqt_hibernate",
        action: "Execute",
        command: "lxqt-leave --hibernate",
    },
    ActionPreset {
        label_key: "lxqt_reboot",
        action: "Execute",
        command: "lxqt-leave --reboot",
    },
    ActionPreset {
        label_key: "lxqt_shut_down",
        action: "Execute",
        command: "lxqt-leave --shutdown",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_menu_uses_openmenu() {
        let preset = PRESETS
            .iter()
            .find(|preset| preset.label_key == "lxqt_open_start_menu")
            .unwrap();
        assert_eq!(preset.command, "lxqt-qdbus openmenu");
        assert!(!preset.command.contains("lxqt-runner"));
    }

    #[test]
    fn run_dialog_uses_lxqt_runner() {
        let preset = PRESETS
            .iter()
            .find(|preset| preset.label_key == "lxqt_run_dialog")
            .unwrap();
        assert_eq!(preset.action, "Execute");
        assert_eq!(preset.command, "lxqt-runner");
    }

    #[test]
    fn presets_have_unique_labels_and_non_empty_commands() {
        for (index, preset) in PRESETS.iter().enumerate() {
            assert!(!preset.label_key.is_empty());
            assert!(!preset.command.trim().is_empty());
            assert!(PRESETS[..index]
                .iter()
                .all(|other| other.label_key != preset.label_key));
        }
    }
}
