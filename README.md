# Soundstoic

Menu bar agent that pins the system default input device to a user-selected mic using Core Audio HAL property listeners (no polling).

## Features

- Lock the system default input to a selected device UID
- Reacts immediately to device/default changes (property listeners)
- Menu bar only, no windows
- Optional Start at Login toggle (SMAppService)

## Build

```bash
cargo build
```

## Run (dev)

```bash
cargo run
```

You should see a menu bar icon (mic). If the icon is unavailable, it falls back to the "MicLock" title.

## How to use

1. Click the menu bar icon.
2. Open "Select Locked Mic..." and choose the input device you want to pin.
3. Toggle "Input Lock" on.
4. Connect Bluetooth or other devices; the app will immediately restore the locked input if macOS changes it.

### Status items

- "Current Input" shows the system default input name.
- "Locked Input" shows the saved device (and "(missing)" if it is unplugged).

## Start at Login

The toggle uses `SMAppService` (macOS 13+). It requires a bundled app with a valid bundle identifier.
When running from `cargo run`, Start at Login will not persist across reboots.

If you want this to work in practice:

1. Bundle the app with your real bundle ID in `resources/Info.plist`.
2. Sign the app (even ad-hoc is fine for local use).
3. Launch the bundled app once, then enable Start at Login.

## Configuration

Config is stored here:

```
~/Library/Application Support/soundstoic/config.json
```

Fields:

- `lock_enabled`: true/false
- `locked_uid`: string or null
- `start_at_login`: true/false

To reset, delete the file and relaunch the app.

## Troubleshooting

- No input devices listed:
  - Ensure System Settings -> Sound -> Input shows at least one device.
  - Some devices report zero channels but still provide input streams; the app falls back to stream checks, so it should still show up. If it does not, report the device name and I will add a specific workaround.

- App crashes on launch:
  - The app reads Core Audio properties directly. If you have unusual virtual devices, try unplugging and relaunching.

- No mic permission prompt:
  - This app does not open input streams, so it should not trigger the microphone permission dialog.

## Bundle metadata

`resources/Info.plist` sets the app to be an agent (no Dock icon):

- `LSUIElement` = true
- `CFBundleIdentifier` should be changed from `com.example.soundstoic` before distribution

## Notes

- The menu bar icon uses the system symbol `mic` as a template image.
- If the symbol API is unavailable, the app falls back to the title text.
