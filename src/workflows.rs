//! User-approved setting changes and their explicit dependencies.

use crate::protocol::DeviceState;
use std::collections::BTreeMap;

pub const CODECS: &[&str] = &["ldac", "aptx", "adaptive", "lossless"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub title: String,
    pub setter: u8,
    pub query: u8,
    pub value: u8,
    pub wanted: Vec<(String, bool)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub title: String,
    pub baseline: Vec<(String, bool)>,
    pub queries: Vec<u8>,
    pub steps: Vec<Step>,
    pub target: Vec<(String, bool)>,
}

impl Plan {
    pub fn confirmation(&self) -> String {
        let actions: Vec<&str> = self.steps.iter().map(|s| s.title.as_str()).collect();
        format!(
            "{}. Tai nghe có thể ngắt nhạc hoặc khởi động lại; ứng dụng sẽ tự kết nối lại và kiểm tra kết quả. Tiếp tục?",
            actions.join(" → ")
        )
    }
}

pub fn known(state: &DeviceState, keys: &[&str]) -> Result<Vec<(String, bool)>, String> {
    let mut out = Vec::with_capacity(keys.len());
    for &key in keys {
        match state.get_bool(key) {
            Some(v) => out.push((key.to_string(), v)),
            None => {
                return Err(
                    "Chưa đọc đủ trạng thái tai nghe. Bấm Đọc lại trạng thái rồi thử lại."
                        .to_string(),
                )
            }
        }
    }
    Ok(out)
}

pub fn codec_step(values: &BTreeMap<String, bool>) -> Step {
    let mut bits: u8 = 0;
    for (key, bit) in [("aptx", 1u8), ("adaptive", 2u8), ("ldac", 8u8), ("lossless", 16u8)] {
        if *values.get(key).unwrap_or(&false) {
            bits |= bit;
        }
    }
    let wanted = CODECS
        .iter()
        .map(|&k| (k.to_string(), *values.get(k).unwrap_or(&false)))
        .collect();
    Step {
        title: "Áp dụng codec đã chọn".to_string(),
        setter: 46,
        query: 47,
        value: bits,
        wanted,
    }
}

pub fn codecs(state: &DeviceState, wanted: &BTreeMap<String, bool>) -> Result<Plan, String> {
    let mut all_keys = vec!["dual"];
    all_keys.extend_from_slice(CODECS);
    let baseline = known(state, &all_keys)?;

    for &k in CODECS {
        if !wanted.contains_key(k) {
            return Err(
                "Chưa đọc đủ trạng thái tai nghe. Bấm Đọc lại trạng thái rồi thử lại.".to_string(),
            );
        }
    }

    let state_dual = state.dual.unwrap();
    let wanted_ldac = *wanted.get("ldac").unwrap();

    let mut steps = Vec::new();
    if wanted_ldac && state_dual {
        steps.push(Step {
            title: "Tắt kết nối hai thiết bị để dùng LDAC".to_string(),
            setter: 39,
            query: 38,
            value: 0,
            wanted: vec![("dual".to_string(), false)],
        });
    }
    if CODECS
        .iter()
        .any(|&k| state.get_bool(k) != wanted.get(k).copied())
    {
        steps.push(codec_step(wanted));
    }

    let mut target: Vec<(String, bool)> = CODECS
        .iter()
        .map(|&k| (k.to_string(), *wanted.get(k).unwrap()))
        .collect();
    target.push((
        "dual".to_string(),
        if wanted_ldac { false } else { state_dual },
    ));

    Ok(Plan {
        title: "Đổi codec Bluetooth".to_string(),
        baseline,
        queries: vec![38, 47],
        steps,
        target,
    })
}

pub fn dual(state: &DeviceState, on: bool) -> Result<Plan, String> {
    let mut all_keys = vec!["dual"];
    all_keys.extend_from_slice(CODECS);
    let baseline = known(state, &all_keys)?;

    let state_dual = state.dual.unwrap();
    let state_ldac = state.ldac.unwrap();

    let mut steps = Vec::new();
    if on && state_ldac {
        let mut wanted = BTreeMap::new();
        for &k in CODECS {
            wanted.insert(k.to_string(), state.get_bool(k).unwrap());
        }
        wanted.insert("ldac".to_string(), false);
        let step = codec_step(&wanted);
        steps.push(Step {
            title: "Tắt LDAC để dùng hai thiết bị".to_string(),
            setter: step.setter,
            query: step.query,
            value: step.value,
            wanted: step.wanted,
        });
    }
    if state_dual != on {
        steps.push(Step {
            title: if on {
                "Bật kết nối hai thiết bị".to_string()
            } else {
                "Tắt kết nối hai thiết bị".to_string()
            },
            setter: 39,
            query: 38,
            value: u8::from(on),
            wanted: vec![("dual".to_string(), on)],
        });
    }

    let mut target = vec![("dual".to_string(), on)];
    for &k in CODECS {
        let val = if on && k == "ldac" {
            false
        } else {
            state.get_bool(k).unwrap()
        };
        target.push((k.to_string(), val));
    }

    Ok(Plan {
        title: "Đổi kết nối đa điểm".to_string(),
        baseline,
        queries: vec![38, 47],
        steps,
        target,
    })
}

pub fn restart_setting(state: &DeviceState, key: &str, on: bool) -> Result<Plan, String> {
    let baseline = known(state, &[key])?;
    let step = match key {
        "le_audio" => Step {
            title: if on {
                "Bật LE Audio".to_string()
            } else {
                "Dùng Bluetooth thông thường / Google Fast Pair".to_string()
            },
            setter: 75,
            query: 76,
            value: u8::from(on),
            wanted: vec![(key.to_string(), on)],
        },
        "ear" => Step {
            title: if on {
                "Bật nhận biết đeo tai".to_string()
            } else {
                "Tắt nhận biết đeo tai".to_string()
            },
            setter: 73,
            query: 74,
            value: u8::from(!on),
            wanted: vec![(key.to_string(), on)],
        },
        _ => return Err("Thiết lập không có quy trình khởi động lại.".to_string()),
    };

    let current = state.get_bool(key).unwrap();
    let title = step.title.clone();
    let query = step.query;
    let target = step.wanted.clone();
    let steps = if current == on { vec![] } else { vec![step] };

    Ok(Plan {
        title,
        baseline,
        queries: vec![query],
        steps,
        target,
    })
}
