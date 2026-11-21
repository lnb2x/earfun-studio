//! Verified TW600 device operations, multi-field readback retries, and restart workflows.

use super::{lock, BackendCore, BackendError, BackendEvent, ConnectionPhase};
use crate::protocol::{self as p, DeviceState, StateUpdate};
use crate::workflows::{Plan, Step};
use std::collections::BTreeMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

impl BackendCore {
    pub async fn query_internal(&self, full: bool) -> Result<(), BackendError> {
        let hook_opt = lock(&self.query_hook).clone();
        if let Some(hook) = hook_opt {
            return hook(full).await;
        }
        let commands = if full { p::QUERIES } else { p::POLL_QUERIES };
        for &cmd in commands {
            match self.request(&p::request(cmd, &[])).await {
                Ok(reply) => {
                    self.apply(&reply);
                }
                Err(BackendError::CommandRejected(msg)) => {
                    self.record("?", &msg);
                }
                Err(e) => return Err(e),
            }
        }
        if full {
            self.phones_internal().await?;
        }
        self.emit_event(BackendEvent::Notice(
            "Đã đọc trạng thái trực tiếp từ tai nghe.".to_string(),
        ));
        Ok(())
    }

    pub async fn query(&self) -> Result<(), BackendError> {
        let _guard = self.op_lock.lock().await;
        if self.is_connected() {
            self.query_internal(true).await?;
        }
        Ok(())
    }

    async fn verify_until<F>(
        &self,
        query: u8,
        max_wait: Duration,
        err_msg: &str,
        mut predicate: F,
    ) -> Result<(), BackendError>
    where
        F: FnMut(&DeviceState, &StateUpdate) -> bool,
    {
        let wait_dur = (*lock(&self.step_sleep)).min(max_wait);
        for _ in 0..8 {
            tokio::time::sleep(wait_dur).await;
            let reply = self.request(&p::request(query, &[])).await?;
            let upd = self.apply(&reply);
            let matched = {
                let st = lock(&self.state);
                predicate(&st, &upd)
            };
            if matched {
                return Ok(());
            }
        }
        Err(BackendError::Runtime(err_msg.to_string()))
    }

    pub async fn set_bool_value(
        &self,
        setter: u8,
        query: u8,
        value: u8,
        key: &str,
        wanted: bool,
    ) -> Result<(), BackendError> {
        let _guard = self.op_lock.lock().await;
        if !self.is_connected() {
            return Err(BackendError::Connection(
                "Kết nối tai nghe trước khi điều chỉnh.".to_string(),
            ));
        }
        self.send(&p::request(setter, &[value])).await?;
        self.verify_until(
            query,
            Duration::from_millis(400),
            "Tai nghe chưa xác nhận giá trị mới. Giao diện hiển thị trạng thái đọc lại.",
            |st, _| st.get_bool(key) == Some(wanted),
        )
        .await?;
        self.emit_event(BackendEvent::Notice(
            "Tai nghe đã xác nhận thay đổi.".to_string(),
        ));
        Ok(())
    }

    pub async fn set_string_value(
        &self,
        setter: u8,
        query: u8,
        value: u8,
        key: &str,
        wanted: &str,
    ) -> Result<(), BackendError> {
        let _guard = self.op_lock.lock().await;
        if !self.is_connected() {
            return Err(BackendError::Connection(
                "Kết nối tai nghe trước khi điều chỉnh.".to_string(),
            ));
        }
        self.send(&p::request(setter, &[value])).await?;
        self.verify_until(
            query,
            Duration::from_millis(400),
            "Tai nghe chưa xác nhận giá trị mới. Giao diện hiển thị trạng thái đọc lại.",
            |_, upd| {
                let actual = match key {
                    "noise" => upd.noise.as_deref(),
                    "sub" => upd.sub.as_deref(),
                    "amb" => upd.amb.as_deref(),
                    "mic" => upd.mic.as_deref(),
                    _ => None,
                };
                actual == Some(wanted)
            },
        )
        .await?;
        self.emit_event(BackendEvent::Notice(
            "Tai nghe đã xác nhận thay đổi.".to_string(),
        ));
        Ok(())
    }

    pub async fn set_u8_value(
        &self,
        setter: u8,
        query: u8,
        value: u8,
        key: &str,
        wanted: u8,
    ) -> Result<(), BackendError> {
        let _guard = self.op_lock.lock().await;
        if !self.is_connected() {
            return Err(BackendError::Connection(
                "Kết nối tai nghe trước khi điều chỉnh.".to_string(),
            ));
        }
        self.send(&p::request(setter, &[value])).await?;
        self.verify_until(
            query,
            Duration::from_millis(400),
            "Tai nghe chưa xác nhận giá trị mới. Giao diện hiển thị trạng thái đọc lại.",
            |_, upd| {
                let actual = match key {
                    "prompt" => upd.prompt,
                    "hearing_limit" => upd.hearing_limit,
                    "balance" => upd.balance,
                    _ => None,
                };
                actual == Some(wanted)
            },
        )
        .await?;
        self.emit_event(BackendEvent::Notice(
            "Tai nghe đã xác nhận thay đổi.".to_string(),
        ));
        Ok(())
    }

    pub async fn direct(&self, command: u8, payload: &[u8]) -> Result<(), BackendError> {
        let pkt = p::try_request(command, payload).map_err(BackendError::Runtime)?;
        let _guard = self.op_lock.lock().await;
        if !self.is_connected() {
            return Err(BackendError::Connection(
                "Kết nối tai nghe trước khi điều chỉnh.".to_string(),
            ));
        }
        self.send(&pkt).await?;
        self.emit_event(BackendEvent::Notice(
            "Đã gửi lệnh. Thiết bị có thể ngắt kết nối; hãy đọc lại để kiểm tra.".to_string(),
        ));
        Ok(())
    }

    pub async fn set_key(
        &self,
        side: &str,
        gesture: &str,
        action: &str,
    ) -> Result<(), BackendError> {
        let _guard = self.op_lock.lock().await;
        if !self.is_connected() {
            return Err(BackendError::Connection(
                "Chưa kết nối tai nghe.".to_string(),
            ));
        }
        {
            let fw = lock(&self.state).version.clone();
            if fw
                .as_deref()
                .and_then(p::parse_firmware_version_code)
                .is_some()
            {
                let allowed = p::allowed_operations(gesture, fw.as_deref());
                if !allowed.iter().any(|(name, _)| *name == action) {
                    return Err(BackendError::Runtime(
                        "Chức năng cảm ứng này không được hỗ trợ cho loại chạm hoặc phiên bản firmware hiện tại."
                            .to_string(),
                    ));
                }
            }
        }
        let pkt = p::gesture_packet(side, gesture, action).map_err(BackendError::Runtime)?;
        self.send(&pkt).await?;
        let field = format!("{}.{}", side, gesture);
        self.verify_until(
            11,
            Duration::from_millis(300),
            "Tai nghe chưa xác nhận thao tác cảm ứng mới.",
            |_, upd| upd.keys.get(&field).map(|s| s.as_str()) == Some(action),
        )
        .await?;
        self.emit_event(BackendEvent::Notice(
            "Đã xác nhận thao tác cảm ứng.".to_string(),
        ));
        Ok(())
    }

    pub async fn set_eq(&self, levels: &[i32]) -> Result<(), BackendError> {
        let packets = p::equalizer(levels).map_err(BackendError::Runtime)?;
        let _guard = self.op_lock.lock().await;
        if !self.is_connected() {
            return Err(BackendError::Connection(
                "Chưa kết nối tai nghe.".to_string(),
            ));
        }
        for pkt in packets {
            self.send(&pkt).await?;
        }
        self.emit_event(BackendEvent::Notice(
            "Đã ghi đủ 10 gói EQ. Chưa có xác nhận đọc lại đường cong EQ.".to_string(),
        ));
        Ok(())
    }

    pub async fn codecs(&self, values: &BTreeMap<String, bool>) -> Result<(), BackendError> {
        let _guard = self.op_lock.lock().await;
        if !self.is_connected() {
            return Err(BackendError::Connection(
                "Kết nối tai nghe trước khi điều chỉnh.".to_string(),
            ));
        }
        let (has_all, dual_on) = {
            let st = lock(&self.state);
            let all = ["ldac", "aptx", "adaptive", "lossless", "dual"]
                .iter()
                .all(|k| st.get_bool(k).is_some());
            (all, st.dual.unwrap_or(false))
        };
        if !has_all {
            return Err(BackendError::Runtime(
                "Chưa đọc đủ trạng thái codec / đa điểm.".to_string(),
            ));
        }
        let wanted_ldac = *values.get("ldac").unwrap_or(&false);
        if wanted_ldac && dual_on {
            return Err(BackendError::Runtime(
                "Tắt đa điểm trước khi bật LDAC.".to_string(),
            ));
        }
        let mut bits: u8 = 0;
        for (key, bit) in [
            ("aptx", 1u8),
            ("adaptive", 2u8),
            ("ldac", 8u8),
            ("lossless", 16u8),
        ] {
            if *values.get(key).unwrap_or(&false) {
                bits |= bit;
            }
        }
        self.send(&p::request(46, &[bits])).await?;
        self.verify_until(
            47,
            Duration::from_millis(400),
            "Tai nghe chưa xác nhận đủ các codec.",
            |st, _| values.iter().all(|(k, &v)| st.get_bool(k) == Some(v)),
        )
        .await?;
        self.emit_event(BackendEvent::Notice(
            "Tai nghe đã xác nhận thay đổi.".to_string(),
        ));
        Ok(())
    }

    pub async fn disable_touch(&self, left: bool, right: bool) -> Result<(), BackendError> {
        let _guard = self.op_lock.lock().await;
        if !self.is_connected() {
            return Err(BackendError::Connection(
                "Kết nối tai nghe trước khi điều chỉnh.".to_string(),
            ));
        }
        {
            let st = lock(&self.state);
            if st.dis_l.is_none() || st.dis_r.is_none() {
                return Err(BackendError::Runtime(
                    "Chưa đọc đủ trạng thái khóa cảm ứng hai tai.".to_string(),
                ));
            }
        }
        let value = if left && right {
            1
        } else if left {
            2
        } else if right {
            3
        } else {
            0
        };
        self.send(&p::request(52, &[value])).await?;
        self.verify_until(
            53,
            Duration::from_millis(400),
            "Chưa xác nhận khóa cảm ứng hai tai.",
            |st, _| st.dis_l == Some(left) && st.dis_r == Some(right),
        )
        .await?;
        self.emit_event(BackendEvent::Notice(
            "Tai nghe đã xác nhận thay đổi.".to_string(),
        ));
        Ok(())
    }

    pub async fn phones_action(&self, command: u8, num: Option<u8>) -> Result<(), BackendError> {
        let _guard = self.op_lock.lock().await;
        if !self.is_connected() {
            return Err(BackendError::Connection(
                "Chưa kết nối tai nghe.".to_string(),
            ));
        }
        if command == 41 || command == 43 {
            let connected_count = lock(&self.phones).values().filter(|p| p.connected).count();
            if connected_count >= 2 {
                return Err(BackendError::Runtime(
                    "Đã kết nối hai thiết bị. Ngắt một thiết bị trước.".to_string(),
                ));
            }
        }
        if command != 44 {
            let payload: Vec<u8> = num.into_iter().collect();
            self.send(&p::request(command, &payload)).await?;
            let sleep_dur = *lock(&self.step_sleep);
            tokio::time::sleep(sleep_dur).await;
        }
        self.phones_internal().await?;
        self.emit_event(BackendEvent::Notice(if command != 43 {
            "Đã đọc danh sách thiết bị.".to_string()
        } else {
            "Đã mở ghép nối. Dùng Cài đặt Bluetooth trên thiết bị mới, rồi bấm Làm mới.".to_string()
        }));
        Ok(())
    }

    pub(crate) async fn stop_find(&self) -> Result<(), BackendError> {
        self.find_seq.fetch_add(1, Ordering::SeqCst);
        let abort_opt = lock(&self.finder_abort).take();
        if let Some(handle) = abort_opt {
            // Never abort the auto-stop timer task from inside its own execution!
            if Some(handle.id()) != tokio::task::try_id() {
                handle.abort();
            }
        }
        if !self.is_connected() {
            self.finding.store(false, Ordering::SeqCst);
            return Ok(());
        }
        self.send(&p::request(50, &[0])).await?;
        self.finding.store(false, Ordering::SeqCst);
        self.emit_event(BackendEvent::Notice("Đã gửi dừng tiếng tìm.".to_string()));
        Ok(())
    }

    pub async fn find(self: &Arc<Self>, mask: u8) -> Result<(), BackendError> {
        if mask == 0 {
            return self.stop_find().await;
        }
        let _guard = self.op_lock.lock().await;
        if !self.is_connected() {
            return Err(BackendError::Connection(
                "Chưa kết nối tai nghe.".to_string(),
            ));
        }
        let my_seq = self.find_seq.fetch_add(1, Ordering::SeqCst) + 1;
        let epoch = self.session_epoch.load(Ordering::SeqCst);
        let abort_opt = lock(&self.finder_abort).take();
        if let Some(handle) = abort_opt {
            handle.abort();
        }

        self.send(&p::request(50, &[mask])).await?;
        if self.find_seq.load(Ordering::SeqCst) != my_seq
            || self.session_epoch.load(Ordering::SeqCst) != epoch
        {
            if self.is_connected() && self.session_epoch.load(Ordering::SeqCst) == epoch {
                let _ = self.send(&p::request(50, &[0])).await;
            }
            return Ok(());
        }
        self.finding.store(true, Ordering::SeqCst);

        let duration = *lock(&self.find_duration);
        let weak = Arc::downgrade(self);
        let join = tokio::spawn(async move {
            tokio::time::sleep(duration).await;
            if let Some(strong) = weak.upgrade() {
                if strong.session_epoch.load(Ordering::SeqCst) != epoch
                    || strong.find_seq.load(Ordering::SeqCst) != my_seq
                {
                    return;
                }
                if let Err(e) = strong.stop_find().await {
                    strong.emit_event(BackendEvent::Error(format!(
                        "Không gửi được dừng tiếng tìm: {}",
                        e
                    )));
                }
            }
        });
        *lock(&self.finder_abort) = Some(join.abort_handle());
        self.emit_event(BackendEvent::Notice(
            "Đang phát tiếng tìm; tự dừng sau 10 giây.".to_string(),
        ));
        Ok(())
    }

    pub async fn run_plan(self: &Arc<Self>, plan: &Plan) -> Result<(), BackendError> {
        let _guard = self.op_lock.lock().await;
        if self.closing.load(Ordering::SeqCst) {
            return Err(BackendError::Runtime(
                "Ứng dụng đang đóng; chưa gửi thay đổi.".to_string(),
            ));
        }
        if !self.is_connected() {
            return Err(BackendError::Connection(
                "Kết nối tai nghe trước khi thay đổi thiết lập.".to_string(),
            ));
        }
        if plan.steps.is_empty() {
            self.emit_event(BackendEvent::Notice(
                "Tai nghe đã ở thiết lập này; không cần gửi lại.".to_string(),
            ));
            return Ok(());
        }
        for &cmd in &plan.queries {
            let reply = self.request(&p::request(cmd, &[])).await?;
            self.apply(&reply);
        }
        {
            let st = lock(&self.state);
            if plan
                .baseline
                .iter()
                .any(|(k, v)| st.get_bool(k) != Some(*v))
            {
                return Err(BackendError::Runtime(
                    "Thiết lập tai nghe đã đổi trong lúc chờ xác nhận. Hãy chọn lại theo trạng thái mới."
                        .to_string(),
                ));
            }
        }

        let mut completed: Vec<String> = Vec::new();
        for (idx, step) in plan.steps.iter().enumerate() {
            if self.closing.load(Ordering::SeqCst) {
                if !self.is_connected() {
                    self.disconnect_internal().await;
                }
                let err = "Đã dừng quy trình vì ứng dụng đang đóng.";
                let detail = if completed.is_empty() {
                    String::new()
                } else {
                    format!("Đã hoàn tất: {}. ", completed.join("; "))
                };
                return Err(BackendError::Runtime(format!(
                    "{}Chưa xác nhận hoàn tất toàn bộ thay đổi. {}",
                    detail, err
                )));
            }
            self.emit_event(BackendEvent::Notice(format!(
                "{}/{} · {}…",
                idx + 1,
                plan.steps.len(),
                step.title
            )));
            if let Err(e) = self.run_restart_step(step).await {
                if self.is_connected() && !self.closing.load(Ordering::SeqCst) {
                    let _ = self.query_internal(true).await;
                    if self.is_connected() {
                        self.set_phase(ConnectionPhase::Ready);
                        self.set_connection_status(super::ConnectionStatus::Connected);
                        self.ensure_polling();
                    } else {
                        self.disconnect_internal().await;
                    }
                } else if !self.is_connected() {
                    self.disconnect_internal().await;
                }
                let detail = if completed.is_empty() {
                    String::new()
                } else {
                    format!("Đã hoàn tất: {}. ", completed.join("; "))
                };
                return Err(BackendError::Runtime(format!(
                    "{}Chưa xác nhận hoàn tất toàn bộ thay đổi. {}",
                    detail, e
                )));
            }
            completed.push(step.title.clone());
        }

        if let Err(e) = self.query_internal(true).await {
            if !self.is_connected() {
                self.disconnect_internal().await;
            } else {
                self.set_connection_status(super::ConnectionStatus::Connected);
                self.ensure_polling();
            }
            let detail = if completed.is_empty() {
                String::new()
            } else {
                format!("Đã hoàn tất: {}. ", completed.join("; "))
            };
            return Err(BackendError::Runtime(format!(
                "{}Chưa xác nhận hoàn tất toàn bộ thay đổi. {}",
                detail, e
            )));
        }
        self.set_phase(ConnectionPhase::Ready);
        self.set_connection_status(super::ConnectionStatus::Connected);
        self.ensure_polling();

        let target_mismatch = {
            let st = lock(&self.state);
            plan.target.iter().any(|(k, v)| st.get_bool(k) != Some(*v))
        };
        if target_mismatch {
            if !self.is_connected() {
                self.disconnect_internal().await;
            }
            let detail = if completed.is_empty() {
                String::new()
            } else {
                format!("Đã hoàn tất: {}. ", completed.join("; "))
            };
            return Err(BackendError::Runtime(format!(
                "{}Chưa xác nhận hoàn tất toàn bộ thay đổi. Trạng thái cuối khác lựa chọn đã xác nhận.",
                detail
            )));
        }

        self.emit_event(BackendEvent::Notice(format!(
            "Đã hoàn tất và đọc lại xác nhận: {}.",
            plan.title
        )));
        Ok(())
    }

    async fn run_restart_step(self: &Arc<Self>, step: &Step) -> Result<(), BackendError> {
        let address = lock(&self.address).clone();
        if let Err(e) = self.send(&p::request(step.setter, &[step.value])).await {
            self.record(
                "?",
                &format!("Kiểm tra sau khi gửi lệnh bị gián đoạn: {}", e),
            );
        }

        let step_sleep = *lock(&self.step_sleep);
        let deadline = tokio::time::Instant::now() + *lock(&self.restart_deadline);

        for attempt in 0..25u8 {
            let now = tokio::time::Instant::now();
            if now >= deadline {
                break;
            }
            if self.closing.load(Ordering::SeqCst) {
                return Err(BackendError::Runtime(
                    "Đã dừng quy trình vì ứng dụng đang đóng.".to_string(),
                ));
            }
            tokio::time::sleep(step_sleep).await;

            if !self.is_connected() {
                let Some(ref addr) = address else {
                    return Err(BackendError::Connection(
                        "Không còn địa chỉ tai nghe để kết nối lại.".to_string(),
                    ));
                };
                self.set_phase(ConnectionPhase::Reconnecting {
                    attempt: attempt + 1,
                    max_attempts: 25,
                });
                self.emit_event(BackendEvent::Notice(format!(
                    "Tai nghe đang khởi động lại. Đang kết nối lại (lần {})…",
                    attempt + 1
                )));
                let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
                if remaining.is_zero() {
                    break;
                }
                let hook_opt = lock(&self.reconnect_hook).clone();
                let reconn_fut = async {
                    if let Some(hook) = hook_opt {
                        hook(addr.clone(), false).await
                    } else {
                        self.connect_internal(addr, false).await
                    }
                };
                match tokio::time::timeout(remaining, reconn_fut).await {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => {
                        self.record("?", &format!("Đọc lại sau thay đổi: {}", e));
                        continue;
                    }
                    Err(_) => {
                        if !self.is_connected() {
                            self.disconnect_internal().await;
                        }
                        break;
                    }
                }
            }

            match self.request(&p::request(step.query, &[])).await {
                Ok(reply) => {
                    self.apply(&reply);
                    let matched = {
                        let st = lock(&self.state);
                        step.wanted.iter().all(|(k, v)| st.get_bool(k) == Some(*v))
                    };
                    if matched {
                        return Ok(());
                    }
                }
                Err(BackendError::CommandRejected(msg)) => {
                    return Err(BackendError::CommandRejected(msg));
                }
                Err(e) => {
                    self.record("?", &format!("Đọc lại sau thay đổi: {}", e));
                }
            }
        }

        if !self.is_connected() {
            self.disconnect_internal().await;
        }
        Err(BackendError::Runtime(format!(
            "Tai nghe chưa xác nhận bước “{}”. Trạng thái đang hiển thị là phản hồi đọc được gần nhất.",
            step.title
        )))
    }
}
