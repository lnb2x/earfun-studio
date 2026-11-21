# EarFun Studio 1.5.0 — Nâng cấp Kiến trúc Production, Giao diện Sáng/Tối & Độ tin cậy BLE

## Thay đổi nổi bật

### 1. Độ tin cậy Bluetooth LE & Giao thức TW600
- **Máy trạng thái kết nối chi tiết (`ConnectionPhase`)**: Hiển thị minh bạch từng chặng kết nối: *Chưa kết nối → Đã ghép đôi · Chờ kết nối → Đang quét BLE → Đang mở kênh BLE → Đang mở dịch vụ GATT → Đang xác thực TW600 → Sẵn sàng điều khiển → Đang nối lại (lần N/25)*.
- **Tự phục hồi bộ thu Bluetooth (`Adapter`)**: Tự động kiểm tra trạng thái `PoweredOn` và làm mới bộ thu khi người dùng tắt/bật lại Bluetooth trên hệ điều hành hoặc cắm lại USB Bluetooth, không còn giữ cache chết.
- **Phát hiện ngắt kết nối đột ngột tức thì**: Khi tai nghe cất vào hộp hoặc mất sóng khiến luồng thông báo GATT đóng lại, ứng dụng lập tức giải phóng phiên và chuyển về trạng thái chờ thay vì treo báo *Đã kết nối* tới 14 giây.
- **Sửa lỗi tự hủy âm báo Tìm tai nghe (`stop_find`)**: Khắc phục lỗi tác vụ hẹn giờ 10 giây tự `abort()` chính mình trước khi kịp gửi gói dừng âm tìm `cmd 50 [0]`.
- **Thu thập nguyên tử danh sách thiết bị đa điểm (`cmd 44`)**: Thu thập trọn vẹn toàn bộ các gói thông báo stream của danh sách điện thoại/máy tính đã ghép trước khi trả kết quả, không nhấp nháy xóa trắng danh sách và chặn chính xác việc nối quá 2 thiết bị.
- **Đọc lại xác nhận đa trường & đúng chuẩn APK `20.1.61`**:
  - Kiểm tra đọc lại đầy đủ cả 4 cờ codec (`LDAC, aptX, aptX Adaptive, aptX Lossless`), khóa cảm ứng hai tai (`dis_l, dis_r`) và thao tác chạm (`cmd 11`).
  - Lọc chính xác danh sách thao tác cảm ứng theo loại chạm (chạm 1/2 lần so với chạm 3 lần/nhấn giữ) và chỉ mở tùy chọn *Gọi lại* khi phiên bản firmware TW600 lớn hơn `0.2.7` (`> 27`).

### 2. Hệ thống Thiết kế Giao diện Sáng / Tối / Hệ thống & Tối ưu UX
- **Ba chế độ giao diện (`Sáng`, `Tối`, `Hệ thống`)**: Chuyển đổi tức thì tại trang *Ứng dụng*, nút nhanh góc dưới Sidebar hoặc phím tắt `Alt + T`; lưu bền vững qua các phiên mở.
- **Thanh gạt Equalizer 10 dải & Cân bằng âm thanh mới**: Thanh kéo dọc vẽ tùy biến từ tâm `0 dB`, hỗ trợ kéo chuột trên đường cong Bezier, cuộn chuột tinh chỉnh, phím mũi tên `↑/↓`, nhấn đúp hoặc phím `0` để đưa về `0 dB`; tự lưu bản nháp khi nhả chuột mà không tự ý ghi BLE xuống tai nghe.
- **Bố cục chống tràn & chống đè chữ ở mọi độ phân giải**: Khắc phục triệt để hiện tượng đè chữ lên nút ở kích thước thu gọn `860×620`, sửa thứ tự thẻ trạng thái Sidebar, khung nhật ký toàn chiều rộng và kiểm thử tự động độ phủ glyph tiếng Việt (không còn ký tự ô vuông `□`).

### 3. An toàn Dữ liệu, Bảo mật & Hiệu năng
- **Bảo vệ cấu hình & preset cá nhân**: Ghi tệp nguyên tử có `sync_all`, thử lại khi Windows/OneDrive khóa tệp tạm thời, và tự động sao lưu tệp cấu hình lỗi sang `.corrupt.bak` thay vì ghi đè mất preset EQ của người dùng.
- **Quyền riêng tư & tiến trình con an toàn**: Tự động ẩn địa chỉ MAC (`AA:BB:**:**:EE:FF`) khi sao chép hoặc xuất tệp nhật ký BLE; bỏ gọi `cmd.exe /C start` trên Windows; chạy điều khiển âm lượng PipeWire/WirePlumber bất đồng bộ có giới hạn thời gian trên Ubuntu để không bao giờ làm khựng giao diện.
- **Hiệu năng nhàn rỗi 0 khung hình thừa**: Thay vòng lặp vẽ lại định kỳ `250ms` bằng cơ chế đánh thức sự kiện trực tiếp từ luồng BLE (`0ms` độ trễ) và lưu đệm `OnceLock` cho danh mục 32 preset EQ.

## Tải về

- **Windows 10/11 (x86_64)**: `EarFunStudio-Windows-1.5.0-x86_64.exe` hoặc `.zip`.
- **Ubuntu 22.04 / 24.04+ (x86_64)**: `earfun-studio_1.5.0_amd64.deb` hoặc `EarFunStudio-Ubuntu-1.5.0-x86_64.tar.gz`.
