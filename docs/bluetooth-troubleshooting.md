# Kết nối âm thanh và bảng điều khiển BLE

Air Pro 4 dùng BR/EDR cho âm thanh và Bluetooth LE cho bảng điều khiển. Hệ điều
hành báo “đã kết nối” hoặc nghe nhạc được chưa đảm bảo dịch vụ điều khiển TW600 đã
sẵn sàng. Ứng dụng chỉ mở điều khiển sau khi đọc được firmware `TW600-…`.

## Service discovery timed out

1. Lấy cả hai tai khỏi hộp, bật Bluetooth và ghép Air Pro 4 trong cài đặt hệ điều hành.
2. Đóng EarFun Audio trên điện thoại và các bản EarFun Studio khác có thể đang giữ phiên BLE.
3. Mở EarFun Studio. 1.4 tự tìm/kết nối và thử mở lại kênh BLE khi cần.
4. Nếu lỗi xuất hiện sau khi cài lại Windows hoặc ghép lại trên hệ điều hành kia, kiểm tra khóa ghép đôi dual boot.

## Dual boot Windows và Ubuntu

Cùng một adapter có cùng địa chỉ MAC trên cả hai hệ điều hành, nhưng lần ghép
mới sinh khóa mới. Tai nghe có thể giữ khóa Windows trong khi Ubuntu vẫn giữ khóa
cũ. Đồng bộ riêng LinkKey chỉ sửa phần âm thanh.

Windows lưu khóa tại hive `SYSTEM`, control set hiện hành từ `Select/Current`,
`Services/BTHPORT/Parameters/Keys/<adapter-mac>`. LinkKey BR/EDR là giá trị theo
địa chỉ thiết bị; dữ liệu BLE nằm trong subkey địa chỉ thiết bị.

| Windows | BlueZ trong `/var/lib/bluetooth/<adapter>/<device>/info` |
| --- | --- |
| Giá trị LinkKey 16 byte | `[LinkKey] Key` |
| `LTK` | Khóa dài hạn LE đúng vai trò: `LongTermKey` (central), `PeripheralLongTermKey` (peripheral) |
| `IRK` | `[IdentityResolvingKey] Key` |
| `KeyLength` | `EncSize` |
| `EDIV` | `EDiv` |
| `ERand` | `Rand` |

Sao lưu cấu hình với quyền chỉ root trước khi đồng bộ. Dừng Bluetooth khi ghi và
khởi động lại để BlueZ nạp khóa. Không công khai khóa trong ảnh, issue hay nhật ký.
Không ghi hive Windows nếu chỉ cần nhập khóa Windows vào Ubuntu. Khi chỉ dùng
một hệ điều hành, có thể xóa ghép đôi và ghép lại trên hệ điều hành đó; thao tác này
có thể làm mất kết nối ở hệ điều hành kia.

Nguồn: [mã lưu/nạp khóa BlueZ](https://github.com/bluez/bluez/blob/master/src/adapter.c),
[định dạng khóa dual boot](https://github.com/mgedmin/btdualboot).
