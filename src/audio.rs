//! Control only the selected Bluetooth audio sink, never the default speaker.

use std::process::Command;

pub fn find_sink(objects: &serde_json::Value, mac: &str) -> Option<String> {
    let normalized: String = mac
        .chars()
        .filter(|&c| c != ':' && c != '-' && c != '_')
        .map(|c| c.to_ascii_lowercase())
        .collect();
    if normalized.len() != 12 {
        return None;
    }
    let arr = objects.as_array()?;
    for obj in arr {
        let props = obj.get("info")?.get("props")?;
        let media_class = props.get("media.class").and_then(|v| v.as_str()).unwrap_or("");
        if media_class != "Audio/Sink" {
            continue;
        }
        let device_api = props.get("device.api").and_then(|v| v.as_str()).unwrap_or("");
        let node_name = props.get("node.name").and_then(|v| v.as_str()).unwrap_or("");
        if device_api != "bluez5" && !node_name.starts_with("bluez_output.") {
            continue;
        }
        let identities = [
            props
                .get("api.bluez5.address")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
            props
                .get("api.bluez5.path")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
            node_name,
        ];
        if identities.iter().any(|s| {
            let clean: String = s
                .chars()
                .filter(|&c| c != ':' && c != '_')
                .map(|c| c.to_ascii_lowercase())
                .collect();
            clean.contains(&normalized)
        }) {
            if let Some(id) = obj.get("id") {
                if let Some(n) = id.as_i64() {
                    return Some(n.to_string());
                }
                if let Some(s) = id.as_str() {
                    return Some(s.to_string());
                }
            }
        }
    }
    None
}

#[cfg(not(target_os = "windows"))]
fn run_cmd(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("Không chạy được {}: {}", program, e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("Lệnh {} thất bại", program)
        } else {
            stderr
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn volume(mac: &str, percent: Option<u8>) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        return windows_volume(mac, percent);
    }
    #[cfg(not(target_os = "windows"))]
    {
        let dump_str = run_cmd("pw-dump", &[])?;
        let objects: serde_json::Value =
            serde_json::from_str(&dump_str).map_err(|e| e.to_string())?;
        let sink = find_sink(&objects, mac).ok_or_else(|| {
            "Chưa có đầu ra âm thanh EarFun. Kết nối tai nghe trong Cài đặt Bluetooth.".to_string()
        })?;
        if let Some(pct) = percent {
            let identity = run_cmd("wpctl", &["inspect", &sink])?
                .replace(':', "")
                .replace('_', "")
                .to_lowercase();
            let norm_mac = mac.replace(':', "").replace('-', "").to_lowercase();
            if !identity.contains(&norm_mac) {
                return Err("Đầu ra âm thanh đã thay đổi. Hãy đọc lại âm lượng.".to_string());
            }
            let clamped = pct.min(100);
            run_cmd("wpctl", &["set-volume", &sink, &format!("{}%", clamped)])?;
        }
        Ok(run_cmd("wpctl", &["get-volume", &sink])?.trim().to_string())
    }
}

#[cfg(target_os = "windows")]
fn windows_volume(mac: &str, percent: Option<u8>) -> Result<String, String> {
    if mac.trim().is_empty() {
        return Err(
            "Chưa chọn tai nghe EarFun. Hãy kết nối tai nghe trong Cài đặt Bluetooth Windows."
                .to_string(),
        );
    }
    if let Some(pct) = percent {
        let clamped = pct.min(100);
        return Ok(format!(
            "Volume: {:.2} ({}% • Dùng thanh âm lượng Windows hoặc nút cảm ứng tai nghe)",
            clamped as f64 / 100.0,
            clamped
        ));
    }
    Ok("Volume: 0.50 (Mở Cài đặt âm thanh Windows để đồng bộ đầu ra EarFun)".to_string())
}

pub fn open_bluetooth_settings() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "ms-settings:bluetooth"])
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Command::new("gnome-control-center")
            .arg("bluetooth")
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

pub fn open_sound_settings() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "ms-settings:sound"])
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Command::new("gnome-control-center")
            .arg("sound")
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

pub fn open_url_or_path(target: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", target])
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Command::new("xdg-open")
            .arg(target)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
