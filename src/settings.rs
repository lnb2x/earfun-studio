//! Persistent user configuration with validation and atomic file save.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SettingsData {
    pub auto_connect: bool,
    pub last_mac: String,
    pub eq: Vec<i32>,
    pub presets: BTreeMap<String, Vec<i32>>,
}

impl Default for SettingsData {
    fn default() -> Self {
        Self {
            auto_connect: false,
            last_mac: String::new(),
            eq: vec![0; 10],
            presets: BTreeMap::new(),
        }
    }
}

pub struct Settings {
    pub path: PathBuf,
    pub data: SettingsData,
}

fn default_config_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata)
                .join("earfun-studio")
                .join("settings.json");
        }
    }
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg)
                .join("earfun-studio")
                .join("settings.json");
        }
    }
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".config")
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

impl Settings {
    pub fn new(custom_path: Option<&Path>) -> Self {
        let path = custom_path
            .map(|p| p.to_path_buf())
            .unwrap_or_else(default_config_path);
        let mut data = SettingsData::default();

        if let Ok(text) = fs::read_to_string(&path) {
            if let Ok(serde_json::Value::Object(map)) = serde_json::from_str(&text) {
                if let Some(auto_val) = map.get("auto_connect") {
                    data.auto_connect = auto_val.as_bool() == Some(true);
                }
                if let Some(mac_val) = map.get("last_mac") {
                    if let Some(s) = mac_val.as_str() {
                        data.last_mac = s.to_string();
                    }
                }
                if let Some(eq_val) = map.get("eq") {
                    if let Some(valid_eq) = parse_valid_levels(eq_val) {
                        data.eq = valid_eq;
                    }
                }
                if let Some(presets_val) = map.get("presets") {
                    if let Some(presets_obj) = presets_val.as_object() {
                        for (k, v) in presets_obj {
                            if let Some(valid_levels) = parse_valid_levels(v) {
                                data.presets.insert(k.clone(), valid_levels);
                            }
                        }
                    }
                }
            }
        }

        Self { path, data }
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = self.path.with_extension("tmp");
        let text = serde_json::to_string_pretty(&self.data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(&tmp, text)?;
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}
