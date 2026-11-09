pub struct ActionPreset {
    pub label_key: &'static str,
    pub action: &'static str,
    pub command: &'static str,
}

pub const PRESETS: &[ActionPreset] = &[
    ActionPreset {
        label_key: "lxqt_open_start_menu",
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
