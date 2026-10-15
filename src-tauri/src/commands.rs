use std::path::Path;

use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

use crate::models::{ConnectedDevice, DetectionResult, Device, Settings, ToolStatus};
use crate::{adb, config, detect, scrcpy};

fn adb_path(app: &AppHandle) -> String {
    let path = config::load(app).settings.adb_path;
    if path.trim().is_empty() {
        "adb".to_string()
    } else {
        path
    }
}

/// Run a blocking operation on a worker thread so the UI/main thread never
/// freezes while adb is working.
async fn blocking<T, F>(f: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Settings {
    config::load(&app).settings
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
pub async fn list_connected(app: AppHandle) -> Result<Vec<ConnectedDevice>, String> {
    let adb = adb_path(&app);
    blocking(move || {
        let output = adb::run(&adb, &["devices", "-l"])?;
        Ok(adb::parse_devices(&output))
    })
    .await
}

#[tauri::command]
pub async fn adb_pair(app: AppHandle, host_port: String, code: String) -> Result<String, String> {
    let adb = adb_path(&app);
    blocking(move || {
        adb::run_with_timeout(&adb, &["pair", &host_port, &code], adb::CONNECT_TIMEOUT)
    })
    .await
}

#[tauri::command]
pub async fn adb_connect(app: AppHandle, host_port: String) -> Result<String, String> {
    let adb = adb_path(&app);
    blocking(move || adb::run_with_timeout(&adb, &["connect", &host_port], adb::CONNECT_TIMEOUT))
        .await
}

#[tauri::command]
pub async fn adb_disconnect(app: AppHandle, host_port: String) -> Result<String, String> {
    let adb = adb_path(&app);
    blocking(move || adb::run(&adb, &["disconnect", &host_port])).await
}

#[tauri::command]
pub async fn adb_kill_server(app: AppHandle) -> Result<String, String> {
    let adb = adb_path(&app);
    blocking(move || adb::run(&adb, &["kill-server"])).await
}

#[tauri::command]
pub fn launch_scrcpy(app: AppHandle, serial: String, args: Vec<String>) -> Result<(), String> {
    let scrcpy = config::load(&app).settings.scrcpy_path;
    scrcpy::launch(&scrcpy, &serial, &args)
}

fn build_tool_status(adb: String, scrcpy_path: String) -> ToolStatus {
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

#[tauri::command]
pub async fn check_tools(app: AppHandle) -> ToolStatus {
    let adb = adb_path(&app);
    let scrcpy_path = config::load(&app).settings.scrcpy_path;

    tauri::async_runtime::spawn_blocking(move || build_tool_status(adb, scrcpy_path))
        .await
        .unwrap_or_else(|e| ToolStatus {
            adb_ok: false,
            adb_path: String::new(),
            adb_message: format!("Tool check failed: {e}"),
            scrcpy_ok: false,
            scrcpy_path: String::new(),
            scrcpy_message: String::new(),
        })
}

#[tauri::command]
pub async fn detect_tools() -> DetectionResult {
    tauri::async_runtime::spawn_blocking(detect::detect)
        .await
        .unwrap_or_default()
}

#[tauri::command]
pub async fn pick_tool_file(app: AppHandle, kind: String) -> Result<Option<String>, String> {
    let title = if kind == "scrcpy" {
        "Select scrcpy.exe"
    } else {
        "Select adb.exe"
    };

    let picked = app
        .dialog()
        .file()
        .set_title(title)
        .add_filter("Executable", &["exe"])
        .blocking_pick_file();

    Ok(picked
        .and_then(|f| f.into_path().ok())
        .map(|p| p.to_string_lossy().to_string()))
}
