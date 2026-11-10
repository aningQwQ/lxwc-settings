# lxwc-settings

English | [简体中文](README.zh-CN.md)

A lightweight settings center for Labwc and LXQt, written in Rust with egui.

`lxwc-settings` edits Labwc's `rc.xml`, `environment`, and `autostart` files while
providing WLR output controls that are not exposed by Labwc's configuration
files. Changes are validated, backed up, and written atomically.

## Features

- **Display and outputs (WLR)**  
  Scan outputs and modes with `wlr-randr`, switch modes, and automatically roll
  back after 15 seconds unless the new mode is confirmed.

- **Global shortcuts**  
  View, add, edit, and delete Labwc keybindings directly in `rc.xml`, including
  key combinations, actions, attributes, and advanced XML fragments.

- **Default display configuration**  
  Persist the selected output and mode as a `wlr-randr` command in Labwc's
  `autostart`.

- **Appearance**  
  Window theme, icon theme, corner radius, drop shadows, decoration mode, and
  maximized decoration.

- **Window behaviour**  
  Placement policy, focus behaviour, gap, snapping, edge resistance, resizing,
  and magnifier settings.

- **Mouse and touchpad**  
  libinput settings, cursor theme and size, and custom pointer acceleration
  curves (`motionCurve`, `scrollCurve`, `fallbackCurve`).

- **Keyboard**  
  XKB layouts, variants, switch options, repeat rate/delay, and NumLock.

- **Touchscreen**  
  Calibration matrix selection.

- **Environment tweaks**  
  Streaming and compatibility switches such as `WLR_DRM_NO_MODIFIERS` and
  `WLR_NO_HARDWARE_CURSORS`.

- **LXQt presets**  
  Preset command providers used by the desktop integration.

- **Safety**  
  Every write is validated and applied through a single file transaction with
  `.lxwc.bak` backups, atomic replacement, external-change detection, and retry
  for failed tasks.

- **Internationalization**  
  English and Simplified Chinese translations are loaded from external
  `locales/*.lang` files.

## Requirements

- Linux with a Wayland session running Labwc
- A recent stable Rust toolchain (edition 2021)
- `wlr-randr` for display scanning and mode switching
- `labwc` in `PATH` for reloading configuration
- Optional: `fc-match`/fontconfig for automatic CJK font loading

## Build and run

```bash
cargo build --release
./target/release/lxwc-settings
```

For development:

```bash
cargo run
```

Install from the source tree:

```bash
cargo install --path .
```

## Configuration

By default, the application reads and writes:

- `~/.config/labwc/rc.xml`
- `~/.config/labwc/environment`
- `~/.config/labwc/autostart`

The following environment variables override the defaults:

| Variable | Purpose |
| --- | --- |
| `LXWC_CONFIG_DIR` | Labwc configuration directory |
| `LXWC_RC_XML` | Path to `rc.xml` |
| `LXWC_ENVIRONMENT` | Path to `environment` |
| `LXWC_AUTOSTART` | Path to `autostart` |
| `LXWC_LANG` | Force `zh` or `en` |
| `LXWC_LOCALE_DIR` | Additional locale directory |
| `LXWC_XKB_RULES` | Alternative `evdev.lst` path |

Changes to `rc.xml` trigger Labwc reconfiguration. Environment changes take
effect after a new session.

Before replacing a file, the current contents are copied to a `.lxwc.bak` file.
Temporary files use the `.lxwc.tmp` suffix.

## Architecture

```text
src/
  app.rs          Shell: task aggregation, confirmation, apply state
  backend.rs      Backend/Group description the shell consumes
  plugin.rs       Plugin manifest: provides, requires, backend, pages, groups
  plugins/        Every optional unit, plus the registry
    mod.rs        Detection, requirement resolution, composition
    labwc/        Backend plugin (requires `wlr.outputs`): adapters, formats, pages
    wlr/          Output provider plugin: owns the `wlr.outputs` contract
    wlr_ui/       Output page plugin: adapts to `wlr.outputs`
  config/         ConfigFile / ConfigFormat / ConfigEdit / PatchOutcome
  modules/        SettingsModule contract shared by every page
  pages/          Reserved for compositor-neutral pages (currently empty)
  services/       Generic file transactions and the background worker
  core/           Domain model: Effect, Task, ApplyReport
  ui/             Sidebar, status bar, and theme
  i18n.rs         Runtime translation loader
```

Dependency direction:

- `core` and `config` know nothing about plugins, pages or UI.
- A capability belongs to the provider that declares it, with a namespaced id
  (`wlr.outputs`). Consumers `require` that id and import the provider module;
  changing providers means adapting the consumer to the new contract.
- A provider is headless and imports no other plugin. Only `plugins/mod.rs`
  references all plugins.
- `plugin_imports_are_backed_by_requires` checks that an import of another
  plugin is backed by a declared requirement.
- `pages/` must not import `crate::plugins` or name a backend key space.
- The registry enables a plugin only when its `requires` are provided; a missing
  provider disables the plugin instead of running half of it.
- `modules/` is only the page contract: pages name a backend file id, never a path.
- `app` reads the resolved backend file list, aggregates one task per file, and
  routes page requests. It never names a plugin.
- Persistence always goes through `Effect::WriteFile` / `Effect::RunCommand`.
- `src/architecture_tests.rs` enforces these edges at test time.

`LXWC_BACKEND` forces a backend plugin by id.

## Optional components

| Feature | Contents | Default |
| --- | --- | --- |
| `labwc` | Labwc backend plugin; pulls `wlr` | on |
| `wlr` | `wlr-randr` provider: owns `wlr.outputs` | on (via `labwc`) |
| `wlr-ui` | Output page plugin; requires `wlr.outputs` | on |

`cargo build --no-default-features` builds the shell with no plugins and shows
"No compositor backend or plugins are enabled". Dropping `wlr` requires dropping
`wlr-ui` and `labwc`; Cargo enforces the dependency edges.

## Testing

```bash
cargo fmt --check
cargo build
cargo test
cargo build --release
```

Tests cover XML patching, keybindings, environment editing, scalar settings,
themes, XKB layout parsing, acceleration curves, display parsing, and file
transaction safety.

An optional test validates patches against a real configuration:

```bash
LXWC_TEST_RC="$HOME/.config/labwc/rc.xml" cargo test real_config
```

## Documentation

- Remaining work: [`doc/TODO.md`](doc/TODO.md)
- Development guidelines: [`doc/守则.md`](doc/守则.md)
- Translations: [`locales/`](locales/)

## License

GPL-3.0-or-later. See [`LICENSE`](LICENSE).
