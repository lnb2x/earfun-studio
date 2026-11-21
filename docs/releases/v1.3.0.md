# EarFun Studio 1.3.0 — Bản viết lại bằng Rust (Windows & Ubuntu)

## Điểm mới trong 1.3.0

- **Viết lại hoàn toàn bằng Rust** (`eframe`/`egui` + `btleplug` + `tokio`), thay thế mã Python/GTK4 cũ.
- **Đa nền tảng (Windows & Ubuntu)**:
  - **Windows 10/11 (x86_64)**: Tệp thực thi độc lập `.exe` sử dụng WinRT Bluetooth LE nguyên bản của Windows, không cần cài đặt Python hay GTK runtime.
  - **Ubuntu 22.04 / 24.04+ (x86_64)**: Gói cài đặt `.deb` và gói giải nén chạy ngay `.tar.gz` qua BlueZ D-Bus và PipeWire/WirePlumber.
- **Tự đóng gói tài nguyên trong binary**: Phông chữ `Be Vietnam Pro` và danh mục 32 preset EQ (`30 mẫu EarFun + 2 mẫu Fernanda` từ APK 20.1.61) được nhúng trực tiếp vào tệp thực thi.
- **Giữ nguyên 100% giao thức và quy trình an toàn của TW600**:
  - Xác nhận firmware `TW600-…` trước khi mở khóa điều khiển.
  - Kiểm tra giá trị đọc lại sau khi ghi cài đặt.
  - Quy trình tự động xử lý xung đột LDAC / Kết nối đa điểm (2 thiết bị) và tự kết nối lại khi tai nghe khởi động lại, không bao giờ tự gửi lặp lệnh setter.
  - Khi đóng ứng dụng hoặc ngắt bảng điều khiển trên Ubuntu, chỉ hủy đăng ký GATT notify để giữ nguyên kết nối âm thanh A2DP.
  - Tự gửi dừng tiếng tìm tai nghe sau 10 giây và khi đóng cửa sổ.

## Gói phát hành

- `EarFunStudio-Windows-1.3.0-x86_64.exe` — Bản chạy ngay trên Windows 10/11 (64-bit).
- `EarFunStudio-Windows-1.3.0-x86_64.zip` — Gói nén Windows kèm tài liệu.
- `earfun-studio_1.3.0_amd64.deb` — Gói cài đặt Debian/Ubuntu (`sudo apt install ./earfun-studio_1.3.0_amd64.deb`).
- `EarFunStudio-Ubuntu-1.3.0-x86_64.tar.gz` — Gói giải nén chạy trực tiếp trên Ubuntu.
