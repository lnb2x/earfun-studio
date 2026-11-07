//! TW600 EQ presets and import/export format, transcribed from EarFun Audio 20.1.61 APK.

use crate::protocol::FREQUENCIES;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const PRESETS_JSON: &str = include_str!("../assets/eq-presets.json");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CatalogSource {
    pub package: String,
    pub version: String,
    pub version_code: u32,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BuiltinPreset {
    pub key: String,
    pub name: String,
    pub group: String,
    pub levels: Vec<i32>,
    pub resource: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PresetCatalog {
    pub model: String,
    pub source: CatalogSource,
    pub presets: Vec<BuiltinPreset>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresetOption {
    pub key: String,
    pub label: String,
    pub levels: Option<Vec<i32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EqDocument {
    pub format: String,
    pub version: u32,
    pub model: String,
    pub name: String,
    pub frequencies: Vec<f64>,
    pub levels: Vec<i32>,
}

pub fn validate_levels(levels: &[i32]) -> Result<Vec<i32>, String> {
    if levels.len() != 10 || levels.iter().any(|&v| !(-10..=10).contains(&v)) {
        return Err(
            "Bản chỉnh cần 10 mức nguyên từ −10 đến +10 theo bảng gain TW600.".to_string(),
        );
    }
    Ok(levels.to_vec())
}

pub fn catalog() -> PresetCatalog {
    let cat: PresetCatalog =
        serde_json::from_str(PRESETS_JSON).expect("Embedded eq-presets.json must be valid JSON");
    for p in &cat.presets {
        validate_levels(&p.levels).expect("Builtin preset levels must be valid");
    }
    cat
}

pub fn builtins() -> Vec<BuiltinPreset> {
    catalog().presets
}

pub fn options(custom: &BTreeMap<String, Vec<i32>>) -> Vec<PresetOption> {
    let mut out = Vec::new();
    out.push(PresetOption {
        key: "draft".to_string(),
        label: "Tùy chỉnh (bản nháp)".to_string(),
        levels: None,
    });
    for p in builtins() {
        out.push(PresetOption {
            key: p.key,
            label: format!("{} · {}", p.group, p.name),
            levels: Some(p.levels),
        });
    }
    for (name, levels) in custom {
        out.push(PresetOption {
            key: format!("custom:{}", name),
            label: format!("Cá nhân · {}", name),
            levels: Some(levels.clone()),
        });
    }
    out
}

pub fn encode(name: &str, levels: &[i32]) -> Result<EqDocument, String> {
    let clean_name = name.trim();
    let final_name = if clean_name.is_empty() {
        "Bản chỉnh nhập".to_string()
    } else {
        clean_name.to_string()
    };
    Ok(EqDocument {
        format: "earfun-studio.eq".to_string(),
        version: 1,
        model: "TW600".to_string(),
        name: final_name,
        frequencies: FREQUENCIES.to_vec(),
        levels: validate_levels(levels)?,
    })
}

pub fn decode_value(val: &serde_json::Value) -> Result<(String, Vec<i32>), String> {
    let obj = val
        .as_object()
        .ok_or_else(|| "Không phải tệp EQ EarFun Studio phiên bản 1.".to_string())?;

    let format_ok = obj.get("format").and_then(|v| v.as_str()) == Some("earfun-studio.eq");
    let version_ok = obj
        .get("version")
        .map(|v| !v.is_boolean() && v.as_i64() == Some(1))
        .unwrap_or(false);
    if !format_ok || !version_ok {
        return Err("Không phải tệp EQ EarFun Studio phiên bản 1.".to_string());
    }

    let model_ok = obj.get("model").and_then(|v| v.as_str()) == Some("TW600");
    let freqs_ok = obj
        .get("frequencies")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.len() == FREQUENCIES.len()
                && arr
                    .iter()
                    .zip(FREQUENCIES.iter())
                    .all(|(a, &b)| !a.is_boolean() && a.as_f64().map(|x| (x - b).abs() < 1e-6).unwrap_or(false))
        })
        .unwrap_or(false);

    if !model_ok || !freqs_ok {
        return Err("Tệp EQ không đúng mẫu TW600 hoặc tần số 10 dải.".to_string());
    }

    let name = obj
        .get("name")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s.chars().count() <= 100)
        .ok_or_else(|| "Tên bản chỉnh phải có từ 1 đến 100 ký tự.".to_string())?;

    let raw_levels = obj
        .get("levels")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            "Bản chỉnh cần 10 mức nguyên từ −10 đến +10 theo bảng gain TW600.".to_string()
        })?;

    if raw_levels.len() != 10 {
        return Err(
            "Bản chỉnh cần 10 mức nguyên từ −10 đến +10 theo bảng gain TW600.".to_string(),
        );
    }

    let mut levels = Vec::with_capacity(10);
    for item in raw_levels {
        if item.is_boolean() || !item.is_i64() {
            return Err(
                "Bản chỉnh cần 10 mức nguyên từ −10 đến +10 theo bảng gain TW600.".to_string(),
            );
        }
        let n = item.as_i64().unwrap();
        if !(-10..=10).contains(&n) {
            return Err(
                "Bản chỉnh cần 10 mức nguyên từ −10 đến +10 theo bảng gain TW600.".to_string(),
            );
        }
        levels.push(n as i32);
    }

    Ok((name, levels))
}
