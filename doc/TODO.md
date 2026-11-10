# TODO

No outstanding core tasks.

## Completed

- Moved the live output page to `plugins/wlr_ui`; `wlr` is now an optional
  prerequisite feature (`wlr-ui = ["wlr"]`) and the default build is unchanged.
- Plugin model: everything optional lives in `plugins/` behind a `Plugin`
  manifest (`provides`/`requires`/`pages`/`groups`), with `wlr` as a headless
  provider and `wlr_ui` as a page plugin that consumes `core::outputs::Outputs`.
  The registry disables plugins with unmet requirements and the shell-only build
  (`--no-default-features`) runs without any backend.
- `SettingsModule` takes `ConfigEdit.file_id` and a single `patch(file_id, raw)` hook;
  `app` aggregates one task per backend file instead of two hardcoded files.
- Architecture tests enforce page/backend/adapter dependency directions.

- Appearance, window behaviour, mouse/touchpad, keyboard, and touchscreen settings.
- Custom pointer acceleration curves.
- `layout(variant)` XKB merge with `XKB_DEFAULT_VARIANT` cleared.
- Cross-module rc.xml/environment aggregation into one atomic write per file.
- Single comment block when appending multiple environment variables.
- Custom ComboBox values remain visible when outside the preset list.
- Ten-layout fallback when `evdev.lst` is unavailable.

## Not planned

- About/system information page. Use existing system tools such as `fastfetch`.
- Full labwc-tweaks `grp:*` preset list. The nine common presets and free-text input remain.
- Percentage-scaled Pointer Speed UI. The raw `-1.0..1.0` float is kept.
- Forced `Adwaita` theme fallback. Only installed themes are listed.
- Developer template page.

## Verification

```bash
cargo fmt
cargo build --offline
cargo test --offline
cargo build --release --offline
```
