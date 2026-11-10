use std::path::PathBuf;

pub fn config_dir() -> PathBuf {
    std::env::var_os("LXWC_CONFIG_DIR")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME").map(|dir| PathBuf::from(dir).join("labwc")))
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config/labwc")))
        .unwrap_or_else(|| PathBuf::from(".config/labwc"))
}

pub fn rc_xml() -> PathBuf {
    std::env::var_os("LXWC_RC_XML")
        .map(PathBuf::from)
        .unwrap_or_else(|| config_dir().join("rc.xml"))
}

pub fn environment() -> PathBuf {
    std::env::var_os("LXWC_ENVIRONMENT")
        .map(PathBuf::from)
        .unwrap_or_else(|| config_dir().join("environment"))
}

pub fn autostart() -> PathBuf {
    std::env::var_os("LXWC_AUTOSTART")
        .map(PathBuf::from)
        .unwrap_or_else(|| config_dir().join("autostart"))
}
