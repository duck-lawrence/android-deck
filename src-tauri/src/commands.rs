use std::path::Path;

use tauri::AppHandle;

use crate::models::{default_scrcpy_path, ConnectedDevice, Device, Settings, ToolStatus};
use crate::{adb, config, scrcpy};

fn adb_path(app: &AppHandle) -> String {
    let path = config::load(app).settings.adb_path;
    if path.trim().is_empty() {
        "adb".to_string()
    } else {
        path
    }
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Settings {
    let mut settings = config::load(&app).settings;
    if settings.adb_path.trim().is_empty() {
        settings.adb_path = "adb".to_string();
    }
    if settings.scrcpy_path.trim().is_empty() {
        settings.scrcpy_path = default_scrcpy_path();
    }
    settings
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: Settings) -> Result<(), String> {
    let mut cfg = config::load(&app);
    cfg.settings = settings;
    config::save(&app, &cfg)
}

#[tauri::command]
pub fn list_devices(app: AppHandle) -> Vec<Device> {
    config::load(&app).devices
}

#[tauri::command]
pub fn save_device(app: AppHandle, mut device: Device) -> Result<Device, String> {
    let mut cfg = config::load(&app);

    if device.id.trim().is_empty() {
        device.id = uuid::Uuid::new_v4().to_string();
    }
    if device.connect_port == 0 {
        device.connect_port = 5555;
    }

    match cfg.devices.iter().position(|d| d.id == device.id) {
        Some(pos) => cfg.devices[pos] = device.clone(),
        None => cfg.devices.push(device.clone()),
    }

    config::save(&app, &cfg)?;
    Ok(device)
}

#[tauri::command]
pub fn delete_device(app: AppHandle, id: String) -> Result<(), String> {
    let mut cfg = config::load(&app);
    cfg.devices.retain(|d| d.id != id);
    config::save(&app, &cfg)
}

#[tauri::command]
pub fn list_connected(app: AppHandle) -> Result<Vec<ConnectedDevice>, String> {
    let adb = adb_path(&app);
    let output = adb::run(&adb, &["devices", "-l"])?;
    Ok(adb::parse_devices(&output))
}

#[tauri::command]
pub fn adb_pair(app: AppHandle, host_port: String, code: String) -> Result<String, String> {
    let adb = adb_path(&app);
    adb::run(&adb, &["pair", &host_port, &code])
}

#[tauri::command]
pub fn adb_connect(app: AppHandle, host_port: String) -> Result<String, String> {
    let adb = adb_path(&app);
    adb::run(&adb, &["connect", &host_port])
}

#[tauri::command]
pub fn adb_disconnect(app: AppHandle, host_port: String) -> Result<String, String> {
    let adb = adb_path(&app);
    adb::run(&adb, &["disconnect", &host_port])
}

#[tauri::command]
pub fn adb_kill_server(app: AppHandle) -> Result<String, String> {
    let adb = adb_path(&app);
    adb::run(&adb, &["kill-server"])
}

#[tauri::command]
pub fn launch_scrcpy(app: AppHandle, serial: String, args: Vec<String>) -> Result<(), String> {
    let scrcpy = config::load(&app).settings.scrcpy_path;
    scrcpy::launch(&scrcpy, &serial, &args)
}

#[tauri::command]
pub fn check_tools(app: AppHandle) -> ToolStatus {
    let settings = get_settings(app.clone());
    let adb = settings.adb_path.clone();

    let (adb_ok, adb_message) = match adb::run(&adb, &["version"]) {
        Ok(output) => {
            let version = output
                .lines()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("adb is ready")
                .to_string();
            (true, version)
        }
        Err(err) => (false, err),
    };

    let scrcpy_path = settings.scrcpy_path.clone();
    let scrcpy_ok = Path::new(&scrcpy_path).is_file();
    let scrcpy_message = if scrcpy_ok {
        format!("Found scrcpy: {scrcpy_path}")
    } else {
        format!("scrcpy not found at: {scrcpy_path}")
    };

    ToolStatus {
        adb_ok,
        adb_path: adb,
        adb_message,
        scrcpy_ok,
        scrcpy_path,
        scrcpy_message,
    }
}
