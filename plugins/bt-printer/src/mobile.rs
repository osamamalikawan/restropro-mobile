//! Android side: thin typed wrapper over the Kotlin plugin (BtPrinterPlugin.kt).
use base64::Engine;
use serde::de::DeserializeOwned;
use tauri::{
    plugin::{PluginApi, PluginHandle},
    AppHandle, Runtime,
};

use crate::models::*;
use crate::{Error, Result};

const PLUGIN_IDENTIFIER: &str = "app.restropro.btprinter";

pub fn init<R: Runtime, C: DeserializeOwned>(
    _app: &AppHandle<R>,
    api: PluginApi<R, C>,
) -> Result<BtPrinter<R>> {
    let handle = api
        .register_android_plugin(PLUGIN_IDENTIFIER, "BtPrinterPlugin")
        .map_err(|e| Error(e.to_string()))?;
    Ok(BtPrinter(handle))
}

pub struct BtPrinter<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> BtPrinter<R> {
    /// Android 12+ needs the "Nearby devices" runtime permission before any Bluetooth call.
    /// Older versions resolve immediately. May show a system dialog the first time.
    fn ensure_permission(&self) -> Result<()> {
        let r: PermissionResponse = self
            .0
            .run_mobile_plugin("ensurePermission", serde_json::json!({}))
            .map_err(|e| Error(e.to_string()))?;
        if r.granted {
            Ok(())
        } else {
            Err(Error(
                "Bluetooth permission was denied. In Android Settings → Apps → Restro Pro POS → Permissions, allow \"Nearby devices\"."
                    .to_string(),
            ))
        }
    }

    pub fn list_paired(&self) -> Result<Vec<PairedPrinter>> {
        self.ensure_permission()?;
        let r: ListResponse = self
            .0
            .run_mobile_plugin("listPaired", serde_json::json!({}))
            .map_err(|e| Error(e.to_string()))?;
        Ok(r.devices)
    }

    /// Does NOT connect: only confirms Bluetooth is on and the printer is still paired.
    pub fn status(&self, address: &str) -> Result<PrinterStatus> {
        self.ensure_permission()?;
        self.0
            .run_mobile_plugin("status", AddressArgs { address })
            .map_err(|e| Error(e.to_string()))
    }

    pub fn print(&self, address: &str, data: &[u8]) -> Result<()> {
        self.ensure_permission()?;
        let data = base64::engine::general_purpose::STANDARD.encode(data);
        let _: serde_json::Value = self
            .0
            .run_mobile_plugin("print", PrintArgs { address, data })
            .map_err(|e| Error(e.to_string()))?;
        Ok(())
    }
}
