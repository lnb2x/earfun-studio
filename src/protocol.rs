//! TW600 wire format ported from EarFun Studio protocol specification.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

pub const SERVICE_STR: &str = "00001100-d102-11e1-9b23-00025b00a5a5";
pub const TX_STR: &str = "00001101-d102-11e1-9b23-00025b00a5a5";
pub const RX_STR: &str = "00001102-d102-11e1-9b23-00025b00a5a5";

pub const SERVICE_UUID: Uuid = Uuid::from_u128(0x00001100_d102_11e1_9b23_00025b00a5a5);
pub const TX_UUID: Uuid = Uuid::from_u128(0x00001101_d102_11e1_9b23_00025b00a5a5);
pub const RX_UUID: Uuid = Uuid::from_u128(0x00001102_d102_11e1_9b23_00025b00a5a5);

pub const QUERIES: &[u8] = &[
    9, 6, 7, 23, 21, 59, 61, 19, 38, 57, 49, 47, 74, 53, 11, 77, 4, 76, 84, 86,
];

pub const POLL_QUERIES: &[u8] = &[6, 7, 23, 21];

pub const GAINS: [f64; 21] = [
    -6.0, -5.0, -4.0, -3.5, -3.0, -2.5, -2.0, -1.5, -1.0, -0.5, 0.0, 0.5, 1.0, 1.5, 2.0, 2.5,
    3.0, 3.5, 4.0, 5.0, 6.0,
];

pub const FREQUENCIES: [f64; 10] = [
    31.5, 63.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];

pub const OPERATIONS: &[(&str, u8)] = &[
    ("Phát / tạm dừng", 1),
    ("Tăng âm lượng", 2),
    ("Giảm âm lượng", 3),
    ("Bài trước", 4),
    ("Bài tiếp", 5),
    ("Trợ lý giọng nói", 6),
    ("Chế độ game", 7),
    ("Đổi chế độ nghe", 8),
    ("ANC / bình thường", 9),
    ("Xuyên âm / bình thường", 10),
    ("ANC / xuyên âm", 11),
    ("Gọi lại", 13),
    ("Không gán", 16),
];

pub const GESTURES: &[(&str, u8)] = &[
    ("Chạm một lần", 1),
    ("Chạm hai lần", 2),
    ("Chạm ba lần", 3),
    ("Nhấn giữ", 17),
];

pub fn operation_code(action: &str) -> Option<u8> {
    OPERATIONS.iter().find(|(k, _)| *k == action).map(|(_, v)| *v)
}

pub fn operation_name(code: u8) -> String {
    OPERATIONS
        .iter()
        .find(|(_, v)| *v == code)
        .map(|(k, _)| (*k).to_string())
        .unwrap_or_else(|| format!("0x{:02X}", code))
}

pub fn gesture_code(gesture: &str) -> Option<u8> {
    GESTURES.iter().find(|(k, _)| *k == gesture).map(|(_, v)| *v)
}

pub fn packet(vendor: u16, command: u16, payload: &[u8]) -> Result<Vec<u8>, String> {
    if payload.len() > 508 {
        return Err("Gói Bluetooth quá dài".to_string());
    }
    let mut out = Vec::with_capacity(4 + payload.len());
    out.extend_from_slice(&vendor.to_be_bytes());
    out.extend_from_slice(&command.to_be_bytes());
    out.extend_from_slice(payload);
    Ok(out)
}

pub fn request(command: u8, payload: &[u8]) -> Vec<u8> {
    packet(10, 0x0300 | (command as u16), payload).expect("request payload within limit")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    pub vendor: u16,
    pub command: u16,
    pub status: u8,
    pub payload: Vec<u8>,
}

pub fn decode(raw: &[u8]) -> Option<Reply> {
    if raw.len() < 5 {
        return None;
    }
    let streamed = raw.len() >= 31 && &raw[..5] == b"\x00\x0a\x03\x2c\x00";
    if (raw[2] & 0x80) == 0 && !streamed {
        return None;
    }
    let vendor = u16::from_be_bytes([raw[0], raw[1]]);
    let cmd = u16::from_be_bytes([raw[2], raw[3]]);
    let status = raw[4];
    Some(Reply {
        vendor,
        command: cmd & 0x7fff,
        status,
        payload: raw[5..].to_vec(),
    })
}

pub fn gesture_packet(side: &str, gesture: &str, action: &str) -> Result<Vec<u8>, String> {
    if side != "L" && side != "R" {
        return Err("Tai nghe không hợp lệ".to_string());
    }
    let g_code = gesture_code(gesture).ok_or_else(|| "Thao tác không hợp lệ".to_string())?;
    let a_code = operation_code(action).ok_or_else(|| "Chức năng không hợp lệ".to_string())?;
    let payload = if side == "L" {
        [g_code, a_code, 0, 0]
    } else {
        [0, 0, g_code, a_code]
    };
    Ok(request(10, &payload))
}

pub fn equalizer(levels: &[i32]) -> Result<Vec<Vec<u8>>, String> {
    if levels.len() != 10 || levels.iter().any(|&x| !(-10..=10).contains(&x)) {
        return Err("EQ cần 10 mức nguyên trong khoảng -10…10".to_string());
    }
    let mut packets = Vec::with_capacity(10);
    for (i, &f) in FREQUENCIES.iter().enumerate() {
        let band_id = (161 + i) as u8;
        let pre_gain: i16 = -360;
        let freq_val: u16 = (f * 3.0) as u16;
        let gain_idx = (levels[i] + 10) as usize;
        let gain_val: i16 = (GAINS[gain_idx] * 60.0) as i16;
        let q_val: u16 = (0.7_f64 * 4096.0) as u16;

        let mut payload = Vec::with_capacity(9);
        payload.push(band_id);
        payload.extend_from_slice(&pre_gain.to_be_bytes());
        payload.extend_from_slice(&freq_val.to_be_bytes());
        payload.extend_from_slice(&gain_val.to_be_bytes());
        payload.extend_from_slice(&q_val.to_be_bytes());
        packets.push(packet(29, 0x0e01, &payload)?);
    }
    Ok(packets)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhoneEntry {
    pub num: u8,
    pub name: String,
    pub mac: String,
    pub connected: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceState {
    pub version: Option<String>,
    pub bat_l: Option<u8>,
    pub bat_r: Option<u8>,
    pub bat_box: Option<u8>,
    pub noise: Option<String>,
    pub sub: Option<String>,
    pub amb: Option<String>,
    pub prompt: Option<u8>,
    pub mic: Option<String>,
    pub codec: Option<String>,
    pub game: Option<bool>,
    pub dual: Option<bool>,
    pub ear: Option<bool>,
    pub le_audio: Option<bool>,
    pub google_fast_pair: Option<bool>,
    pub voice_language: Option<String>,
    pub aptx: Option<bool>,
    pub adaptive: Option<bool>,
    pub ldac: Option<bool>,
    pub lossless: Option<bool>,
    pub dis_l: Option<bool>,
    pub dis_r: Option<bool>,
    pub find_l: Option<bool>,
    pub find_r: Option<bool>,
    pub api: Option<u8>,
    pub hearing_limit: Option<u8>,
    pub balance: Option<u8>,
    pub keys: BTreeMap<String, String>,
}

impl DeviceState {
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        match key {
            "game" => self.game,
            "dual" => self.dual,
            "ear" => self.ear,
            "le_audio" => self.le_audio,
            "google_fast_pair" => self.google_fast_pair,
            "aptx" => self.aptx,
            "adaptive" => self.adaptive,
            "ldac" => self.ldac,
            "lossless" => self.lossless,
            "dis_l" => self.dis_l,
            "dis_r" => self.dis_r,
            "find_l" => self.find_l,
            "find_r" => self.find_r,
            _ => None,
        }
    }

    pub fn set_bool(&mut self, key: &str, val: bool) {
        match key {
            "game" => self.game = Some(val),
            "dual" => self.dual = Some(val),
            "ear" => self.ear = Some(val),
            "le_audio" => {
                self.le_audio = Some(val);
                self.google_fast_pair = Some(!val);
            }
            "google_fast_pair" => self.google_fast_pair = Some(val),
            "aptx" => self.aptx = Some(val),
            "adaptive" => self.adaptive = Some(val),
            "ldac" => self.ldac = Some(val),
            "lossless" => self.lossless = Some(val),
            "dis_l" => self.dis_l = Some(val),
            "dis_r" => self.dis_r = Some(val),
            "find_l" => self.find_l = Some(val),
            "find_r" => self.find_r = Some(val),
            _ => {}
        }
    }

    pub fn merge(&mut self, update: &StateUpdate) {
        if let Some(v) = &update.version {
            self.version = Some(v.clone());
        }
        if let Some(v) = update.bat_l {
            self.bat_l = Some(v);
        }
        if let Some(v) = update.bat_r {
            self.bat_r = Some(v);
        }
        if let Some(v) = update.bat_box {
            self.bat_box = Some(v);
        }
        if let Some(v) = &update.noise {
            self.noise = Some(v.clone());
        }
        if let Some(v) = &update.sub {
            self.sub = Some(v.clone());
        }
        if let Some(v) = &update.amb {
            self.amb = Some(v.clone());
        }
        if let Some(v) = update.prompt {
            self.prompt = Some(v);
        }
        if let Some(v) = &update.mic {
            self.mic = Some(v.clone());
        }
        if let Some(v) = &update.codec {
            self.codec = Some(v.clone());
        }
        if let Some(v) = update.game {
            self.game = Some(v);
        }
        if let Some(v) = update.dual {
            self.dual = Some(v);
        }
        if let Some(v) = update.ear {
            self.ear = Some(v);
        }
        if let Some(v) = update.le_audio {
            self.le_audio = Some(v);
        }
        if let Some(v) = update.google_fast_pair {
            self.google_fast_pair = Some(v);
        }
        if let Some(v) = &update.voice_language {
            self.voice_language = Some(v.clone());
        }
        if let Some(v) = update.aptx {
            self.aptx = Some(v);
        }
        if let Some(v) = update.adaptive {
            self.adaptive = Some(v);
        }
        if let Some(v) = update.ldac {
            self.ldac = Some(v);
        }
        if let Some(v) = update.lossless {
            self.lossless = Some(v);
        }
        if let Some(v) = update.dis_l {
            self.dis_l = Some(v);
        }
        if let Some(v) = update.dis_r {
            self.dis_r = Some(v);
        }
        if let Some(v) = update.find_l {
            self.find_l = Some(v);
        }
        if let Some(v) = update.find_r {
            self.find_r = Some(v);
        }
        if let Some(v) = update.api {
            self.api = Some(v);
        }
        if let Some(v) = update.hearing_limit {
            self.hearing_limit = Some(v);
        }
        if let Some(v) = update.balance {
            self.balance = Some(v);
        }
        if !update.keys.is_empty() {
            for (k, val) in &update.keys {
                self.keys.insert(k.clone(), val.clone());
            }
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StateUpdate {
    pub version: Option<String>,
    pub phone: Option<PhoneEntry>,
    pub bat_l: Option<u8>,
    pub bat_r: Option<u8>,
    pub bat_box: Option<u8>,
    pub noise: Option<String>,
    pub sub: Option<String>,
    pub amb: Option<String>,
    pub prompt: Option<u8>,
    pub mic: Option<String>,
    pub codec: Option<String>,
    pub game: Option<bool>,
    pub dual: Option<bool>,
    pub ear: Option<bool>,
    pub le_audio: Option<bool>,
    pub google_fast_pair: Option<bool>,
    pub voice_language: Option<String>,
    pub aptx: Option<bool>,
    pub adaptive: Option<bool>,
    pub ldac: Option<bool>,
    pub lossless: Option<bool>,
    pub dis_l: Option<bool>,
    pub dis_r: Option<bool>,
    pub find_l: Option<bool>,
    pub find_r: Option<bool>,
    pub api: Option<u8>,
    pub hearing_limit: Option<u8>,
    pub balance: Option<u8>,
    pub keys: BTreeMap<String, String>,
}

impl StateUpdate {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

pub fn state(reply: &Reply) -> StateUpdate {
    let mut out = StateUpdate::default();
    if reply.vendor != 10 || (reply.command >> 8) != 3 || reply.status != 0 {
        return out;
    }
    let cmd = (reply.command & 0xff) as u8;
    let data = &reply.payload;

    if cmd == 9 {
        let s = String::from_utf8_lossy(data)
            .trim_end_matches('\0')
            .to_string();
        out.version = Some(s);
        return out;
    }

    if cmd == 44 {
        if data.len() < 26 {
            return out;
        }
        let name = String::from_utf8_lossy(&data[1..20])
            .trim_end_matches('\0')
            .to_string();
        let mac = format!("{:02X}{:02X}{:02X}", data[22], data[23], data[24]);
        out.phone = Some(PhoneEntry {
            num: data[0],
            name,
            mac,
            connected: data[25] == 1,
        });
        return out;
    }

    if cmd == 11 {
        let mut keys = BTreeMap::new();
        for (g, (gesture, _)) in GESTURES.iter().enumerate() {
            for (side_idx, side_name) in ["L", "R"].iter().enumerate() {
                let pos = 1 + g * 4 + side_idx * 2;
                if pos < data.len() {
                    keys.insert(
                        format!("{}.{}", side_name, gesture),
                        operation_name(data[pos]),
                    );
                }
            }
        }
        out.keys = keys;
        return out;
    }

    if data.is_empty() {
        return out;
    }
    let v = data[0];

    match cmd {
        6 if v <= 100 => out.bat_l = Some(v),
        7 if v <= 100 => out.bat_r = Some(v),
        23 if v <= 100 => out.bat_box = Some(v),
        21 => {
            let vals = ["normal", "anc", "ambient", "adaptive", "wind"];
            if (v as usize) < vals.len() {
                out.noise = Some(vals[v as usize].to_string());
            }
        }
        59 => {
            let vals = ["EAR_ADAPTIVE", "WIND", "DEFAULT", "BALANCE", "ADAPTIVE"];
            if (v as usize) < vals.len() {
                out.sub = Some(vals[v as usize].to_string());
            }
        }
        61 => {
            let vals = ["DEFAULT", "TOTAL"];
            if (v as usize) < vals.len() {
                out.amb = Some(vals[v as usize].to_string());
            }
        }
        57 => {
            let vals: [u8; 5] = [100, 75, 50, 25, 0];
            if (v as usize) < vals.len() {
                out.prompt = Some(vals[v as usize]);
            }
        }
        49 => {
            let vals = ["AUTO", "L", "R"];
            if (v as usize) < vals.len() {
                out.mic = Some(vals[v as usize].to_string());
            }
        }
        77 => {
            let vals = [
                "SBC",
                "AAC",
                "aptX",
                "aptX HD",
                "aptX Adaptive",
                "aptX Lossless",
                "LDAC",
            ];
            if v >= 1 && ((v - 1) as usize) < vals.len() {
                out.codec = Some(vals[(v - 1) as usize].to_string());
            }
        }
        19 if v <= 1 => out.game = Some(v == 1),
        38 if v <= 1 => out.dual = Some(v == 1),
        74 if v <= 1 => out.ear = Some(v == 0),
        76 if v <= 1 => {
            out.le_audio = Some(v == 1);
            out.google_fast_pair = Some(v == 0);
        }
        113 if v < 4 => {
            let vals = ["EN", "ZH", "DE", "JA"];
            out.voice_language = Some(vals[v as usize].to_string());
        }
        47 => {
            out.aptx = Some((v & 1) != 0);
            out.adaptive = Some((v & 2) != 0);
            out.ldac = Some((v & 8) != 0);
            out.lossless = Some((v & 16) != 0);
        }
        53 if v <= 3 => {
            out.dis_l = Some(v == 1 || v == 2);
            out.dis_r = Some(v == 1 || v == 3);
        }
        51 => {
            out.find_l = Some((v & 1) != 0);
            out.find_r = Some((v & 2) != 0);
        }
        4 => out.api = Some(v),
        84 => out.hearing_limit = Some(v),
        86 => out.balance = Some(v),
        _ => {}
    }

    out
}

pub fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn hex_encode_spaced(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    let clean: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if clean.len() % 2 != 0 {
        return Err("Invalid hex length".to_string());
    }
    (0..clean.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&clean[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}
