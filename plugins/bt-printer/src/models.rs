use serde::{Deserialize, Serialize};

/// A Bluetooth device already paired (bonded) in Android settings.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairedPrinter {
    /// MAC address, e.g. "00:11:22:33:44:55". This is what the web app stores as the printer "port".
    pub address: String,
    pub name: String,
    /// Heuristic only (device class / name). Used to sort likely printers to the top.
    #[serde(default)]
    pub likely_printer: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PrinterStatus {
    pub present: bool,
    pub detail: String,
}

#[derive(Deserialize)]
pub(crate) struct ListResponse {
    pub devices: Vec<PairedPrinter>,
}

#[derive(Deserialize)]
pub(crate) struct PermissionResponse {
    pub granted: bool,
}

#[derive(Serialize)]
pub(crate) struct AddressArgs<'a> {
    pub address: &'a str,
}

#[derive(Serialize)]
pub(crate) struct PrintArgs<'a> {
    pub address: &'a str,
    /// Base64: a JSON number array for a few KB of ESC/POS is slow to cross the JNI boundary.
    pub data: String,
}
