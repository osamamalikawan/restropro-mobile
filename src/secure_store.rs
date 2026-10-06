//! Device token / secret storage.
//!
//! Desktop: the OS credential store (Windows Credential Manager / macOS Keychain).
//! Android: the `keyring` crate has no Android backend, so values go in a file inside the app's
//! private storage (other apps cannot read it without root). That is weaker than the Android
//! Keystore: moving this to a Keystore-backed plugin is a recommended follow-up.
use std::sync::Mutex;

#[cfg(not(target_os = "android"))]
mod imp {
    use keyring::Entry;
    const SERVICE: &str = "restropro-desktop";
    fn entry(key: &str) -> Result<Entry, String> {
        Entry::new(SERVICE, key).map_err(|e| e.to_string())
    }
    pub fn set(key: &str, value: &str) -> Result<(), String> {
        entry(key)?.set_password(value).map_err(|e| e.to_string())
    }
    pub fn get(key: &str) -> Result<Option<String>, String> {
        match entry(key)?.get_password() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
    pub fn delete(key: &str) -> Result<(), String> {
        match entry(key)?.delete_credential() {
            Ok(_) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[cfg(target_os = "android")]
mod imp {
    use serde_json::{Map, Value};
    use std::path::PathBuf;

    fn file() -> PathBuf {
        crate::paths::root().join("secure.json")
    }
    fn load() -> Map<String, Value> {
        std::fs::read_to_string(file())
            .ok()
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default()
    }
    fn save(map: &Map<String, Value>) -> Result<(), String> {
        let path = file();
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec(map).map_err(|e| e.to_string())?)
            .map_err(|e| format!("could not save secure store: {e}"))?;
        std::fs::rename(&tmp, &path).map_err(|e| format!("could not save secure store: {e}"))
    }
    pub fn set(key: &str, value: &str) -> Result<(), String> {
        let mut m = load();
        m.insert(key.to_string(), Value::String(value.to_string()));
        save(&m)
    }
    pub fn get(key: &str) -> Result<Option<String>, String> {
        Ok(load().get(key).and_then(|v| v.as_str()).map(|s| s.to_string()))
    }
    pub fn delete(key: &str) -> Result<(), String> {
        let mut m = load();
        m.remove(key);
        save(&m)
    }
}

// Serialises read-modify-write of the Android file; harmless on desktop.
static LOCK: Mutex<()> = Mutex::new(());

#[tauri::command]
pub fn secure_set(key: String, value: String) -> Result<(), String> {
    let _g = LOCK.lock().map_err(|e| e.to_string())?;
    imp::set(&key, &value)
}

#[tauri::command]
pub fn secure_get(key: String) -> Result<Option<String>, String> {
    let _g = LOCK.lock().map_err(|e| e.to_string())?;
    imp::get(&key)
}

#[tauri::command]
pub fn secure_delete(key: String) -> Result<(), String> {
    let _g = LOCK.lock().map_err(|e| e.to_string())?;
    imp::delete(&key)
}
