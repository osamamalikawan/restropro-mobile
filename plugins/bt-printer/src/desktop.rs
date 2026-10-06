//! Non-Android builds: Bluetooth printing goes through the COM port there (serial_print.rs).
//! This exists so the crate compiles everywhere; every call just says so.
use serde::de::DeserializeOwned;
use tauri::{plugin::PluginApi, AppHandle, Runtime};

use crate::models::*;
use crate::{Error, Result};

pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> Result<BtPrinter<R>> {
    Ok(BtPrinter(app.clone()))
}

pub struct BtPrinter<R: Runtime>(#[allow(dead_code)] AppHandle<R>);

fn unsupported<T>() -> Result<T> {
    Err(Error("The Bluetooth printer plugin only runs on Android.".to_string()))
}

impl<R: Runtime> BtPrinter<R> {
    pub fn list_paired(&self) -> Result<Vec<PairedPrinter>> {
        unsupported()
    }
    pub fn status(&self, _address: &str) -> Result<PrinterStatus> {
        unsupported()
    }
    pub fn print(&self, _address: &str, _data: &[u8]) -> Result<()> {
        unsupported()
    }
}
