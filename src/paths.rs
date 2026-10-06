//! One place that decides where this app keeps its files (local.db, downloaded bundles, ...).
//!
//! Desktop keeps the exact location the Windows app already used (<data dir>/restropro), so an
//! existing install is not disturbed. Android has no such directory: files go in the app's own
//! private storage, which only Tauri can tell us, so this is filled in from `setup` before
//! anything touches the database or the bundle folder.
use std::path::PathBuf;
use std::sync::OnceLock;

static ROOT: OnceLock<PathBuf> = OnceLock::new();

pub fn init(handle: &tauri::AppHandle) -> Result<(), String> {
    #[cfg(not(target_os = "android"))]
    let base = {
        let _ = handle;
        dirs::data_dir().ok_or("could not find the user data directory")?.join("restropro")
    };
    #[cfg(target_os = "android")]
    let base = {
        use tauri::Manager;
        handle.path().app_data_dir().map_err(|e| format!("could not find the app data directory: {e}"))?
    };
    std::fs::create_dir_all(&base).map_err(|e| format!("could not create {base:?}: {e}"))?;
    let _ = ROOT.set(base);
    Ok(())
}

/// True once `init` has run. The app:// handler checks this so an early request gets a clean 503
/// instead of a panic.
pub fn ready() -> bool {
    ROOT.get().is_some()
}

pub fn root() -> PathBuf {
    ROOT.get().cloned().expect("paths::init must run in setup before the data directory is used")
}
