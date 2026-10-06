//! Android version of the serial/COM commands, backed by classic Bluetooth (SPP).
//!
//! Same command names and JSON shapes as serial_print.rs (Windows), so the web app's
//! "Bluetooth / COM" printer setting works unchanged. The "port" is the printer's Bluetooth MAC
//! address, which the Android plugin takes from the list of printers paired in Android settings.
//! `baud` is ignored (Bluetooth has no baud rate).
use serde::Serialize;
use tauri_plugin_bt_printer::BtPrinterExt;

#[derive(Serialize)]
pub struct SerialPortEntry {
    /// MAC address: what gets saved as the printer's port.
    name: String,
    /// Shown next to it in the picker: the device's Bluetooth name.
    description: String,
    bluetooth: bool,
}

#[derive(Serialize)]
pub struct SerialPortStatus {
    present: bool,
    detail: String,
}

// The Kotlin side answers on its own thread while we wait, so never call it from the main thread
// (sync commands run there): use async commands and a blocking worker.
async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| format!("Bluetooth task failed — {e}"))?
}

#[tauri::command]
pub async fn list_serial_ports(app: tauri::AppHandle) -> Result<Vec<SerialPortEntry>, String> {
    blocking(move || {
        let devices = app.bt_printer().list_paired().map_err(|e| e.to_string())?;
        Ok(devices
            .into_iter()
            .map(|d| SerialPortEntry { name: d.address, description: d.name, bluetooth: true })
            .collect())
    })
    .await
}

#[tauri::command]
pub async fn check_serial_port(app: tauri::AppHandle, port: String) -> Result<SerialPortStatus, String> {
    blocking(move || {
        let s = app.bt_printer().status(&port).map_err(|e| e.to_string())?;
        Ok(SerialPortStatus { present: s.present, detail: s.detail })
    })
    .await
}

#[tauri::command]
pub async fn print_raw_serial(app: tauri::AppHandle, port: String, baud: u32, data: Vec<u8>) -> Result<(), String> {
    let _ = baud;
    blocking(move || app.bt_printer().print(&port, &data).map_err(|e| e.to_string())).await
}
