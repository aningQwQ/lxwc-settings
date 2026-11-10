//! Dependency-direction guards.
//!
//! Rust gives no visibility boundary inside a binary crate, so these tests keep
//! the layering honest: generic pages must not learn about a backend, backend
//! adapters must not learn about pages or UI, and backends must not import each
//! other.

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
fn generic_pages_do_not_depend_on_a_backend() {
    assert_absent("src/pages", &["crate::backends", "/labwc_config"]);
}

#[test]
fn config_layer_does_not_depend_on_pages_or_backends() {
    assert_absent("src/config", &["crate::backends", "crate::modules"]);
}

#[test]
fn core_does_not_depend_on_pages_or_backends() {
    assert_absent(
        "src/core",
        &["crate::backends", "crate::modules", "crate::pages"],
    );
}

#[test]
fn backends_do_not_import_each_other() {
    let backends = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/backends");
    let entries = fs::read_dir(&backends).expect("src/backends");
    for entry in entries.flatten() {
        if !entry.path().is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().to_string();
        let prefix = format!("src/backends/{id}");
        for path in sources(&prefix) {
            let text = fs::read_to_string(&path).expect("read source");
            for other in fs::read_dir(&backends).expect("src/backends") {
                let other = other.expect("backend entry");
                let other_id = other.file_name().to_string_lossy().to_string();
                if other_id == id || !other.path().is_dir() {
                    continue;
                }
                let needle = format!("crate::backends::{other_id}");
                assert!(
                    !text.contains(&needle),
                    "{} must not import backend `{}`",
                    path.display(),
                    other_id
                );
            }
        }
    }
}

#[test]
fn backend_adapters_do_not_depend_on_pages_or_ui() {
    let labwc = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/backends/labwc");
    let entries = fs::read_dir(&labwc).expect("src/backends/labwc");
    for entry in entries.flatten() {
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
