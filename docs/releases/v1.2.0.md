# EarFun Studio Ubuntu 1.2.0

## Đã bổ sung

- 30 mẫu EQ EarFun đúng `presets_10` và 2 mẫu Fernanda riêng TW600.
- Tên tiếng Việt và giá trị từng dải đối chiếu APK20.1.61 mã205.
- Danh sách có tìm kiếm, nạp bản nháp khi chọn; tên mẫu luôn khớp đường cong.
- Đổi tên/xóa bản chỉnh riêng, nhập/xuất JSON có kiểm tra mẫu máy và 10 tần số.
- Giữ bản nháp khi đóng ứng dụng. Không tự ghi EQ khi mở/nạp/sửa/lưu preset.
- Google Fast Pair đối nghịch LE Audio theo cùng lệnh75/76 của TW600.

## Kiểm tra

20 kiểm thử tự động đạt. Kiểm tra GTK đầy đủ ở1440×900 và1060×760 đạt;
xác minh có32mẫu, giá trị Bass Boost3 đúng, đổi tên/xóa bản riêng, không gửi
BLE khi nạp/reset/sửa bản nháp, và không gắn nhãn Mặc định cho đường cong khác.

Đã đọc lại tai nghe thật TW600-3091_20250919_v0.4.1, pin trái90%, phải100%,
LE Audio tắt và Fast Pair bật. Không tự động ghi EQ hoặc thay đổi chế độ Bluetooth trong kiểm thử.
Getter ngôn ngữ113 không trả lời trên firmware này; đã bỏ khỏi chuỗi truy vấn
khi khởi động, tránh gây timeout kết nối. Việc cài gói giọng giữ ở APK chính hãng.

## Phạm vi còn thiếu so với APK

Chưa có adaptive EQ dựa trên bài đo thính lực, firmware/gói giọngOTA, quy trình
quét/tham gia Auracast qua GBSS, AI/dịch thuật, tài khoản hoặc hồ sơ đám mây.
Không xem32preset là thay thế cho các quy trình này. Không tuyên bố bản Ubuntu
đã thay thế100%APK. Xem `apk-feature-audit.md` để tra bằng chứng từng tính năng.
