//! Classic Bluetooth (SPP / RFCOMM) raw printing for Android.
//!
//! Usage (Android only):
//!     builder.plugin(tauri_plugin_bt_printer::init())
//!     app.bt_printer().print("00:11:22:33:44:55", &bytes)?;
use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

mod error;
// Some of these types are only used by the Android-only wrapper (mobile.rs).
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
mod models;

#[cfg(not(target_os = "android"))]
mod desktop;
#[cfg(target_os = "android")]
mod mobile;

#[cfg(not(target_os = "android"))]
use desktop::BtPrinter;
#[cfg(target_os = "android")]
use mobile::BtPrinter;

pub use error::{Error, Result};
pub use models::{PairedPrinter, PrinterStatus};

/// Access to the plugin from anything that can reach the app (AppHandle, App, Window...).
pub trait BtPrinterExt<R: Runtime> {
    fn bt_printer(&self) -> &BtPrinter<R>;
}

impl<R: Runtime, T: Manager<R>> BtPrinterExt<R> for T {
    fn bt_printer(&self) -> &BtPrinter<R> {
        self.state::<BtPrinter<R>>().inner()
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("bt-printer")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            let printer = mobile::init(app, api)?;
            #[cfg(not(target_os = "android"))]
            let printer = desktop::init(app, api)?;
            app.manage(printer);
            Ok(())
        })
        .build()
}
