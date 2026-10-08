use serde::{Deserialize, Serialize};

pub fn default_adb_path() -> String {
    "adb".to_string()
}

pub fn default_scrcpy_path() -> String {
    r"C:\Apps\scrcpy-win64-v4.1\scrcpy.exe".to_string()
}

pub fn default_connect_port() -> u16 {
    5555
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub ip: String,
    #[serde(default = "default_connect_port")]
    pub connect_port: u16,
    #[serde(default)]
    pub pair_ip: String,
    #[serde(default)]
    pub pair_port: Option<u16>,
    #[serde(default)]
    pub scrcpy_args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default = "default_adb_path")]
    pub adb_path: String,
    #[serde(default = "default_scrcpy_path")]
    pub scrcpy_path: String,
    #[serde(default)]
    pub default_scrcpy_args: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            adb_path: default_adb_path(),
            scrcpy_path: default_scrcpy_path(),
            default_scrcpy_args: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectedDevice {
    pub serial: String,
    pub status: String,
    pub model: Option<String>,
    pub product: Option<String>,
    pub transport: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolStatus {
    pub adb_ok: bool,
    pub adb_path: String,
    pub adb_message: String,
    pub scrcpy_ok: bool,
    pub scrcpy_path: String,
    pub scrcpy_message: String,
}
