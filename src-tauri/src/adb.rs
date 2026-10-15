use std::fs::{self, File};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use wait_timeout::ChildExt;

use crate::models::ConnectedDevice;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Default timeout for quick/read-only adb commands.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(15);
/// Timeout for `adb connect` / `adb pair`, which can stall on unreachable hosts.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Run an adb command and return the combined stdout + stderr.
pub fn run(adb: &str, args: &[&str]) -> Result<String, String> {
    run_with_timeout(adb, args, DEFAULT_TIMEOUT)
}

/// Run an adb command with a hard timeout.
///
/// Output is captured through temporary files (not pipes) so a detached adb
/// server process inheriting the std handles can never hang the reader, and the
/// child is killed if it does not exit within `timeout`.
pub fn run_with_timeout(adb: &str, args: &[&str], timeout: Duration) -> Result<String, String> {
    let stdout_path = temp_path("out");
    let stderr_path = temp_path("err");

    let stdout_file = File::create(&stdout_path)
        .map_err(|e| format!("Could not create a temp file: {e}"))?;
    let stderr_file = File::create(&stderr_path)
        .map_err(|e| format!("Could not create a temp file: {e}"))?;

    let mut cmd = Command::new(adb);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file));
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            cleanup(&stdout_path, &stderr_path);
            return Err(format!("Could not run adb (\"{adb}\"): {e}"));
        }
    };

    let status = match child.wait_timeout(timeout) {
        Ok(Some(status)) => status,
        Ok(None) => {
            let _ = child.kill();
            let _ = child.wait();
            cleanup(&stdout_path, &stderr_path);
            return Err(format!(
                "adb {} timed out after {}s",
                args.join(" "),
                timeout.as_secs()
            ));
        }
        Err(e) => {
            let _ = child.kill();
            let _ = child.wait();
            cleanup(&stdout_path, &stderr_path);
            return Err(format!("Could not run adb (\"{adb}\"): {e}"));
        }
    };

    let stdout = fs::read_to_string(&stdout_path)
        .unwrap_or_default()
        .trim()
        .to_string();
    let stderr = fs::read_to_string(&stderr_path)
        .unwrap_or_default()
        .trim()
        .to_string();
    cleanup(&stdout_path, &stderr_path);

    if !status.success() {
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

fn temp_path(tag: &str) -> PathBuf {
    let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!(
        "android-deck-{}-{}-{}-{}.tmp",
        std::process::id(),
        tag,
        n,
        nanos
    ))
}

fn cleanup(a: &PathBuf, b: &PathBuf) {
    let _ = fs::remove_file(a);
    let _ = fs::remove_file(b);
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
