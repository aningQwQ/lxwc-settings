use std::collections::BTreeSet;
use std::path::PathBuf;

fn data_home() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from(".local/share"))
}

fn data_dirs() -> Vec<PathBuf> {
    if let Some(value) = std::env::var_os("XDG_DATA_DIRS") {
        let parsed = std::env::split_paths(&value).collect::<Vec<_>>();
        if !parsed.is_empty() {
            return parsed;
        }
    }
    vec![
        PathBuf::from("/usr/local/share"),
        PathBuf::from("/usr/share"),
    ]
}

fn theme_dirs(kind: Kind) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    match kind {
        Kind::Window => {
            if let Some(home) = std::env::var_os("HOME") {
                paths.push(PathBuf::from(home).join(".themes"));
            }
        }
        Kind::Icon | Kind::Cursor => {
            if let Some(home) = std::env::var_os("HOME") {
                paths.push(PathBuf::from(home).join(".icons"));
            }
        }
    }
    for dir in std::iter::once(data_home()).chain(data_dirs()) {
        paths.push(match kind {
            Kind::Window => dir.join("themes"),
            Kind::Icon | Kind::Cursor => dir.join("icons"),
        });
    }
    paths
}

#[derive(Clone, Copy)]
enum Kind {
    Window,
    Icon,
    Cursor,
}

pub fn window_themes() -> Vec<String> {
    scan(Kind::Window)
}

pub fn icon_themes() -> Vec<String> {
    scan(Kind::Icon)
}

pub fn cursor_themes() -> Vec<String> {
    scan(Kind::Cursor)
}

fn scan(kind: Kind) -> Vec<String> {
    let mut names = BTreeSet::new();
    for dir in theme_dirs(kind) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let include = match kind {
                Kind::Window => has_subdir(&path, "openbox-3") || has_subdir(&path, "labwc"),
                Kind::Icon => !only_contains_cursors(&path),
                Kind::Cursor => has_subdir(&path, "cursors"),
            };
            if include {
                names.insert(name.to_string());
            }
        }
    }
    let mut themes = names.into_iter().collect::<Vec<_>>();
    themes.sort_by_key(|name| name.to_lowercase());
    themes
}

fn has_subdir(path: &PathBuf, name: &str) -> bool {
    path.join(name).is_dir()
}

fn only_contains_cursors(path: &PathBuf) -> bool {
    let Ok(entries) = std::fs::read_dir(path) else {
        return false;
    };
    let dirs = entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| entry.file_name().to_str().map(str::to_string))
        .collect::<Vec<_>>();
    dirs.len() == 1 && dirs[0] == "cursors"
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("lxwc-theme-test-{unique}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn cursor_only_icon_theme_is_skipped() {
        let dir = temp_dir();
        let cursor_only = dir.join("CursorOnly");
        std::fs::create_dir_all(cursor_only.join("cursors")).unwrap();
        let full = dir.join("FullTheme");
        std::fs::create_dir_all(full.join("actions")).unwrap();

        assert!(only_contains_cursors(&cursor_only));
        assert!(!only_contains_cursors(&full));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn window_theme_requires_openbox_or_labwc_dir() {
        let dir = temp_dir();
        let openbox = dir.join("OpenboxTheme");
        std::fs::create_dir_all(openbox.join("openbox-3")).unwrap();
        let plain = dir.join("Plain");
        std::fs::create_dir_all(&plain).unwrap();

        assert!(has_subdir(&openbox, "openbox-3"));
        assert!(!has_subdir(&plain, "openbox-3"));
        std::fs::remove_dir_all(dir).ok();
    }
}
