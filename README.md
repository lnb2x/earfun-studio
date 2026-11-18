# EarFun Studio (Rust · Windows & Ubuntu)

Ứng dụng desktop tiếng Việt viết bằng **Rust** điều khiển tai nghe **EarFun Air Pro 4 / TW600** cho **Windows 10/11** và **Ubuntu Linux**.

- **Giao diện hiện đại**: `eframe` / `egui` với hệ thống thiết kế **Sáng / Tối / Theo hệ thống**, nhúng sẵn phông chữ *Be Vietnam Pro*, biểu tượng vector sắc nét và đồ thị đường cong EQ 10 dải tương tác thời gian thực.
- **Bluetooth LE tin cậy**: `btleplug` (dùng WinRT BLE nguyên bản trên Windows và BlueZ D-Bus trên Ubuntu) với máy trạng thái kết nối chi tiết, tự làm mới bộ thu Bluetooth và xác nhận đọc lại đa trường.
- **Âm lượng hệ thống an toàn**: Điều khiển bất đồng bộ qua PipeWire / WirePlumber trên Ubuntu (chỉ tác động đúng sink Bluetooth của tai nghe) và lối tắt Cài đặt âm thanh / Bộ trộn âm lượng trên Windows & Ubuntu.
- **Độc lập & Gọn nhẹ**: Chỉ gồm **1 tệp thực thi duy nhất**, không cần cài đặt Python hay runtime phụ.

## Tải về và sử dụng

### 1. Windows 10 / 11 (64-bit)

1. Tải `EarFunStudio-Windows-1.5.0-x86_64.exe` (hoặc `.zip`) từ trang [GitHub Releases](https://github.com/lnb2x/earfun-studio/releases).
2. Ghép đôi **EarFun Air Pro 4** trong **Settings → Bluetooth & devices** của Windows.
3. Mở `EarFunStudio-Windows-1.5.0-x86_64.exe`. Ứng dụng tự tìm và kết nối tai nghe ngay khi mở; nếu có nhiều tai nghe, chọn đúng thiết bị một lần để ứng dụng ghi nhớ.

### 2. Ubuntu 22.04 / 24.04+ (64-bit)

**Cách 1 — Cài từ gói `.deb`:**

```sh
sudo apt install ./earfun-studio_1.5.0_amd64.deb
earfun-studio
```

**Cách 2 — Dùng gói `.tar.gz` hoặc biên dịch từ mã nguồn:**

```sh
./install.sh
./run.sh
```

Gỡ biểu tượng menu ứng dụng người dùng (giữ nguyên cấu hình cá nhân):

```sh
./uninstall.sh
```

## Chức năng chính (6 Trang)

1. **Tổng quan**: Pin tai trái / tai phải / hộp sạc, firmware, codec hiện hành, trạng thái kết nối BLE chi tiết, chống ồn chủ động ANC (5 chế độ) / xuyên âm (2 chế độ) / bình thường, chế độ chơi game (*Game Mode <50ms*), cảm biến nhận biết đeo tai.
2. **Equalizer 10 dải**: Chỉnh từ −6 đến +6 dB theo đúng bảng gain phi tuyến của phần cứng TW600; tích hợp sẵn **30 mẫu EarFun + 2 mẫu Fernanda** trích xuất từ APK `20.1.61`; kéo trực tiếp điểm trên đồ thị Bezier hoặc thanh gạt tâm `0 dB` (hỗ trợ phím mũi tên, cuộn chuột, nhấp đúp về `0 dB`); lưu/đổi tên/xóa mẫu cá nhân và nhập/xuất JSON (`earfun-studio.eq`).
3. **Cảm ứng**: Tùy chỉnh 8 thao tác chạm (Chạm 1/2/3 lần, Nhấn giữ cho từng bên tai, tự động lọc đúng các thao tác phần cứng TW600 và phiên bản firmware hỗ trợ) cùng khóa cảm ứng riêng từng tai.
4. **Hai thiết bị (Multipoint)**: Bật/tắt kết nối đa điểm (tự động xử lý phụ thuộc hai chiều với codec LDAC), xem đầy đủ danh sách thiết bị đã ghép trên tai nghe, kết nối/ngắt hoặc mở chế độ ghép thiết bị mới.
5. **Tai nghe**: Âm lượng giọng thông báo, micro đàm thoại, giới hạn trần âm lượng bảo vệ thính giác, cân bằng âm lượng Trái/Phải, chuyển đổi chế độ Bluetooth (*Thông thường · Google Fast Pair* ↔ *LE Audio*), bật/tắt các codec (*LDAC, aptX, aptX Adaptive, aptX Lossless*), tìm tai nghe (đếm ngược tự dừng sau 10 giây), khôi phục cài đặt gốc.
6. **Ứng dụng**: Chuyển đổi giao diện **Sáng / Tối / Hệ thống** (`Alt + T`), tùy chọn tự kết nối lại mỗi 8 giây, tra cứu phím tắt (`Alt + 1..6`, `F5`, `Enter/Esc`), xem/lọc và sao chép/xuất nhật ký gói tin BLE (tự động ẩn địa chỉ MAC nhạy cảm).

## Biên dịch và kiểm thử từ mã nguồn (Rust)

### Yêu cầu

- Rust stable (`cargo`, `rustc`)
- Trên Ubuntu cần các gói phát triển hệ thống:
  ```sh
  sudo apt install pkg-config libdbus-1-dev libglib2.0-dev libgtk-3-dev libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libx11-dev libgl1-mesa-dev
  ```

### Lệnh kiểm tra chất lượng & đóng gói

```sh
# Kiểm tra định dạng mã nguồn & cảnh báo tĩnh Clippy
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings

# Chạy toàn bộ kiểm thử tự động (giao thức, preset, cấu hình, backend BLE giả lập, độ phủ phông chữ & bố cục Sáng/Tối)
cargo test --locked

# Kiểm tra khởi tạo giao diện 6 trang không cần phần cứng Bluetooth
EARFUN_HEADLESS=1 cargo run --locked --release -- --smoke-test

# Đóng gói cho Ubuntu (.tar.gz và .deb trong dist/)
./packaging/build.sh

# Đóng gói cho Windows (PowerShell - tạo .exe và .zip trong dist/)
pwsh ./packaging/build-windows.ps1
```

## Cấu hình & Tài liệu Kỹ thuật

- **Ubuntu / Linux**: `${XDG_CONFIG_HOME:-~/.config}/earfun-studio/settings.json` (quyền `0600`)
- **Windows**: `%APPDATA%\earfun-studio\settings.json`
- Nếu tệp cấu hình bị hỏng ngoài ý muốn, ứng dụng tự động sao lưu sang `settings.json.corrupt.bak` để không làm mất dữ liệu preset cá nhân.
- Xem thêm:
  - [Kiến trúc kỹ thuật & Máy trạng thái BLE](docs/architecture.md)
  - [Xử lý sự cố Bluetooth & Dual-boot Windows/Ubuntu](docs/bluetooth-troubleshooting.md)
  - [Đối chiếu tính năng và giới hạn phần cứng với APK 20.1.61](docs/apk-feature-audit.md)

## Giới hạn đã xác minh

- Giao thức TW600 hỗ trợ ghi trực tiếp 10 dải EQ xuống DSP tai nghe nhưng không hỗ trợ lệnh đọc ngược đường cong EQ từ tai nghe.
- Các quy trình cập nhật Firmware OTA, cài gói giọng thông báo OTA mới, đo thính lực cá nhân hóa (*Adaptive EQ*) và quản lý nguồn phát *Auracast (GBSS)* không được giả lập trên máy tính để đảm bảo an toàn phần cứng; vui lòng sử dụng ứng dụng **EarFun Audio** trên điện thoại cho các tác vụ này.

Kiểm tra kết nối thực tế ở chế độ chỉ đọc (không thay đổi bất kỳ cài đặt nào của tai nghe):

```sh
cargo run --example diagnose
# Hoặc chỉ định địa chỉ MAC đã biết:
cargo run --example diagnose -- AA:BB:CC:DD:EE:FF
```
