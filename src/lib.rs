// Restro Pro POS shell (Windows and Android).
//
// Beyond showing the web app in a window, this app gives the page a way to open a raw TCP
// socket to a network thermal printer and write ESC/POS bytes to it, which plain browser
// JavaScript cannot do. UI and business logic stay in the Next.js app.
use std::io::Write;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const WRITE_TIMEOUT: Duration = Duration::from_secs(5);

fn resolve(ip: &str, port: u16) -> Result<SocketAddr, String> {
    format!("{ip}:{port}")
        .to_socket_addrs()
        .map_err(|e| format!("Could not resolve {ip}:{port}. {e}"))?
        .next()
        .ok_or_else(|| format!("No address found for {ip}:{port}"))
}

/// Opens a raw TCP connection to a network printer, writes the ESC/POS bytes, and closes it.
/// Works the same on Windows and Android: no drivers, no print queue.
#[tauri::command]
fn print_raw(ip: String, port: u16, data: Vec<u8>) -> Result<(), String> {
    let addr = resolve(&ip, port)?;
    let mut stream = TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT).map_err(|e| {
        format!("Could not reach the printer at {ip}:{port}. {e}. Check the IP, that the printer is on, and that this device is on the same network.")
    })?;
    stream
        .set_write_timeout(Some(WRITE_TIMEOUT))
        .map_err(|e| format!("Could not configure the connection. {e}"))?;
    stream
        .write_all(&data)
        .map_err(|e| format!("Connected to {ip}:{port} but the print job failed partway. {e}"))?;
    Ok(())
}

/// Reachability check for the settings page: connects and drops the socket without sending
/// anything, so it works on printers that have no query protocol.
#[tauri::command]
fn test_printer_connection(ip: String, port: u16) -> Result<(), String> {
    let addr = resolve(&ip, port)?;
    TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT)
        .map(|_| ())
        .map_err(|e| format!("Could not reach the printer at {ip}:{port}. {e}"))
}

#[tauri::command]
fn list_usb_printers() -> Result<Vec<String>, String> {
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    {
        use nusb::MaybeFuture;
        let devices = nusb::list_devices().wait().map_err(|e| format!("Failed to list USB devices: {e}"))?;
        let mut list = Vec::new();
        for dev in devices {
            let name = match (dev.manufacturer_string(), dev.product_string()) {
                (Some(m), Some(p)) => format!("{m} {p}"),
                (None, Some(p)) => p.to_string(),
                (Some(m), None) => format!("{m} USB Device"),
                (None, None) => format!("USB Device {:04x}:{:04x}", dev.vendor_id(), dev.product_id()),
            };
            list.push(format!("{name} ({:04x}:{:04x})", dev.vendor_id(), dev.product_id()));
        }
        Ok(list)
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        Ok(Vec::new())
    }
}

#[tauri::command]
fn print_raw_usb(printer_identifier: String, data: Vec<u8>) -> Result<(), String> {
    let _ = (printer_identifier, data);
    Err("Direct raw USB printing is not configured. On Windows, use Windows printer spooler commands.".to_string())
}

// Printing through the Windows print spooler (USB printers, by Windows printer name).
// Your three existing Windows commands live in src/winprint.rs and are compiled on Windows
// only. Every other platform gets these stubs, which return a clear error, so the same code
// builds for Android.
#[cfg(windows)]
mod winprint;

#[cfg(not(windows))]
mod winprint {
    use serde::Serialize;

    #[derive(Serialize)]
    pub struct WindowsPrinterStatus {
        pub installed: bool,
        pub online: bool,
        pub detail: String,
    }

    const MSG: &str = "Windows printers are only available in the Windows app.";

    #[tauri::command]
    pub fn list_windows_printers() -> Result<Vec<String>, String> {
        Err(MSG.to_string())
    }

    #[tauri::command]
    pub fn check_windows_printer(printer_name: String) -> Result<WindowsPrinterStatus, String> {
        let _ = printer_name;
        Err(MSG.to_string())
    }

    #[tauri::command]
    pub fn print_raw_windows(printer_name: String, data: Vec<u8>) -> Result<(), String> {
        let _ = (printer_name, data);
        Err(MSG.to_string())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            print_raw,
            test_printer_connection,
            winprint::list_windows_printers,
            winprint::check_windows_printer,
            winprint::print_raw_windows,
            list_usb_printers,
            print_raw_usb
        ])
        .run(tauri::generate_context!())
        .expect("error while running Restro Pro POS");
}
