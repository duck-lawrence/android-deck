use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use crate::models::ConnectedDevice;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Run an adb command and return the combined stdout + stderr.
pub fn run(adb: &str, args: &[&str]) -> Result<String, String> {
    let mut cmd = Command::new(adb);
    cmd.args(args);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let output = cmd
        .output()
        .map_err(|e| format!("Could not run adb (\"{adb}\"): {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if !output.status.success() {
        let msg = if stderr.is_empty() { stdout } else { stderr };
        return Err(msg);
    }

    let combined = match (stdout.is_empty(), stderr.is_empty()) {
        (false, false) => format!("{stdout}\n{stderr}"),
        (false, true) => stdout,
        (true, false) => stderr,
        (true, true) => String::new(),
    };
    Ok(combined)
}

/// Parse the output of `adb devices -l`.
pub fn parse_devices(output: &str) -> Vec<ConnectedDevice> {
    let mut devices = Vec::new();

    for line in output.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() || line.starts_with('*') {
            continue;
        }

        let mut parts = line.split_whitespace();
        let Some(serial) = parts.next() else { continue };
        let Some(status) = parts.next() else { continue };

        let mut model = None;
        let mut product = None;
        let mut transport = None;

        for field in parts {
            if let Some((key, value)) = field.split_once(':') {
                match key {
                    "model" => model = Some(value.replace('_', " ")),
                    "product" => product = Some(value.to_string()),
                    "transport_id" => transport = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        devices.push(ConnectedDevice {
            serial: serial.to_string(),
            status: status.to_string(),
            model,
            product,
            transport,
        });
    }

    devices
}
