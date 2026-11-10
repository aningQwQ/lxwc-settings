//! Dependency-direction guards.
//!
//! Rust gives no visibility boundary inside a binary crate, so these tests keep
//! the layering honest: pages must not learn about plugins, plugins must not
//! import each other, and backends must not import pages or UI.

use std::fs;
use std::path::{Path, PathBuf};

fn sources(root: &str) -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                out.push(path);
            }
        }
    }

    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join(root);
    let mut out = Vec::new();
    walk(&base, &mut out);
    out
}

fn assert_absent(root: &str, needles: &[&str]) {
    for path in sources(root) {
        let text = fs::read_to_string(&path).expect("read source");
        for needle in needles {
            assert!(
                !text.contains(needle),
                "{} must not reference `{}`",
                path.display(),
                needle
            );
        }
    }
}

#[test]
fn generic_pages_do_not_depend_on_plugins() {
    assert_absent("src/pages", &["crate::plugins", "/labwc_config"]);
}

#[test]
fn config_layer_does_not_depend_on_pages_or_plugins() {
    assert_absent("src/config", &["crate::plugins", "crate::modules"]);
}

#[test]
fn core_does_not_depend_on_pages_plugins_or_modules() {
    assert_absent(
        "src/core",
        &["crate::plugins", "crate::modules", "crate::pages"],
    );
}

#[test]
fn plugins_do_not_import_each_other() {
    let plugins = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/plugins");
    for entry in fs::read_dir(&plugins).expect("src/plugins").flatten() {
        if !entry.path().is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().to_string();
        for path in sources(&format!("src/plugins/{id}")) {
            let text = fs::read_to_string(&path).expect("read source");
            for other in fs::read_dir(&plugins).expect("src/plugins").flatten() {
                let other_id = other.file_name().to_string_lossy().to_string();
                if other_id == id || !other.path().is_dir() {
                    continue;
                }
                let needle = format!("crate::plugins::{other_id}");
                assert!(
                    !text.contains(&needle),
                    "{} must not import plugin `{}`",
                    path.display(),
                    other_id
                );
            }
        }
    }
}

#[test]
fn backend_adapters_do_not_depend_on_pages_or_ui() {
    let labwc = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/plugins/labwc");
    for entry in fs::read_dir(&labwc).expect("src/plugins/labwc").flatten() {
        let path = entry.path();
        if !path.is_file()
            || path.file_name().is_none_or(|name| name == "mod.rs")
            || path.extension().is_none_or(|ext| ext != "rs")
        {
            continue;
        }
        let text = fs::read_to_string(&path).expect("read source");
        assert!(
            !text.contains("crate::ui::"),
            "adapter {} must not depend on UI",
            path.display()
        );
        assert!(
            !text.contains("crate::modules::"),
            "adapter {} must not depend on page contracts",
            path.display()
        );
    }
}

#[test]
fn output_provider_stays_headless() {
    assert_absent(
        "src/plugins/wlr",
        &["crate::ui", "crate::modules", "crate::pages"],
    );
}
