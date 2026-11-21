# AGENTS.md — Quy tắc Kỹ thuật cho EarFun Studio

Tệp này ghi lại các bất biến kiến trúc và an toàn phần cứng bắt buộc khi phát triển hoặc bảo trì **EarFun Studio** (`EarFun Air Pro 4 / TW600`).

## 1. An toàn Phần cứng & Giao thức TW600 (Bắt buộc)

- Đọc kỹ `docs/apk-feature-audit.md` và `docs/bluetooth-troubleshooting.md` trước khi chạm vào `src/protocol.rs`, `src/workflows.rs` hoặc `src/backend/`.
- Không bao giờ đoán mã lệnh GAIA, định dạng payload hoặc giả lập các tính năng phụ thuộc OTA/phần cứng chưa kiểm chứng (*Firmware OTA*, *Gói giọng OTA*, *Adaptive EQ*, *Auracast GBSS*).
- Không đưa `getter 113` quay lại `protocol::QUERIES` (gây timeout trên firmware thực tế `v0.4.1`).
- Không tự động gửi lệnh ghi EQ khi khởi động ứng dụng, nạp preset, đặt lại `0 dB` hoặc lưu mẫu cá nhân; chỉ gửi 10 gói EQ khi người dùng chủ động bấm **Gửi tới tai nghe**.
- Trên Linux/Ubuntu, `close_session()` chỉ gọi `unsubscribe(&rx_char)`, không gọi `peripheral.disconnect()` để tránh ngắt âm thanh BR/EDR A2DP của hệ thống.
- Mọi thao tác gây ngắt âm thanh, khởi động lại tai nghe, phát âm lượng lớn (*Tìm tai nghe*) hoặc xóa dữ liệu (*Khôi phục cài đặt gốc*) đều phải đi qua hộp thoại xác nhận `ask_confirm`.
- `stop_find` (lệnh `50 [0]`) không được giữ `op_lock`, để nút **Dừng ngay** và bộ hẹn giờ 10 giây không bị chặn sau các thao tác đọc/ghi dài khác.

## 2. Quy tắc Đồng thời, Lưu trữ & Giao diện

- Trong `src/backend/`, không dùng `.lock().unwrap()` trực tiếp và tuyệt đối không giữ `std::sync::MutexGuard` qua `.await`.
- Mọi thay đổi ghi tệp người dùng (`settings.json`, xuất EQ JSON, xuất nhật ký `.log`) phải đi qua `settings::write_atomic` và không bao giờ âm thầm ghi đè mất `settings.json` lỗi mà chưa sao lưu `.corrupt.bak` (hoặc khóa ghi nếu sao lưu thất bại).
- Khi sao chép hoặc xuất nhật ký BLE, luôn đi qua `protocol::redact_sensitive_identifiers` để ẩn địa chỉ MAC.
- Ghi chú phát hành đặt tại `docs/releases/vX.Y.Z.md`; ảnh chụp xem trước UI phải dùng `examples/preview.rs` với thư mục cấu hình cô lập để không làm lộ dữ liệu cá nhân.
- Trong giao diện `egui` (`src/ui/`):
  - Dùng bảng màu động `ui_palette(ui)` / `palette(ctx)` để hỗ trợ đầy đủ cả giao diện **Sáng** và **Tối**.
  - Dùng `setting_row` và `draw_card_header_with_right` (cấp phát vùng điều khiển bên phải trước, giới hạn chiều rộng văn bản bên trái sau) để không bao giờ tràn hoặc đè chữ ở kích thước tối thiểu `860×620`.
  - Mọi ký tự đặc biệt mới đưa vào chuỗi giao diện phải vượt qua kiểm thử độ phủ glyph trong `src/ui/tests.rs` (phông `BeVietnamPro` không có `→` `U+2192`; dùng `•`, `·` hoặc biểu tượng vector).

## 3. Cổng Kiểm chứng Chất lượng Trước Khi Commit / Phát hành

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
EARFUN_HEADLESS=1 cargo run --locked -- --smoke-test
```
