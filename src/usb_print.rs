//! Raw USB printing (desktop only, via nusb). Android gets stubs with the same signatures.
use serde::Serialize;

#[derive(Serialize)]
pub struct UsbDeviceInfo {
    vendor_id: u16,
    product_id: u16,
    manufacturer: Option<String>,
    product: Option<String>,
    serial: Option<String>,
}

#[cfg(not(target_os = "android"))]
mod imp {
    use super::UsbDeviceInfo;
    use nusb::descriptors::TransferType;
    use nusb::transfer::{Bulk, Direction, Out};
    use nusb::MaybeFuture;
    use std::io::Write;

    pub fn list() -> Result<Vec<UsbDeviceInfo>, String> {
        let devices = nusb::list_devices()
            .wait()
            .map_err(|e| format!("Could not list USB devices — {e}"))?;
        Ok(devices
            .map(|d| UsbDeviceInfo {
                vendor_id: d.vendor_id(),
                product_id: d.product_id(),
                manufacturer: d.manufacturer_string().map(|s| s.to_string()),
                product: d.product_string().map(|s| s.to_string()),
                serial: d.serial_number().map(|s| s.to_string()),
            })
            .collect())
    }

    pub fn print(vendor_id: u16, product_id: u16, data: Vec<u8>) -> Result<(), String> {
        let device_info = nusb::list_devices()
            .wait()
            .map_err(|e| format!("Could not list USB devices — {e}"))?
            .find(|d| d.vendor_id() == vendor_id && d.product_id() == product_id)
            .ok_or_else(|| format!("USB device {vendor_id:04x}:{product_id:04x} is not connected"))?;

        let device = device_info.open().wait().map_err(|e| {
            format!("Could not open the USB device — {e}. On Windows it usually needs to be bound to the generic WinUSB driver via Zadig first.")
        })?;

        let mut target: Option<(u8, u8)> = None;
        'search: for config in device.configurations() {
            for iface in config.interfaces() {
                for alt in iface.alt_settings() {
                    for ep in alt.endpoints() {
                        if ep.transfer_type() == TransferType::Bulk && ep.direction() == Direction::Out {
                            target = Some((iface.interface_number(), ep.address()));
                            break 'search;
                        }
                    }
                }
            }
        }
        let (interface_number, endpoint_address) = target.ok_or_else(|| {
            "Could not find a bulk OUT endpoint on this USB device — it may not be a printer, or its descriptors are non-standard.".to_string()
        })?;

        let interface = device.claim_interface(interface_number).wait().map_err(|e| {
            format!("Could not claim the USB interface — {e}. Another app (or a real driver still bound to this device) may be using it.")
        })?;

        let mut writer = interface
            .endpoint::<Bulk, Out>(endpoint_address)
            .map_err(|e| format!("Could not open endpoint 0x{endpoint_address:02x} — {e}"))?
            .writer(4096);

        writer.write_all(&data).map_err(|e| format!("USB write failed — {e}"))?;
        writer.flush().map_err(|e| format!("USB write failed to flush — {e}"))?;
        Ok(())
    }
}

#[cfg(target_os = "android")]
mod imp {
    use super::UsbDeviceInfo;
    pub fn list() -> Result<Vec<UsbDeviceInfo>, String> {
        Ok(Vec::new())
    }
    pub fn print(_v: u16, _p: u16, _d: Vec<u8>) -> Result<(), String> {
        Err("Direct USB printing is not available in the Android app yet. Use a network (Wi-Fi/LAN) printer.".to_string())
    }
}

#[tauri::command]
pub fn list_usb_printers() -> Result<Vec<UsbDeviceInfo>, String> {
    imp::list()
}

#[tauri::command]
pub fn print_raw_usb(vendor_id: u16, product_id: u16, data: Vec<u8>) -> Result<(), String> {
    imp::print(vendor_id, product_id, data)
}
