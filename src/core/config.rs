use std::path::{Path, PathBuf};

use super::env_file;
use super::keybinds;
use super::model::Baseline;

pub fn config_dir() -> PathBuf {
    std::env::var_os("LXWC_CONFIG_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("XDG_CONFIG_HOME").map(|dir| PathBuf::from(dir).join("labwc"))
        })
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config/labwc")))
        .unwrap_or_else(|| PathBuf::from(".config/labwc"))
}

pub fn rc_path() -> PathBuf {
    std::env::var_os("LXWC_RC_XML")
        .map(PathBuf::from)
        .unwrap_or_else(|| config_dir().join("rc.xml"))
}

pub fn environment_path() -> PathBuf {
    std::env::var_os("LXWC_ENVIRONMENT")
        .map(PathBuf::from)
        .unwrap_or_else(|| config_dir().join("environment"))
}

pub fn read_optional(path: &Path) -> Result<(String, bool), String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok((text, true)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok((String::new(), false)),
        Err(err) => Err(format!("读取 {} 失败：{err}", path.display())),
    }
}

pub fn load_baseline() -> Result<Baseline, String> {
    let rc_path = rc_path();
    let env_path = environment_path();
    let (rc_raw, rc_exists) = read_optional(&rc_path)?;
    let (env_raw, env_exists) = read_optional(&env_path)?;
    let keybinds = keybinds::parse(&rc_raw)?;
    Ok(Baseline {
        rc_path,
        env_path,
        rc_exists,
        env_exists,
        environment: env_file::parse_active(&env_raw),
        keybinds,
    })
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
            .map_err(|err| format!("创建目录 {} 失败：{err}", parent.display()))?;
    }

    if path.exists() {
        let backup = with_suffix(path, ".lxwc.bak");
        std::fs::copy(path, &backup)
            .map_err(|err| format!("备份 {} 失败：{err}", path.display()))?;
    }

    let tmp = with_suffix(path, ".lxwc.tmp");
    std::fs::write(&tmp, contents)
        .map_err(|err| format!("写入临时文件 {} 失败：{err}", tmp.display()))?;

    if let Ok(meta) = std::fs::metadata(path) {
        let _ = std::fs::set_permissions(&tmp, meta.permissions());
    }

    std::fs::rename(&tmp, path).map_err(|err| {
        let _ = std::fs::remove_file(&tmp);
        format!("替换 {} 失败：{err}", path.display())
    })
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}
