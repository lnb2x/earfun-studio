# EarFun Studio 1.4.0 — Tự kết nối và sửa kết nối BLE

## Thay đổi

- Windows và Ubuntu tự tìm/kết nối Air Pro 4 ngay khi mở; không cần quét rồi bấm kết nối. Ghi nhớ tai nghe sau khi xác nhận firmware TW600 và thử lại khi tai nghe ra khỏi hộp.
- Bật tự kết nối khi nâng cấp từ mặc định cũ của 1.3; giữ EQ, preset và địa chỉ thiết bị. Có thể tắt trong Ứng dụng. Khi chủ động ngắt kết nối, không tự nối lại trong phiên đó.
- Giữ bộ thu Bluetooth suốt phiên, sửa việc mất danh sách thiết bị/phiên WinRT sau khi quét trên Windows. Bắt đầu quét BLE tự động khi hệ điều hành chưa cung cấp thiết bị.
- Mở kênh điều khiển LE ngay cả khi kết nối âm thanh BR/EDR đã hoạt động. Không dừng ngay ở timeout ServicesResolved ngắn của BlueZ; khám phá lại dịch vụ TW600 và dọn phiên thông báo cũ trước khi thử lại.
- Giới hạn thời gian đăng ký GATT notify; trả trạng thái về chưa kết nối khi lỗi. Đóng bảng điều khiển Ubuntu không gọi ngắt kết nối âm thanh toàn thiết bị.
- EQ chỉ báo đã gửi sau khi ghi GATT thành công, không báo thành công trước tác vụ nền; bỏ hiển thị âm lượng Windows giả, dùng lối tắt âm lượng hệ thống.
- Sửa nút kết nối tràn thẻ thiết bị và thanh âm lượng đè nút Đọc; thanh công cụ xuống hàng, các nhóm nút tự xuống dòng; toàn bộ nội dung có thể cuộn ở cửa sổ nhỏ. Phân biệt ID vùng cuộn nội dung và nhật ký.
- Bản dựng lấy phiên bản trực tiếp từ Cargo; chỉ phát hành khi đẩy tag đúng phiên bản, kèm SHA256SUMS.txt. Đẩy nhánh không ghi đè tài sản của bản phát hành cũ.

## Lưu ý dual boot

Nếu vừa cài lại Windows/ghép lại tai nghe, khóa lưu trong Ubuntu có thể không còn
khớp. **Cần đồng bộ cả LinkKey cho âm thanh và khóa BLE LTK/IRK cùng EDiv/Rand cho
điều khiển**, hoặc ghép lại trên hệ điều hành đang dùng. Tự kết nối không thể bỏ qua
xác thực Bluetooth và không tự thay khóa ghép đôi. Các thay đổi khóa trên máy phát
triển là sửa cấu hình máy, không phải tính năng tự đồng bộ của ứng dụng.

## Tải về

- Windows 10/11 x64: `EarFunStudio-Windows-1.4.0-x86_64.exe` hoặc `.zip`.
- Ubuntu x64: `earfun-studio_1.4.0_amd64.deb` hoặc `EarFunStudio-Ubuntu-1.4.0-x86_64.tar.gz`.
- Ghép tai nghe trong cài đặt hệ điều hành, lấy tai nghe ra khỏi hộp, rồi mở ứng dụng.
