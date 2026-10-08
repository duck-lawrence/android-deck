use std::path::Path;
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Launch scrcpy in its own separate window without blocking the app.
pub fn launch(scrcpy_path: &str, serial: &str, args: &[String]) -> Result<(), String> {
    let path = Path::new(scrcpy_path);
    if !path.is_file() {
        return Err(format!("scrcpy.exe not found at: {scrcpy_path}"));
    }

    let mut cmd = Command::new(path);
    if let Some(dir) = path.parent() {
        cmd.current_dir(dir);
    }
    cmd.arg("-s").arg(serial);
    cmd.args(args);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    cmd.spawn()
        .map(|_| ())
        .map_err(|e| format!("Failed to launch scrcpy: {e}"))
}
