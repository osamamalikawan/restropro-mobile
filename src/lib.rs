// Restro Pro POS shell (Windows + Android).
//
// Ported from the Windows shell (restropro-windows): offline-first local database, background
// sync, staff PIN login, API bridge, secure token store, and the app:// bundle server, plus the
// printing commands. Windows-only (spooler) and desktop-only (USB, COM/Bluetooth, OS keyring)
// pieces are cfg-gated; on Android those commands exist as stubs that return a clear message.
use std::io::Write;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

use tauri::Manager;

mod api_bridge;
mod bundle_updater;
mod local_db;
mod offline_api;
mod paths;
mod secure_store;
mod staff_auth;
mod sync;
mod usb_print;

#[cfg(not(target_os = "android"))]
mod serial_print;
#[cfg(target_os = "android")]
#[path = "serial_print_stub.rs"]
mod serial_print;

// Windows print spooler (USB printers by Windows name). Other platforms get stubs.
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

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const WRITE_TIMEOUT: Duration = Duration::from_secs(5);

fn resolve(ip: &str, port: u16) -> Result<SocketAddr, String> {
    format!("{ip}:{port}")
        .to_socket_addrs()
        .map_err(|e| format!("Could not resolve {ip}:{port} — {e}"))?
        .next()
        .ok_or_else(|| format!("No address found for {ip}:{port}"))
}

/// Raw TCP to a network thermal printer: no drivers, no print queue. Same on Windows and Android.
#[tauri::command]
fn print_raw(ip: String, port: u16, data: Vec<u8>) -> Result<(), String> {
    let addr = resolve(&ip, port)?;
    let mut stream = TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT).map_err(|e| {
        format!("Could not reach printer at {ip}:{port} — {e}. Check the IP, that the printer is powered on, and that this device is on the same network.")
    })?;
    stream
        .set_write_timeout(Some(WRITE_TIMEOUT))
        .map_err(|e| format!("Could not configure the connection — {e}"))?;
    stream
        .write_all(&data)
        .map_err(|e| format!("Connected to {ip}:{port} but the print job failed partway — {e}"))?;
    Ok(())
}

#[tauri::command]
fn test_printer_connection(ip: String, port: u16) -> Result<(), String> {
    let addr = resolve(&ip, port)?;
    TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT)
        .map(|_| ())
        .map_err(|e| format!("Could not reach printer at {ip}:{port} — {e}"))
}

fn resolve_static_file(base: &std::path::Path, raw_path: &str) -> Option<std::path::PathBuf> {
    let candidates = [
        base.join(raw_path),
        base.join(format!("{raw_path}.html")),
        base.join(raw_path).join("index.html"),
    ];
    candidates.into_iter().find(|p| p.is_file())
}

fn text_response(status: u16, body: &str) -> tauri::http::Response<Vec<u8>> {
    tauri::http::Response::builder()
        .status(status)
        .body(body.as_bytes().to_vec())
        .unwrap()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .register_uri_scheme_protocol("app", |_app, request| {
            if !paths::ready() {
                return text_response(503, "Starting up — try again in a moment.");
            }
            let raw_path = request.uri().path().trim_start_matches('/').to_string();
            let raw_path = if raw_path.is_empty() { "index.html".to_string() } else { raw_path };

            let Some(base) = bundle_updater::current_bundle_path() else {
                return text_response(
                    503,
                    "No app bundle downloaded yet — connect to the internet once to finish setup.",
                );
            };

            match resolve_static_file(&base, &raw_path) {
                Some(file_path) => {
                    let data = std::fs::read(&file_path).unwrap_or_default();
                    let mime = mime_guess::from_path(&file_path).first_or_octet_stream();
                    tauri::http::Response::builder()
                        .header("Content-Type", mime.as_ref())
                        .body(data)
                        .unwrap()
                }
                None => {
                    eprintln!("404: no match for {raw_path:?} under {base:?}");
                    let not_found = base.join("404.html");
                    if not_found.is_file() {
                        let data = std::fs::read(&not_found).unwrap_or_default();
                        tauri::http::Response::builder()
                            .status(404)
                            .header("Content-Type", "text/html")
                            .body(data)
                            .unwrap()
                    } else {
                        text_response(404, "not found")
                    }
                }
            }
        })
        .setup(|app| {
            // The data directory must exist before the database or bundle folder is touched
            // (on Android it comes from Tauri, so this cannot happen earlier).
            paths::init(app.handle())?;
            app.manage(local_db::init());
            app.manage(staff_auth::SessionState::default());

            // Background sync: uploads queued offline sales and refreshes the local snapshot
            // whenever the internet is reachable. Failing while offline is normal — ignore it.
            let sync_handle = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_secs(20));
                loop {
                    let _ = sync::run_sync(&sync_handle, false, false);
                    std::thread::sleep(Duration::from_secs(300));
                }
            });

            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let result = bundle_updater::check_and_update();
                match &result {
                    Ok(Some(v)) => println!("Updated bundle to {v}"),
                    Ok(None) => println!("Bundle already current"),
                    Err(e) => eprintln!("Update check failed (using cached bundle if any): {e}"),
                }

                if let Some(window) = handle.get_webview_window("main") {
                    if bundle_updater::current_bundle_path().is_some() {
                        let _ = window.eval("window.location.reload()");
                    }
                    let _ = window.show();
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            print_raw,
            test_printer_connection,
            winprint::list_windows_printers,
            winprint::check_windows_printer,
            winprint::print_raw_windows,
            serial_print::list_serial_ports,
            serial_print::check_serial_port,
            serial_print::print_raw_serial,
            usb_print::list_usb_printers,
            usb_print::print_raw_usb,
            secure_store::secure_set,
            secure_store::secure_get,
            secure_store::secure_delete,
            staff_auth::get_device_info,
            staff_auth::get_cached_staff_list,
            staff_auth::verify_staff_pin,
            staff_auth::activate_device,
            staff_auth::get_local_session,
            staff_auth::staff_logout,
            sync::sync_now,
            sync::get_sync_status,
            sync::get_cached_data,
            sync::search_local_customers,
            sync::create_local_sale,
            api_bridge::api_request,
            api_bridge::check_online
        ])
        .run(tauri::generate_context!())
        .expect("error while running Restro Pro POS");
}
