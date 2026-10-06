// No JS-callable commands: the app's own Rust commands (list_serial_ports, ...) call this plugin
// directly, so nothing needs a webview permission. `android_path` tells Tauri where the Kotlin
// library lives so it is added to the generated Android project.
const COMMANDS: &[&str] = &[];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).android_path("android").build();
}
