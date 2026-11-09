use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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

    fn file_name(self) -> &'static str {
        match self {
            Self::ZhCn => "zh-CN.lang",
            Self::En => "en-US.lang",
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

static TABLES: OnceLock<HashMap<Lang, HashMap<String, &'static str>>> = OnceLock::new();

pub fn t(key: &'static str) -> &'static str {
    let tables = tables();
    if let Some(value) = tables.get(&current()).and_then(|table| table.get(key)) {
        return value;
    }
    if let Some(value) = tables.get(&Lang::En).and_then(|table| table.get(key)) {
        return value;
    }
    key
}

pub fn format_template(template: &str, args: &[&dyn std::fmt::Display]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut rest = template;
    let mut index = 0usize;

    while let Some(position) = rest.find("{}") {
        out.push_str(&rest[..position]);
        if let Some(arg) = args.get(index) {
            use std::fmt::Write;
            let _ = write!(out, "{arg}");
        } else {
            out.push_str("{}");
        }
        index += 1;
        rest = &rest[position + 2..];
    }

    out.push_str(rest);
    out
}

fn tables() -> &'static HashMap<Lang, HashMap<String, &'static str>> {
    TABLES.get_or_init(|| {
        let mut tables = HashMap::new();
        for lang in Lang::ALL {
            tables.insert(lang, load_translations(lang.file_name(), lang));
        }
        tables
    })
}

fn load_translations(file: &str, lang: Lang) -> HashMap<String, &'static str> {
    let mut translations = parse_locale(embedded_locale(lang));

    if let Some(path) = find_locale_file(file) {
        if let Ok(content) = std::fs::read_to_string(&path) {
            for (key, translation) in parse_locale(&content) {
                translations.insert(key, translation);
            }
        }
    }

    translations
}

fn find_locale_file(file: &str) -> Option<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(dir) = std::env::var_os("LXWC_LOCALE_DIR") {
        candidates.push(PathBuf::from(dir).join(file));
    }
    if let Some(dir) = std::env::var_os("XDG_CONFIG_HOME") {
        candidates.push(
            PathBuf::from(dir)
                .join("lxwc-settings")
                .join("locales")
                .join(file),
        );
    }
    if let Some(home) = std::env::var_os("HOME") {
        candidates.push(
            PathBuf::from(home)
                .join(".config")
                .join("lxwc-settings")
                .join("locales")
                .join(file),
        );
    }
    if let Ok(executable) = std::env::current_exe() {
        if let Some(dir) = executable.parent() {
            candidates.push(dir.join("locales").join(file));
        }
    }
    candidates.push(Path::new("locales").join(file));
    candidates.push(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("locales")
            .join(file),
    );

    candidates.into_iter().find(|path| path.is_file())
}

fn embedded_locale(lang: Lang) -> &'static str {
    match lang {
        Lang::ZhCn => include_str!("../locales/zh-CN.lang"),
        Lang::En => include_str!("../locales/en-US.lang"),
    }
}

fn parse_locale(content: &str) -> HashMap<String, &'static str> {
    let mut translations = HashMap::new();

    for line in content.lines() {
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let Some((key, translation)) = line.split_once('\t') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() || translation.trim().is_empty() {
            continue;
        }
        let translation: &'static str = Box::leak(translation.to_string().into_boxed_str());
        translations.insert(key.to_string(), translation);
    }

    translations
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
    ($key:literal $(, $arg:expr)* $(,)?) => {
        $crate::i18n::format_template(
            $crate::i18n::t($key),
            &[$( &$arg as &dyn ::std::fmt::Display ),*] as &[&dyn ::std::fmt::Display],
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogs_load() {
        let tables = tables();
        let english = tables.get(&Lang::En).expect("English catalog");
        let chinese = tables.get(&Lang::ZhCn).expect("Chinese catalog");
        assert_eq!(english.get("lxwc_settings").copied(), Some("LXWC Settings"));
        assert_eq!(chinese.get("lxwc_settings").copied(), Some("LXWC 设置"));
        assert_eq!(english.get("key_bindings").copied(), Some("Key Bindings"));
        assert_eq!(english.get("current").copied(), Some(" (current)"));
        assert!(english.contains_key("failed_to_read"));
    }

    #[test]
    fn runtime_language_lookup() {
        set_current(Lang::En);
        assert_eq!(t("lxwc_settings"), "LXWC Settings");
        assert_eq!(
            crate::trf!("failed_to_read", "a", "b"),
            "Failed to read a: b"
        );
        set_current(Lang::ZhCn);
        assert_eq!(t("lxwc_settings"), "LXWC 设置");
        assert_eq!(crate::trf!("failed_to_read", "a", "b"), "读取 a 失败：b");
    }

    #[test]
    fn format_template_substitutes_placeholders() {
        let args: [&dyn std::fmt::Display; 2] = [&"a", &7];
        assert_eq!(format_template("{} → {}", &args), "a → 7");
        assert_eq!(format_template("no placeholders", &[]), "no placeholders");
        assert_eq!(format_template("{} {} {}", &args), "a 7 {}");
    }
}
