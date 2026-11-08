# EarFun Studio (Rust · Windows & Ubuntu)

Ứng dụng desktop tiếng Việt viết bằng **Rust** điều khiển tai nghe **EarFun Air Pro 4 / TW600** cho cả **Windows 10/11** và **Ubuntu Linux**.

- **Giao diện**: `eframe` / `egui` (nhúng sẵn phông chữ *Be Vietnam Pro* và đồ thị đường cong EQ 10 dải thời gian thực).
- **Bluetooth LE**: `btleplug` (dùng WinRT BLE nguyên bản trên Windows và BlueZ D-Bus trên Ubuntu).
- **Âm lượng hệ thống**: PipeWire / WirePlumber trên Ubuntu (chỉ điều khiển đúng sink Bluetooth của tai nghe) và lối tắt Cài đặt âm thanh/Bluetooth trên Windows & Ubuntu.
- Ứng dụng độc lập, chỉ gồm **1 tệp thực thi duy nhất**, không cần cài đặt Python hay GTK4.

## Tải về và sử dụng

### 1. Windows 10 / 11 (64-bit)

1. Tải `EarFunStudio-Windows-1.4.0-x86_64.exe` (hoặc `.zip`) từ trang [GitHub Releases](https://github.com/lnb2x/earfun-studio/releases).
2. Ghép đôi **EarFun Air Pro 4** trong **Settings → Bluetooth & devices** của Windows.
3. Mở `EarFunStudio-Windows-1.4.0-x86_64.exe`. Ứng dụng tự tìm và kết nối tai nghe; không cần bấm quét/kết nối. Nếu có nhiều tai nghe, chọn đúng thiết bị một lần để ứng dụng ghi nhớ.

### 2. Ubuntu 24.04+ (64-bit)

**Cách 1 — Cài từ gói `.deb`:**

```sh
sudo apt install ./earfun-studio_1.4.0_amd64.deb
earfun-studio
```

**Cách 2 — Dùng gói `.tar.gz` hoặc biên dịch từ mã nguồn:**

```sh
./install.sh
./run.sh
```

## Chức năng chính

- **Tổng quan**: Pin tai trái / tai phải / hộp sạc, firmware, codec hiện hành, chống ồn (ANC 5 chế độ) / xuyên âm (2 chế độ) / bình thường, chế độ chơi game (Game Mode), cảm biến nhận biết đeo tai.
- **Equalizer 10 dải**: Chỉnh từ −6 đến +6 dB theo đúng bảng gain phần cứng TW600; tích hợp sẵn **30 mẫu EarFun + 2 mẫu Fernanda** từ APK 20.1.61; tìm kiếm mẫu, lưu/đổi tên/xóa bản chỉnh cá nhân, nhập/xuất tệp JSON (`earfun-studio.eq`).
- **Cảm ứng**: Tùy chỉnh 8 thao tác chạm (Chạm 1/2/3 lần, Nhấn giữ cho từng bên tai) và khóa cảm ứng riêng từng tai.
- **Hai thiết bị (Multipoint)**: Bật/tắt kết nối đa điểm (tự động xử lý phụ thuộc với LDAC), xem danh sách thiết bị đã ghép, kết nối/ngắt hoặc mở ghép thiết bị mới.
- **Tai nghe**: Âm lượng thông báo, micro cuộc gọi, giới hạn âm lượng bảo vệ thính giác, cân bằng trái/phải, chuyển đổi chế độ Bluetooth (*Thông thường · Google Fast Pair* / *LE Audio*), bật/tắt các codec (*LDAC, aptX, aptX Adaptive, aptX Lossless*), tìm tai nghe (tự dừng sau 10 giây), khôi phục cài đặt gốc.
- **Ứng dụng**: Tự tìm và kết nối khi mở ứng dụng, tự thử lại khi tai nghe ra khỏi hộp, xem và xuất nhật ký gói tin BLE theo thời gian thực.

## Biên dịch và kiểm thử từ mã nguồn (Rust)

### Yêu cầu

- Rust 1.80+ (`cargo`, `rustc`)
- Trên Ubuntu cần các gói phát triển hệ thống:
  ```sh
  sudo apt install pkg-config libdbus-1-dev libglib2.0-dev libgtk-3-dev libxkbcommon-dev libwayland-dev libx11-dev libgl1-mesa-dev
  ```

### Lệnh kiểm thử & đóng gói

```sh
# Chạy toàn bộ kiểm thử đơn vị (giao thức, preset, cài đặt, quy trình khởi động lại, xung đột LDAC)
cargo test

# Kiểm tra khởi tạo giao diện 6 trang (không truy cập Bluetooth)
cargo run --release -- --smoke-test

# Đóng gói cho Ubuntu (.tar.gz và .deb trong dist/)
./packaging/build.sh

# Đóng gói cho Windows (PowerShell)
pwsh ./packaging/build-windows.ps1
```

## Cấu hình

- **Ubuntu / Linux**: `${XDG_CONFIG_HOME:-~/.config}/earfun-studio/settings.json`
- **Windows**: `%APPDATA%\earfun-studio\settings.json`


## Tự kết nối và xử lý lỗi Bluetooth trong 1.4

Tự kết nối được bật sẵn trên Windows và Ubuntu, kể cả khi nâng cấp từ mặc định tắt
của 1.3. Ưu tiên địa chỉ tai nghe đã kết nối thành công; lần đầu tự chọn khi chỉ có
một Air Pro 4. Nếu có nhiều tai nghe, chọn thiết bị một lần. Sau khi chủ động bấm
**Ngắt kết nối**, ứng dụng không tự nối lại trong phiên đó. Có thể tắt tự kết nối
trong trang **Ứng dụng**; lựa chọn mới được lưu qua các lần mở.

Tai nghe cần được lấy ra khỏi hộp, bật Bluetooth và ghép trong cài đặt hệ điều hành
để dùng âm thanh. Ứng dụng chỉ xác nhận kết nối sau khi đọc được firmware TW600.

Nếu gặp `Service discovery timed out` sau khi cài lại Windows hoặc ghép lại tai nghe
trên một hệ điều hành: đồng bộ **cả khóa BR/EDR (LinkKey) và BLE (LTK, IRK, EDiv,
Rand)** giữa Windows và Ubuntu. Chỉ đồng bộ LinkKey có thể khôi phục nghe nhạc nhưng
không khôi phục kênh điều khiển BLE. Xem [xử lý kết nối](docs/bluetooth-troubleshooting.md).

Kiểm tra kết nối thật, chỉ đọc trạng thái, không thay đổi cài đặt tai nghe:

```sh
cargo run --example diagnose
# Hoặc chỉ định địa chỉ đã biết:
cargo run --example diagnose -- AA:BB:CC:DD:EE:FF
```
