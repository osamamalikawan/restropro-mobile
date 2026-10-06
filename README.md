# Restro Pro POS — Windows + Android shell

Tauri v2 shell around the Restro Pro web app (`osamamalikawan/restropro`). The UI is a Next.js
static bundle downloaded from the web repo's `latest.json` and served locally at `app://`
(`http://app.localhost` on Android), so web updates reach the app without a new install.

The shell adds what a browser cannot do: offline-first local database + background sync, staff PIN
login, secure device-token storage, the `/api` bridge, and thermal-printer output.

| Feature | Windows | Android |
|---|---|---|
| Offline DB, sync, staff PIN, API bridge | yes | yes |
| Network (Wi-Fi/LAN) ESC/POS printer | yes | yes |
| Windows print spooler | yes | stub |
| Raw USB (nusb) | yes | stub |
| Bluetooth printer ("Bluetooth / COM") | COM port | classic Bluetooth plugin (`plugins/bt-printer`) |
| Device-token storage | OS credential store | app-private file |

## Version gate
`bundle_updater` refuses a bundle whose `min_shell_version` is higher than this app's version
(`Cargo.toml` and `tauri.conf.json`, kept equal). Bump both when the web repo raises it.

## Android build
Requires Android Studio (SDK + NDK), `JAVA_HOME`, and the Rust Android targets:

    rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
    cargo tauri android init
    cargo tauri android dev          # device/emulator
    cargo tauri android build --apk

`tauri.android.conf.json` points the window at `http://app.localhost/index.html`; the Android
capability (`capabilities/mobile.json`) allows that origin.

## Windows build
    cargo tauri build

## Bluetooth printing on Android
`plugins/bt-printer` is a Tauri plugin (Rust + Kotlin) that prints raw ESC/POS to a classic
Bluetooth (SPP / RFCOMM) receipt printer. It answers the same commands the web app already calls
(`list_serial_ports`, `check_serial_port`, `print_raw_serial`), so printer settings → "Bluetooth / COM"
works unchanged; the saved "port" is the printer's Bluetooth MAC address.

1. Pair the printer in **Android Settings → Bluetooth** first (PIN is usually 0000 or 1234). The app
   lists paired devices only; it never scans, so it needs no location permission.
2. On Android 12+ the first use asks for **Nearby devices** permission.
3. Switch the printer on and keep it close before printing.

Limits: classic Bluetooth only (BLE-only printers will not appear); one print job at a time; the
`baud` setting is ignored. Chunk size and pauses mirror the Windows COM-port path
(`serial_print.rs`); adjust `CHUNK`, `CHUNK_PAUSE_MS`, `DRAIN_MS` in `BtPrinterPlugin.kt` if a model
drops lines or cuts the end of a receipt.

If Gradle cannot find the plugin after pulling this in, re-run `cargo tauri android init`.
