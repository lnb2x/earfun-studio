# Hướng dẫn đóng góp (Contributing)

Cảm ơn bạn đã quan tâm tới **EarFun Studio**! Trước khi thay đổi mã nguồn, hãy đọc kỹ [`AGENTS.md`](AGENTS.md) để nắm các bất biến an toàn phần cứng TW600.

## Cấu trúc dự án

- `src/protocol.rs`: Mã hóa/giải mã khung lệnh GAIA/TW600, bảng gain EQ, ẩn MAC.
- `src/workflows.rs`: Kế hoạch chuyển đổi trạng thái có phụ thuộc (LDAC ↔ Multipoint, LE Audio, Nhận biết đeo tai).
- `src/backend/`: Máy trạng thái BLE, truyền tải `btleplug` / `GattClient`, `commands`, `lifecycle`.
- `src/settings.rs`, `src/presets.rs`, `src/audio.rs`: Lưu trữ nguyên tử, preset EQ, PipeWire/URI an toàn.
- `src/ui/`: Giao diện `eframe` / `egui` (Sáng/Tối/Hệ thống, biểu tượng vector, 6 trang chức năng).
- `packaging/`: Kịch bản đóng gói `.tar.gz`/`.deb` (Ubuntu) và `.exe`/`.zip` (Windows).
- `docs/`: Tài liệu kiến trúc, đối chiếu APK, xử lý sự cố và ghi chú phát hành tại `docs/releases/`.

## Thiết lập & Kiểm thử

Chi tiết cài đặt thư viện hệ thống xem tại [`docs/development.md`](docs/development.md). Trước khi gửi Pull Request, mọi thay đổi phải vượt qua toàn bộ cổng kiểm chứng:

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
EARFUN_HEADLESS=1 cargo run --locked -- --smoke-test
```

## Quy ước mã nguồn & phần cứng

- Không thêm `getter 113` vào `protocol::QUERIES`.
- Không tự động gửi gói EQ xuống tai nghe; chỉ gửi 10 gói khi người dùng bấm **Gửi tới tai nghe**.
- Trên Linux/Ubuntu, `close_session()` chỉ `unsubscribe(&rx_char)` và không gọi `peripheral.disconnect()`.
- Mọi hành động rủi ro (phát âm tìm, khôi phục cài đặt gốc, đổi codec/LE Audio) phải qua `ask_confirm`.
- Không giữ `std::sync::MutexGuard` qua `.await` và không dùng `.lock().unwrap()` trong `src/backend/`.
- Mọi chuỗi giao diện mới trong `src/ui/` phải dùng bảng màu động và vượt qua kiểm thử glyph *Be Vietnam Pro*.

## Quy trình Commit & Pull Request

- Dùng định dạng Conventional Commits (`fix(backend): ...`, `fix(ui): ...`, `docs: ...`, `ci: ...`).
- Mở Pull Request nhắm vào nhánh mặc định **`codex/ubuntu-app`** và điền đầy đủ mẫu kiểm tra trong PR template.
- Tuyệt đối không đính kèm địa chỉ MAC Bluetooth đầy đủ hoặc khóa ghép đôi vào log, ảnh chụp hay issue.
