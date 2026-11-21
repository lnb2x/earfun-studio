## Tóm tắt (Summary)

<!-- Mô tả ngắn gọn thay đổi trong PR này -->

## Động lực (Motivation)

<!-- Vì sao cần thay đổi này? Liên kết issue nếu có -->

## Thay đổi chính (Changes)

-

## Kiểm thử (Testing)

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --locked --all-targets -- -D warnings`
- [ ] `cargo test --locked`
- [ ] `EARFUN_HEADLESS=1 cargo run --locked -- --smoke-test` (12 dòng `UI OK`)

## Ảnh chụp giao diện (Screenshots - nếu thay đổi UI)

<!-- Đính kèm ảnh chụp (cô lập cấu hình, không để lộ MAC hoặc preset cá nhân nếu dùng dữ liệu demo) -->

## Lưu ý tương thích ngược / Hồi quy (Regression Considerations)

<!-- Thay đổi có ảnh hưởng định dạng `settings.json`, CLI hoặc gói phát hành không? -->

## Danh sách kiểm tra an toàn phần cứng (`AGENTS.md`)

- [ ] Không đưa `getter 113` vào `protocol::QUERIES`
- [ ] Không tự động ghi EQ xuống tai nghe khi khởi động, nạp preset, đặt lại `0 dB` hay lưu mẫu cá nhân
- [ ] `close_session()` trên Linux không gọi `peripheral.disconnect()` (giữ luồng âm thanh A2DP)
- [ ] Các thao tác khởi động lại, *Tìm tai nghe* và *Khôi phục cài đặt gốc* đều đi qua `ask_confirm`
- [ ] Không dùng `.lock().unwrap()` hay giữ `std::sync::MutexGuard` qua `.await` trong `src/backend/`
- [ ] Nhật ký xuất/sao chép đi qua `redact_sensitive_identifiers`; mọi ghi tệp người dùng dùng `settings::write_atomic`
- [ ] Không thêm glyph thiếu trong phông `BeVietnamPro` vào chuỗi UI
