# Chính sách bảo mật (Security Policy)

## Phiên bản được hỗ trợ

| Phiên bản | Hỗ trợ cập nhật bảo mật |
|---|---|
| Mới nhất (`1.5.1`) | Có |
| `< 1.5.1` | Không |

## Báo cáo lỗ hổng bảo mật

Vui lòng **không** mở issue công khai cho các lỗ hổng bảo mật, lỗi thực thi lệnh hoặc rò rỉ dữ liệu nhạy cảm.

Hãy gửi báo cáo riêng tư thông qua tính năng **GitHub Private Vulnerability Reporting**:

- https://github.com/lnb2x/earfun-studio/security/advisories/new

Khi gửi báo cáo, vui lòng cung cấp:

- Phiên bản EarFun Studio và hệ điều hành đang sử dụng.
- Các bước tái hiện chi tiết hoặc đoạn mã minh họa (PoC).
- Ảnh hưởng dự kiến (ví dụ: thao tác tiến trình con, URI hệ thống, xử lý tệp cấu hình, gói BLE bất thường).
- Nhật ký đã được làm sạch (tuyệt đối không gửi kèm địa chỉ MAC đầy đủ, khóa ghép đôi Bluetooth hay dữ liệu cá nhân).

## Phạm vi bảo vệ của ứng dụng

- Nhật ký chẩn đoán BLE luôn được ẩn địa chỉ MAC qua `protocol::redact_sensitive_identifiers` khi sao chép hoặc xuất tệp.
- Các liên kết ngoài được xác thực danh sách cho phép (`ms-settings:*` và `https://github.com/...`) và gọi trực tiếp không qua shell trung gian.
- Tệp cấu hình người dùng được ghi nguyên tử (quyền `0600` trên Unix) và tự động sao lưu trước khi ghi đè khi phát hiện hỏng.
