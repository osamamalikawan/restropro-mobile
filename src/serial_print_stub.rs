//! Android placeholder for serial/Bluetooth-SPP printing. Same command names and return shapes as
//! serial_print.rs so the web app does not hit "command not found"; it just reports that the
//! port list is empty. Bluetooth printing on Android needs a native (Kotlin) plugin: not done yet.
use serde::Serialize;

#[derive(Serialize)]
pub struct SerialPortEntry {
    name: String,
    description: String,
    bluetooth: bool,
}

#[derive(Serialize)]
pub struct SerialPortStatus {
    present: bool,
    detail: String,
}

const MSG: &str = "Serial / Bluetooth COM-port printing is not available in the Android app yet. Use a network (Wi-Fi/LAN) printer.";

#[tauri::command]
pub fn list_serial_ports() -> Result<Vec<SerialPortEntry>, String> {
    Ok(Vec::new())
}

#[tauri::command]
pub fn check_serial_port(port: String) -> Result<SerialPortStatus, String> {
    let _ = port;
    Ok(SerialPortStatus { present: false, detail: MSG.to_string() })
}

#[tauri::command]
pub async fn print_raw_serial(port: String, baud: u32, data: Vec<u8>) -> Result<(), String> {
    let _ = (port, baud, data);
    Err(MSG.to_string())
}
