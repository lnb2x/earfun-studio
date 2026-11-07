# EarFun Studio cho Ubuntu

Ứng dụng desktop tiếng Việt cho **EarFun Air Pro 4 / TW600**, chuyển từ mã nguồn
C# của EarFun Studio Windows do người dùng cung cấp. Giao diện GTK 4, Bluetooth LE
qua BlueZ/Bleak, điều chỉnh âm lượng đầu ra tai nghe qua PipeWire/WirePlumber.
Ứng dụng độc lập, không phải ứng dụng chính thức của EarFun.

## Mở trên máy này

Biểu tượng **EarFun Studio** nằm trong menu ứng dụng Ubuntu. Hoặc mở terminal
trong thư mục dự án và chạy:

```sh
./run.sh
```

1. Ghép Air Pro 4 trong **Cài đặt → Bluetooth** của Ubuntu để dùng âm thanh.
2. Lấy hai tai ra khỏi hộp. Đóng EarFun Audio trên điện thoại nếu tranh chấp BLE.
3. Trong EarFun Studio, bấm **Quét**, chọn tai nghe và bấm **Kết nối**.
4. Chờ đọc trạng thái xong. Chỉ sau xác nhận firmware `TW600-…` mới mở điều khiển.

Đóng cửa sổ sẽ thoát ứng dụng, gửi dừng tiếng tìm nếu cần và đóng kênh BLE.
Không có chế độ khay hệ thống. Tự kết nối lại là tùy chọn, mặc định tắt.
Khi ngắt bảng điều khiển, ứng dụng chỉ dừng thông báo GATT và đóng phiên cục bộ;
không gọi ngắt toàn bộ thiết bị BlueZ để giữ kết nối âm thanh A2DP.

## Cài từ mã nguồn

```sh
git clone https://github.com/lnb2x/earfun-studio-ubuntu.git
cd earfun-studio-ubuntu
```

Hoặc giải nén gói do `./packaging/build.sh` tạo vào một thư mục cố định.
Cần Python 3.10+, GTK 4, BlueZ và PipeWire/WirePlumber; hướng tới Ubuntu 24.04+.
Đã chạy giao diện trên Ubuntu 26.04.1; chưa kiểm tra trực tiếp Ubuntu 24.04.

```sh
sudo apt install python3 python3-venv python3-gi gir1.2-gtk-4.0 bluez wireplumber
./install.sh
```

Trình cài tạo `.venv` riêng và biểu tượng ứng dụng cho người dùng hiện tại;
cần Internet để tải các thư viện Python. Giữ nguyên thư mục sau khi cài vì biểu
tượng trỏ tới thư mục đó. Nếu di chuyển thư mục, chạy lại `./install.sh`.
Không chạy ứng dụng bằng sudo. Gỡ biểu tượng bằng `./uninstall.sh`.

## Chức năng

- Tổng quan: pin trái/phải/hộp, firmware, codec, ANC/xuyên âm, chế độ game.
- EQ 10 dải: chỉnh từ −6 đến +6 dB theo bảng mức của thiết bị; 30 mẫu EarFun
  và 2 mẫu Fernanda đối chiếu trực tiếp từ APK 20.1.61 cho TW600, lưu/nạp, đổi
  tên/xóa và nhập/xuất các bản chỉnh riêng.
- Cảm ứng: tám thao tác, khóa riêng từng tai.
- Hai thiết bị: bật/tắt đa điểm, danh sách ghép nối, kết nối/ngắt và mở ghép mới.
- Tai nghe: cảm biến đeo, thông báo, micro, giới hạn âm lượng, cân bằng, codec,
  LE Audio, tìm tai và khôi phục cài đặt gốc.
- Ứng dụng: tự kết nối lại khi đang mở, xuất nhật ký theo yêu cầu.
- Âm lượng hệ thống chỉ chọn Bluetooth sink có địa chỉ trùng thiết bị đã chọn;
  không dùng loa mặc định làm phương án thay thế.

## Xác nhận và giới hạn

Mã giao thức dựa trên `Core/Tw600Protocol.cs`, `Core/BleConnection.cs` và
`Services/DeviceController.cs` của bản Windows được cung cấp, không nạp các tệp
APK/executable của hãng. Chỉ hỗ trợ TW600; không suy diễn lệnh cho mẫu tai khác.

Các thiết lập có lệnh đọc lại sẽ kiểm tra giá trị phản hồi, không báo thành công
chỉ vì đã gửi. Giá trị chưa có dữ liệu hiển thị `—`. Lỗi timeout đóng phiên để
tránh nhầm phản hồi muộn. Lệnh EQ kiểm tra ghi GATT của từng gói, **chưa xác nhận
được đường cong EQ bằng cách đọc lại**. Các luồng đổi codec, đa điểm, cảm biến
và LE Audio đọc lại thiết lập, xử lý kết nối lại khi tai nghe khởi động lại và
báo rõ bước chưa hoàn tất. Lệnh khôi phục cài đặt gốc chỉ báo đã gửi; sau đó
cần ghép lại tai nghe. Các luồng nhiều bước hiện được kiểm thử bằng thiết bị giả,
chưa chủ động đổi các chế độ này trong kiểm tra phần cứng.

Bật codec trên tai nghe không cài thêm codec cho Ubuntu. LDAC và đa điểm không
được bật đồng thời. Không tự thử ghi các thiết lập trên tai nghe khi kiểm thử.
Tìm tai cần xác nhận đã tháo khỏi tai; gửi dừng sau 10 giây và lúc thoát. Nếu
Bluetooth bị mất, không thể bảo đảm thiết bị nhận được lệnh dừng.

Không hỗ trợ cập nhật firmware, bài kiểm tra thính lực, tài khoản hay đám mây.
Đã đọc thành công tai nghe thật trên Ubuntu: firmware
`TW600-3091_20250919_v0.4.1`, pin hai tai 100%, codec aptX Adaptive, ANC, game,
cảm ứng và danh sách đa điểm. Đã kiểm tra cả sáu trang với dữ liệu thật.
Chưa tự thử các lệnh thay đổi thiết lập trên phần cứng; tương thích với
firmware/adapter khác vẫn cần được kiểm chứng.

## Cấu hình và kiểm tra

Cấu hình nằm ở `${XDG_CONFIG_HOME:-~/.config}/earfun-studio/settings.json`.
Nhật ký nằm trong bộ nhớ, tối đa 2.000 dòng; chỉ lưu ra tệp khi chọn xuất.
Nhật ký có thể chứa tên/địa chỉ Bluetooth.

```sh
.venv/bin/python -m unittest discover -s tests -v
./run.sh --smoke-test
./packaging/build.sh
```

`--smoke-test` mở lần lượt sáu trang rồi thoát; không truy cập Bluetooth.
Kiểm thử dùng gói byte tham chiếu, trạng thái lỗi/thiếu, kết nối giả, timeout,
kiểm tra xung đột LDAC/đa điểm và chọn đúng đầu ra PipeWire. Không thay thế
kiểm tra phần cứng. Bằng chứng chạy cục bộ nằm trong `artifacts/`.

## Giao diện 1.1 — đồng bộ Windows

Bố cục, bảng màu và phông chữ đối chiếu trực tiếp với `MainWindow.Layout.cs`,
`Theme.xaml`, `UiPrimitives.cs` và ảnh `nowius-themed-offline` của bản Windows.
Dùng cùng Be Vietnam Pro (kèm giấy phép OFL), biểu tượng vector, nền xanh/tím,
thẻ thiết bị và ba thẻ pin cố định trên mọi trang. EQ có đồ thị 10 điểm cập nhật
cùng thanh trượt; Cảm ứng chia hai cột và tự xếp dọc khi cửa sổ nhỏ.

Các lựa chọn chỉ ghi khi bấm **Áp dụng**/**Lưu**; công tắc và ô chế độ nghe áp
dụng trực tiếp giống bản Windows. Thanh tiêu đề và cài đặt hệ thống dùng Ubuntu;
bản Ubuntu chưa có chức năng thu nhỏ xuống khay hệ thống.

Kiểm tra bố cục và hành vi tương tác không dùng Bluetooth:

```sh
PYTHONPATH=. .venv/bin/python tests/ui_check.py
PYTHONPATH=. .venv/bin/python tests/ui_check.py --compact
```

Các ảnh trong `artifacts/ui-preview/` là giao diện ngoại tuyến, không phải dữ
liệu tai nghe giả lập để quảng cáo. Sau khi cập nhật, đóng cửa sổ ứng dụng cũ
và mở lại EarFun Studio để nạp giao diện mới.

## Sửa kết nối BlueZ 1.1.1

Thiết bị Bluetooth hai chế độ có thể được BlueZ kết nối bằng BR/EDR, trong khi
các đối tượng GATT đã lưu vẫn tồn tại. Trước đây ứng dụng coi đây là BLE đã sẵn
sàng, khiến lần đọc firmware trả `org.bluez.Error.Failed: Not connected`.
Ứng dụng nay phát hiện trường hợp này, yêu cầu mở kênh còn lại, nạp lại dịch vụ
GATT rồi xác nhận lại firmware TW600. Không áp dụng cơ chế thử lại này cho lệnh
thay đổi cài đặt. Bộ chuyển đổi BlueZ được cô lập trong `earfun/bluez.py` và
đối chiếu với phiên bản Bleak 3.0.2 đã ghim.

Tai nghe đã ghép được nạp ngay khi mở ứng dụng. Có thể mở và kết nối thiết bị
cuối bằng `./run.sh --connect-last`. Ngắt bảng điều khiển chỉ đóng phiên điều
khiển của ứng dụng; muốn ngắt cả âm thanh, dùng Cài đặt Bluetooth Ubuntu.

## Bộ EQ và tính năng APK trong 1.2.0

Đã thay ba mẫu tự đặt bằng **30 mẫu EarFun + 2 mẫu Fernanda** của cấu hình TW600
trong EarFun Audio20.1.61. Tên tiếng Việt lấy từ tài nguyên cùng APK. Từng số
trong tệp `assets/eq-presets.json` là chỉ số của bảng gain thiết bị, không phải
dB tuyến tính. Nhãn nguồn APK ở tệp này phục vụ đối chiếu; không đóng gói APK,
mã Java hay bất cứ thư viện ứng dụng điện thoại nào.

Chọn preset chỉ nạp bản nháp, không ghi BLE. Chỉnh một thanh trượt sẽ cập nhật
nhãn bản nháp để không hiển thị “Mặc định” khi đường cong đã khác. **Gửi tới tai
nghe** vẫn là hành động riêng. Giá trị bản nháp được giữ khi đóng/mở ứng dụng.
Có tìm kiếm trong danh sách32 mẫu, lưu/đổi tên/xóa bản riêng, nhập/xuất JSON theo
định dạng `earfun-studio.eq` có kiểm tra mẫu máy/tần số/giới hạn.

Google Fast Pair được bổ sung theo chính setter75/getter76 của TW600, đối nghịch
với LE Audio; đổi trạng thái có thể khởi động lại thiết bị. Không có nút Auracast
giả gắn với lệnh này. Ngôn ngữ thông báo trên APK dùng OTA gói giọng, không tự
gửi getter/setter chung không được firmware xác nhận.

**Bản Ubuntu vẫn chưa thay thế toàn bộ APK**: bài đo/cá nhân hóa thính lực,
firmware/gói giọng OTA, quy trình tham gia Auracast, AI/dịch thuật, tài khoản và
các nội dung đám mây chưa triển khai. Xem `docs/apk-feature-audit.md` để biết
đối chiếu từng phần, bằng chứng và giới hạn.
