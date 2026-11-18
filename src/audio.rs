//! Control only the selected Bluetooth audio sink, never the default speaker.
//! Uses validated URIs and bounded subprocess timeouts without shell-metacharacter expansion.

use std::process::Command;
#[cfg(not(target_os = "windows"))]
use std::time::Duration;

#[cfg(not(target_os = "windows"))]
const MAX_SUBPROCESS_BYTES: usize = 4 * 1024 * 1024;

/// Normalize a 6-byte Bluetooth MAC (`AA:BB:CC:DD:EE:FF`, `AA-BB-...`, `AA_BB_...`) to 12 lowercase hex chars.
pub fn normalize_mac(mac: &str) -> Option<String> {
    let mut out = String::with_capacity(12);
    for c in mac.chars() {
        if matches!(c, ':' | '-' | '_') {
            continue;
        }
        if !c.is_ascii_hexdigit() {
            return None;
        }
        out.push(c.to_ascii_lowercase());
    }
    if out.len() == 12 {
        Some(out)
    } else {
        None
    }
}

pub fn find_sink(objects: &serde_json::Value, mac: &str) -> Option<String> {
    let normalized = normalize_mac(mac)?;
    let arr = objects.as_array()?;
    for obj in arr {
        let Some(props) = obj.get("info").and_then(|i| i.get("props")) else {
            continue;
        };
        let media_class = props
            .get("media.class")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if media_class != "Audio/Sink" {
            continue;
        }
        let device_api = props
            .get("device.api")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let node_name = props
            .get("node.name")
            .and_then(|v| v.as_str())
            .unwrap_or("");
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
                .filter(|&c| c != ':' && c != '-' && c != '_')
                .map(|c| c.to_ascii_lowercase())
                .collect();
            clean.contains(&normalized)
        }) {
            if let Some(id) = obj.get("id") {
                if let Some(n) = id.as_i64().filter(|&n| n >= 0) {
                    return Some(n.to_string());
                }
                if let Some(s) = id
                    .as_str()
                    .filter(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
                {
                    return Some(s.to_string());
                }
            }
        }
    }
    None
}

#[cfg(not(target_os = "windows"))]
async fn run_cmd_async(program: &str, args: &[&str]) -> Result<String, String> {
    use tokio::io::AsyncReadExt;

    let mut child = tokio::process::Command::new(program)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| format!("Không tìm thấy công cụ âm thanh hệ thống ({}).", program))?;

    let stdout_pipe = child
        .stdout
        .take()
        .ok_or_else(|| format!("Không mở được đầu ra của {}.", program))?;

    let read_fut = async {
        let mut buf = Vec::new();
        stdout_pipe
            .take((MAX_SUBPROCESS_BYTES + 1) as u64)
            .read_to_end(&mut buf)
            .await
            .map_err(|_| format!("Không đọc được phản hồi từ {}.", program))?;
        let status = child
            .wait()
            .await
            .map_err(|_| format!("Không hoàn tất được lệnh {}.", program))?;
        Ok::<(std::process::ExitStatus, Vec<u8>), String>((status, buf))
    };

    let (status, buf) = tokio::time::timeout(Duration::from_secs(3), read_fut)
        .await
        .map_err(|_| format!("Công cụ âm thanh {} phản hồi quá thời gian chờ.", program))??;

    if buf.len() > MAX_SUBPROCESS_BYTES {
        return Err(format!(
            "Dữ liệu trả về từ {} vượt giới hạn an toàn.",
            program
        ));
    }
    if !status.success() {
        return Err(format!(
            "Không thao tác được âm lượng qua {} (mã thoát {}).",
            program,
            status.code().unwrap_or(-1)
        ));
    }
    Ok(String::from_utf8_lossy(&buf).to_string())
}

/// Asynchronous non-blocking PipeWire/WirePlumber volume control on Ubuntu/Linux.
pub async fn volume_async(mac: &str, percent: Option<u8>) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        windows_volume(mac, percent)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let norm_mac = normalize_mac(mac)
            .ok_or_else(|| "Chưa có địa chỉ tai nghe hợp lệ để đọc âm lượng đầu ra.".to_string())?;
        let dump_str = run_cmd_async("pw-dump", &[]).await?;
        let objects: serde_json::Value = serde_json::from_str(&dump_str)
            .map_err(|_| "Không phân tích được danh sách đầu ra PipeWire.".to_string())?;
        let sink = find_sink(&objects, mac).ok_or_else(|| {
            "Chưa có đầu ra âm thanh EarFun. Kết nối tai nghe trong Cài đặt Bluetooth.".to_string()
        })?;
        if let Some(pct) = percent {
            let identity = run_cmd_async("wpctl", &["inspect", &sink])
                .await?
                .chars()
                .filter(|&c| c != ':' && c != '-' && c != '_')
                .map(|c| c.to_ascii_lowercase())
                .collect::<String>();
            if !identity.contains(&norm_mac) {
                return Err("Đầu ra âm thanh đã thay đổi. Hãy đọc lại âm lượng.".to_string());
            }
            let clamped = pct.min(100);
            let vol_arg = format!("{}%", clamped);
            run_cmd_async("wpctl", &["set-volume", &sink, &vol_arg]).await?;
        }
        Ok(run_cmd_async("wpctl", &["get-volume", &sink])
            .await?
            .trim()
            .to_string())
    }
}

pub fn volume(mac: &str, percent: Option<u8>) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        windows_volume(mac, percent)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;
        rt.block_on(volume_async(mac, percent))
    }
}

#[cfg(target_os = "windows")]
fn windows_volume(_mac: &str, _percent: Option<u8>) -> Result<String, String> {
    Err("Điều chỉnh âm lượng đầu ra EarFun trong Cài đặt âm thanh Windows hoặc bằng nút cảm ứng tai nghe.".into())
}

/// Validate external URI targets against an explicit allowlist and reject shell metacharacters.
pub fn validate_external_target(target: &str) -> Result<&str, String> {
    let trimmed = target.trim();
    if trimmed.is_empty() || trimmed.len() > 512 {
        return Err("Đường dẫn mở ngoài không hợp lệ.".to_string());
    }
    if trimmed.chars().any(|c| {
        c.is_control()
            || c.is_whitespace()
            || matches!(
                c,
                '"' | '\'' | '`' | '$' | '%' | '&' | '|' | ';' | '<' | '>' | '(' | ')' | '\\'
            )
    }) {
        return Err("Đường dẫn chứa ký tự không an toàn.".to_string());
    }
    if matches!(
        trimmed,
        "ms-settings:bluetooth" | "ms-settings:sound" | "ms-settings:apps-volume"
    ) {
        return Ok(trimmed);
    }
    if let Some(rest) = trimmed.strip_prefix("https://") {
        let host = rest.split('/').next().unwrap_or("");
        if !host.is_empty()
            && host
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
        {
            return Ok(trimmed);
        }
    }
    Err("Chỉ cho phép mở cài đặt hệ thống được phê duyệt hoặc liên kết HTTPS an toàn.".to_string())
}

fn spawn_and_reap(mut cmd: Command, err_context: &str) -> Result<(), String> {
    let mut child = cmd.spawn().map_err(|e| format!("{}: {}", err_context, e))?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

pub fn open_bluetooth_settings() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        open_url_or_path("ms-settings:bluetooth")
    }
    #[cfg(not(target_os = "windows"))]
    {
        let mut cmd = Command::new("gnome-control-center");
        cmd.arg("bluetooth");
        spawn_and_reap(cmd, "Không mở được Cài đặt Bluetooth hệ thống")
    }
}

pub fn open_sound_settings() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        open_url_or_path("ms-settings:sound")
    }
    #[cfg(not(target_os = "windows"))]
    {
        let mut cmd = Command::new("gnome-control-center");
        cmd.arg("sound");
        spawn_and_reap(cmd, "Không mở được Cài đặt âm thanh hệ thống")
    }
}

pub fn open_url_or_path(target: &str) -> Result<(), String> {
    let safe_target = validate_external_target(target)?;
    #[cfg(target_os = "windows")]
    {
        let mut cmd = Command::new("explorer.exe");
        cmd.arg(safe_target);
        spawn_and_reap(cmd, "Không mở được liên kết")
    }
    #[cfg(not(target_os = "windows"))]
    {
        let mut cmd = Command::new("xdg-open");
        cmd.arg(safe_target);
        spawn_and_reap(cmd, "Không mở được liên kết")
    }
}
