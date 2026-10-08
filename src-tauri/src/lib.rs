mod adb;
mod commands;
mod config;
mod detect;
mod models;
mod scrcpy;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::list_devices,
            commands::save_device,
            commands::delete_device,
            commands::list_connected,
            commands::adb_pair,
            commands::adb_connect,
            commands::adb_disconnect,
            commands::adb_kill_server,
            commands::launch_scrcpy,
            commands::check_tools,
            commands::detect_tools,
            commands::pick_tool_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
