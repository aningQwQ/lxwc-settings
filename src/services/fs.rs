use std::path::{Path, PathBuf};

pub struct LoadedFile {
    pub raw: String,
    pub exists: bool,
}

pub fn load(path: &Path) -> Result<LoadedFile, String> {
    let (raw, exists) = read_optional(path)?;
    Ok(LoadedFile { raw, exists })
}

pub fn read_optional(path: &Path) -> Result<(String, bool), String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok((text, true)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok((String::new(), false)),
        Err(err) => Err(crate::trf!("failed_to_read", path.display(), err)),
    }
}

pub fn write_file_if_unchanged(
    path: &Path,
    expected: Option<&str>,
    contents: &str,
) -> Result<(), String> {
    let (current, exists) = read_optional(path)?;
    let changed = match expected {
        Some(expected) => !exists || current != expected,
        None => exists,
    };
    if changed {
        return Err(crate::trf!("file_changed_externally", path.display()));
    }
    atomic_write(path, contents)
}

pub fn atomic_write(path: &Path, contents: &str) -> Result<(), String> {
    let path = if path.exists() {
        std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
    } else {
        path.to_path_buf()
    };
    let path = path.as_path();

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| crate::trf!("failed_to_create_directory", parent.display(), err))?;
    }

    if path.exists() {
        let backup = with_suffix(path, ".lxwc.bak");
        std::fs::copy(path, &backup)
            .map_err(|err| crate::trf!("failed_to_back_up", path.display(), err))?;
    }

    let tmp = with_suffix(path, ".lxwc.tmp");
    std::fs::write(&tmp, contents)
        .map_err(|err| crate::trf!("failed_to_write_temporary_file", tmp.display(), err))?;

    if let Ok(meta) = std::fs::metadata(path) {
        let _ = std::fs::set_permissions(&tmp, meta.permissions());
    }

    std::fs::rename(&tmp, path).map_err(|err| {
        let _ = std::fs::remove_file(&tmp);
        crate::trf!("failed_to_replace", path.display(), err)
    })
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path() -> (PathBuf, PathBuf) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("lxwc-config-test-{unique}"));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.txt");
        (dir, path)
    }

    #[test]
    fn creates_file_when_expected_missing() {
        let (dir, path) = temp_path();
        write_file_if_unchanged(&path, None, "new").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn rejects_external_change() {
        let (dir, path) = temp_path();
        std::fs::write(&path, "external").unwrap();
        assert!(write_file_if_unchanged(&path, Some("snapshot"), "new").is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "external");
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn rejects_created_file_when_expected_missing() {
        let (dir, path) = temp_path();
        std::fs::write(&path, "external").unwrap();
        assert!(write_file_if_unchanged(&path, None, "new").is_err());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn updates_when_snapshot_matches() {
        let (dir, path) = temp_path();
        std::fs::write(&path, "old").unwrap();
        write_file_if_unchanged(&path, Some("old"), "new").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
        std::fs::remove_dir_all(dir).ok();
    }
}
