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
  app.rs          Module routing, task aggregation, confirmation and apply state
  core/           Domain model: Effect, Task, TaskResult, ApplyReport
  services/       Generic file transactions and the background worker
  wlr/            WLR adapter: wlr-randr scanning, parsing, mode commands
  labwc/          Labwc adapter: rc.xml, environment, autostart, paths, reconfigure
  plugins/        UI-free presets such as LXQt commands
  modules/        UI and business state for each settings page
  ui/             Sidebar, status bar, and theme
  i18n.rs         Runtime translation loader
```

Dependency direction:

- `core` does not depend on UI or system adapters.
- `wlr` and `labwc` are peer external-system adapters.
- `modules` contain UI and state, but no direct file writes.
- `app` collects tasks and routes module requests.
- Persistence always goes through `Effect::WriteFile`.

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
- Translations: [`locales/`](locales/)

## License

GPL-3.0-or-later. See [`LICENSE`](LICENSE).
