use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    ZhCn,
    En,
}

impl Lang {
    pub const ALL: [Lang; 2] = [Lang::ZhCn, Lang::En];

    pub fn from_env() -> Self {
        if let Some(lang) = override_lang() {
            return lang;
        }

        for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(value) = std::env::var(key) {
                if value.is_empty() {
                    continue;
                }
                return if value.to_lowercase().contains("zh") {
                    Self::ZhCn
                } else {
                    Self::En
                };
            }
        }

        Self::ZhCn
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::ZhCn => "简体中文",
            Self::En => "English",
        }
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn set_current(lang: Lang) {
    CURRENT.store(
        match lang {
            Lang::ZhCn => 0,
            Lang::En => 1,
        },
        Ordering::Relaxed,
    );
}

pub fn current() -> Lang {
    if CURRENT.load(Ordering::Relaxed) == 1 {
        Lang::En
    } else {
        Lang::ZhCn
    }
}

pub fn t(zh: &'static str, en: &'static str) -> &'static str {
    match current() {
        Lang::ZhCn => zh,
        Lang::En => en,
    }
}

fn override_lang() -> Option<Lang> {
    let value = std::env::var("LXWC_LANG").ok()?;
    let value = value.to_lowercase();
    if value.starts_with("zh") {
        Some(Lang::ZhCn)
    } else if value.starts_with("en") {
        Some(Lang::En)
    } else {
        None
    }
}

fn config_home() -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(dir);
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".config");
    }
    PathBuf::from(".config")
}

fn preference_path() -> PathBuf {
    config_home().join("lxwc-settings").join("language")
}

fn load_saved() -> Option<Lang> {
    let value = std::fs::read_to_string(preference_path()).ok()?;
    match value.trim().to_lowercase().as_str() {
        "zh" | "zh-cn" | "zh_cn" => Some(Lang::ZhCn),
        "en" | "en-us" | "en_us" => Some(Lang::En),
        _ => None,
    }
}

pub fn save(lang: Lang) {
    let path = preference_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let value = match lang {
        Lang::ZhCn => "zh",
        Lang::En => "en",
    };
    let _ = std::fs::write(path, value);
}

pub fn initial() -> Lang {
    if let Some(lang) = override_lang() {
        return lang;
    }
    load_saved().unwrap_or_else(Lang::from_env)
}

#[macro_export]
macro_rules! trf {
    ($zh:literal, $en:literal $(, $arg:expr)* $(,)?) => {{
        if $crate::i18n::current() == $crate::i18n::Lang::ZhCn {
            format!($zh $(, $arg)*)
        } else {
            format!($en $(, $arg)*)
        }
    }};
}
