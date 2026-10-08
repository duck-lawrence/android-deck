use std::env;
use std::path::{Path, PathBuf};

use crate::models::{DetectionResult, DetectedTool};

const ADB: &str = "adb.exe";
const SCRCPY: &str = "scrcpy.exe";

/// Search the system for adb.exe and scrcpy.exe in well-known locations.
pub fn detect() -> DetectionResult {
    let mut scrcpy = find_scrcpy();
    sort_by_version(&mut scrcpy);

    let mut adb = find_adb();
    for tool in &scrcpy {
        if let Some(dir) = Path::new(&tool.path).parent() {
            push_tool(&mut adb, dir.join(ADB), "bundled with scrcpy");
        }
    }

    DetectionResult { adb, scrcpy }
}

/* ---------------------- adb ---------------------- */

fn find_adb() -> Vec<DetectedTool> {
    let mut list = Vec::new();

    for dir in path_dirs() {
        push_tool(&mut list, dir.join(ADB), "PATH");
    }

    for var in ["ANDROID_HOME", "ANDROID_SDK_ROOT", "ANDROID_SDK_HOME"] {
        if let Some(root) = env::var_os(var) {
            push_tool(
                &mut list,
                PathBuf::from(root).join("platform-tools").join(ADB),
                var,
            );
        }
    }

    if let Some(local) = local_app_data() {
        push_tool(
            &mut list,
            local.join("Android").join("Sdk").join("platform-tools").join(ADB),
            "Android SDK",
        );
        scan_winget(&mut list, "Google.PlatformTools", ADB, 3, "winget");
    }

    if let Some(pf) = program_files() {
        push_tool(
            &mut list,
            pf.join("Android").join("platform-tools").join(ADB),
            "Program Files",
        );
    }

    for dir in [
        r"C:\Android\platform-tools",
        r"C:\platform-tools",
        r"C:\adb",
        r"C:\ProgramData\chocolatey\bin",
    ] {
        push_tool(&mut list, PathBuf::from(dir).join(ADB), "common location");
    }

    if let Some(home) = home() {
        push_tool(&mut list, home.join("scoop").join("shims").join(ADB), "scoop");
        push_tool(
            &mut list,
            home.join("scoop")
                .join("apps")
                .join("adb")
                .join("current")
                .join(ADB),
            "scoop",
        );
    }

    for root in scan_roots() {
        let mut found = Vec::new();
        find_file_within(&root, ADB, 3, &mut found);
        for path in found {
            push_tool(&mut list, path, "scan");
        }
    }

    list
}

/* ---------------------- scrcpy ---------------------- */

fn find_scrcpy() -> Vec<DetectedTool> {
    let mut list = Vec::new();

    for dir in path_dirs() {
        push_tool(&mut list, dir.join(SCRCPY), "PATH");
    }

    if let Some(local) = local_app_data() {
        push_tool(&mut list, local.join("scrcpy").join(SCRCPY), "LOCALAPPDATA");
        scan_winget(&mut list, "Genymobile.scrcpy", SCRCPY, 4, "winget");
    }

    if let Some(home) = home() {
        push_tool(&mut list, home.join("scrcpy").join(SCRCPY), "user folder");
        push_tool(
            &mut list,
            home.join("scoop")
                .join("apps")
                .join("scrcpy")
                .join("current")
                .join(SCRCPY),
            "scoop",
        );
    }

    if let Some(pf) = program_files() {
        push_tool(&mut list, pf.join("scrcpy").join(SCRCPY), "Program Files");
    }

    for dir in [
        r"C:\scrcpy",
        r"C:\ProgramData\chocolatey\lib\scrcpy\tools",
        r"C:\ProgramData\chocolatey\bin",
    ] {
        push_tool(&mut list, PathBuf::from(dir).join(SCRCPY), "common location");
    }

    for root in scan_roots() {
        let mut found = Vec::new();
        find_file_within(&root, SCRCPY, 4, &mut found);
        for path in found {
            push_tool(&mut list, path, "scan");
        }
    }

    list
}

/* ---------------------- helpers ---------------------- */

fn push_tool(list: &mut Vec<DetectedTool>, path: PathBuf, source: &str) {
    if !path.is_file() {
        return;
    }
    let key = path.to_string_lossy().to_lowercase();
    if list.iter().any(|t| t.path.to_lowercase() == key) {
        return;
    }
    list.push(DetectedTool {
        path: path.to_string_lossy().to_string(),
        source: source.to_string(),
    });
}

fn path_dirs() -> Vec<PathBuf> {
    env::var_os("PATH")
        .map(|value| env::split_paths(&value).collect())
        .unwrap_or_default()
}

fn home() -> Option<PathBuf> {
    env::var_os("USERPROFILE").map(PathBuf::from)
}

fn local_app_data() -> Option<PathBuf> {
    env::var_os("LOCALAPPDATA").map(PathBuf::from)
}

fn program_files() -> Option<PathBuf> {
    env::var_os("ProgramFiles").map(PathBuf::from)
}

fn scan_roots() -> Vec<PathBuf> {
    vec![PathBuf::from(r"C:\Apps"), PathBuf::from(r"C:\Tools")]
}

fn scan_winget(
    list: &mut Vec<DetectedTool>,
    prefix: &str,
    file: &str,
    depth: usize,
    source: &str,
) {
    let Some(local) = local_app_data() else {
        return;
    };
    let root = local.join("Microsoft").join("WinGet").join("Packages");
    let Ok(entries) = std::fs::read_dir(&root) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(prefix) {
            let mut found = Vec::new();
            find_file_within(&entry.path(), file, depth, &mut found);
            for path in found {
                push_tool(list, path, source);
            }
        }
    }
}

fn find_file_within(dir: &Path, file: &str, depth: usize, out: &mut Vec<PathBuf>) {
    if depth == 0 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            find_file_within(&path, file, depth - 1, out);
        } else if path
            .file_name()
            .map(|n| n.eq_ignore_ascii_case(file))
            .unwrap_or(false)
        {
            out.push(path);
        }
    }
}

fn sort_by_version(list: &mut [DetectedTool]) {
    list.sort_by(|a, b| version_of(&b.path).cmp(&version_of(&a.path)));
}

fn version_of(path: &str) -> (u32, u32, u32) {
    let parent = path
        .rsplit(['\\', '/'])
        .nth(1)
        .unwrap_or(path);
    let bytes = parent.as_bytes();
    for i in 0..bytes.len() {
        if (bytes[i] == b'v' || bytes[i] == b'V')
            && i + 1 < bytes.len()
            && bytes[i + 1].is_ascii_digit()
        {
            let rest = &parent[i + 1..];
            let numeric: String = rest
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            let mut nums = [0u32, 0, 0];
            for (idx, part) in numeric.split('.').take(3).enumerate() {
                nums[idx] = part.parse().unwrap_or(0);
            }
            return (nums[0], nums[1], nums[2]);
        }
    }
    (0, 0, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_runs() {
        let result = detect();
        println!("adb candidates:");
        for tool in &result.adb {
            println!("  {}  ({})", tool.path, tool.source);
        }
        println!("scrcpy candidates:");
        for tool in &result.scrcpy {
            println!("  {}  ({})", tool.path, tool.source);
        }
    }

    #[test]
    fn parses_versions() {
        assert_eq!(version_of(r"C:\Apps\scrcpy-win64-v4.1\scrcpy.exe"), (4, 1, 0));
        assert_eq!(version_of(r"C:\Apps\scrcpy-win64-v3.10.2\scrcpy.exe"), (3, 10, 2));
        assert_eq!(version_of(r"C:\Apps\plain\scrcpy.exe"), (0, 0, 0));
    }
}

