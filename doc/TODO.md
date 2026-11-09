# TODO

No outstanding core tasks.

## Completed

- Appearance, window behaviour, mouse/touchpad, keyboard, and touchscreen settings.
- Custom pointer acceleration curves.
- `layout(variant)` XKB merge with `XKB_DEFAULT_VARIANT` cleared.
- Dynamic multi-output persistence through the kanshi config.
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
