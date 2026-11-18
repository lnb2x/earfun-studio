//! Persistent user configuration with validation, corruption backup, and atomic file save.

use crate::presets;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

const MAX_SETTINGS_BYTES: u64 = 1_048_576;
static TMP_COUNTER: AtomicU64 = AtomicU64::new(1);

/// User-selectable color theme mode.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    #[default]
    Light,
    Dark,
    System,
}

impl ThemeMode {
    pub fn label(&self) -> &'static str {
        match self {
            ThemeMode::Light => "Sáng",
            ThemeMode::Dark => "Tối",
            ThemeMode::System => "Hệ thống",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SettingsData {
    pub auto_connect: bool,
    pub connection_policy_version: u8,
    pub last_mac: String,
    pub eq: Vec<i32>,
    pub presets: BTreeMap<String, Vec<i32>>,
    #[serde(default)]
    pub theme: ThemeMode,
}

impl Default for SettingsData {
    fn default() -> Self {
        Self {
            auto_connect: true,
            connection_policy_version: 2,
            last_mac: String::new(),
            eq: vec![0; 10],
            presets: BTreeMap::new(),
            theme: ThemeMode::Light,
        }
    }
}

pub struct Settings {
    pub path: PathBuf,
    pub data: SettingsData,
    pub load_warning: Option<String>,
}

fn default_config_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            if !appdata.trim().is_empty() {
                return PathBuf::from(appdata)
                    .join("earfun-studio")
                    .join("settings.json");
            }
        }
    }
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.trim().is_empty() {
            return PathBuf::from(xdg)
                .join("earfun-studio")
                .join("settings.json");
        }
    }
    if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
        if !home.trim().is_empty() {
            return PathBuf::from(home)
                .join(".config")
                .join("earfun-studio")
                .join("settings.json");
        }
    }
    std::env::temp_dir()
        .join("earfun-studio")
        .join("settings.json")
}

fn parse_valid_levels(val: &serde_json::Value) -> Option<Vec<i32>> {
    let arr = val.as_array()?;
    if arr.len() != 10 {
        return None;
    }
    let mut out = Vec::with_capacity(10);
    for item in arr {
        if item.is_boolean() || !item.is_i64() {
            return None;
        }
        let n = item.as_i64()?;
        if !(-10..=10).contains(&n) {
            return None;
        }
        out.push(n as i32);
    }
    Some(out)
}

fn is_valid_saved_address(s: &str) -> bool {
    s.is_empty()
        || (s.len() <= 64
            && s.is_ascii()
            && !s.chars().any(|c| c.is_control() || c.is_whitespace()))
}

fn backup_corrupt_file(path: &Path, raw: &[u8]) {
    let fname = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("settings.json");
    let backup = path.with_file_name(format!("{}.corrupt.bak", fname));
    let _ = write_atomic_inner(&backup, raw, true);
}

fn write_atomic_inner(
    path: &Path,
    bytes: &[u8],
    #[cfg_attr(not(unix), allow(unused_variables))] private_dir: bool,
) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
            #[cfg(unix)]
            if private_dir {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
            }
        }
    }

    let base_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("earfun-file");
    let seq = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp = path.with_file_name(format!(".{}.{}.{}.tmp", base_name, std::process::id(), seq));

    let write_res = (|| -> Result<(), std::io::Error> {
        let mut opts = OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)]
        if private_dir {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let mut file = opts.open(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        Ok(())
    })();

    if let Err(e) = write_res {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }

    let mut last_err = None;
    for attempt in 0..4 {
        match fs::rename(&tmp, path) {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_err = Some(e);
                if attempt < 3 {
                    std::thread::sleep(Duration::from_millis(20));
                }
            }
        }
    }
    let _ = fs::remove_file(&tmp);
    Err(last_err.unwrap_or_else(|| std::io::Error::other("Atomic rename failed")))
}

/// Write `bytes` atomically to `path` via a unique sibling temp file, `sync_all`,
/// and bounded Windows sharing-violation retry without altering parent directory permissions.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), std::io::Error> {
    write_atomic_inner(path, bytes, false)
}

impl Settings {
    pub fn new(custom_path: Option<&Path>) -> Self {
        let path = custom_path
            .map(|p| p.to_path_buf())
            .unwrap_or_else(default_config_path);
        let mut data = SettingsData::default();
        let mut load_warning = None;

        match fs::metadata(&path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                load_warning = Some(format!("Không kiểm tra được tệp cấu hình: {}", e));
            }
            Ok(meta) if !meta.is_file() || meta.len() > MAX_SETTINGS_BYTES => {
                load_warning = Some("Tệp cấu hình không hợp lệ hoặc vượt quá 1 MiB.".to_string());
            }
            Ok(_) => match fs::File::open(&path).and_then(|f| {
                let mut buf = Vec::new();
                f.take(MAX_SETTINGS_BYTES + 1).read_to_end(&mut buf)?;
                Ok(buf)
            }) {
                Err(e) => {
                    load_warning = Some(format!("Không đọc được tệp cấu hình: {}", e));
                }
                Ok(raw) => match serde_json::from_slice::<serde_json::Value>(&raw) {
                    Ok(serde_json::Value::Object(map)) => {
                        let mut partial_corruption = false;
                        if let Some(auto_val) = map.get("auto_connect").filter(|_| {
                            map.get("connection_policy_version")
                                .and_then(|v| v.as_u64())
                                == Some(2)
                        }) {
                            if let Some(b) = auto_val.as_bool() {
                                data.auto_connect = b;
                            } else {
                                data.auto_connect = false;
                                partial_corruption = true;
                            }
                        }
                        if let Some(mac_val) = map.get("last_mac") {
                            if let Some(s) = mac_val.as_str() {
                                if is_valid_saved_address(s) {
                                    data.last_mac = s.to_string();
                                } else {
                                    partial_corruption = true;
                                }
                            }
                        }
                        if let Some(eq_val) = map.get("eq") {
                            if let Some(valid_eq) = parse_valid_levels(eq_val) {
                                data.eq = valid_eq;
                            } else {
                                partial_corruption = true;
                            }
                        }
                        if let Some(presets_val) = map.get("presets") {
                            if let Some(presets_obj) = presets_val.as_object() {
                                for (k, v) in presets_obj {
                                    match (presets::sanitize_preset_name(k), parse_valid_levels(v))
                                    {
                                        (Ok(clean_name), Some(valid_levels)) => {
                                            data.presets.insert(clean_name, valid_levels);
                                        }
                                        _ => partial_corruption = true,
                                    }
                                }
                            } else {
                                partial_corruption = true;
                            }
                        }
                        if let Some(theme_val) = map.get("theme").and_then(|v| v.as_str()) {
                            data.theme = match theme_val {
                                "dark" => ThemeMode::Dark,
                                "system" => ThemeMode::System,
                                _ => ThemeMode::Light,
                            };
                        }
                        if partial_corruption {
                            backup_corrupt_file(&path, &raw);
                            load_warning = Some(
                                "Một số mục cấu hình không hợp lệ đã được sao lưu sang .corrupt.bak."
                                    .to_string(),
                            );
                        }
                    }
                    _ => {
                        backup_corrupt_file(&path, &raw);
                        load_warning = Some(
                            "Cấu hình cũ bị lỗi cú pháp; đã sao lưu sang .corrupt.bak và khởi tạo mặc định."
                                .to_string(),
                        );
                    }
                },
            },
        }

        Self {
            path,
            data,
            load_warning,
        }
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let mut text = serde_json::to_string_pretty(&self.data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        text.push('\n');
        write_atomic_inner(&self.path, text.as_bytes(), true)
    }
}
