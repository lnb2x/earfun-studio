# Đối chiếu EarFun Audio APK với EarFun Studio Ubuntu

Ngày kiểm tra: 31/10/2026. Mẫu thiết bị: **EarFun Air Pro 4 / TW600**.

## Nguồn và phạm vi

APK cục bộ do chủ máy cung cấp: EarFun Audio `20.1.61`, versionCode `205`,
package `com.corelink.earfun`. SHA-256:
`1a0f2793dfc298d621786bab8cb990d45c94d780de472cec4b3319a41c4f21f1`.
Bản APK nằm trong `.tools/earfun-official-base.apk` của thư mục Windows đã cung
cấp; không được đưa vào bộ cài Ubuntu.

Đối chiếu bằng JADX/REA và cây mã dịch ngược cục bộ
`.tools/earfun-official-source/sources/`. Đây là quan sát tĩnh từ gói phát hành,
không phải mã nguồn gốc của hãng và không phải xác nhận đã chạy các tính năng.
Không thao tác trên điện thoại hoặc ghi cài đặt tai nghe trong lượt kiểm tra này.
Không dùng tính năng chung của các mẫu EarFun khác để suy ra TW600 hỗ trợ.

Định danh APK: Evidence
`ev_40982c64ff763765cb21b843b9b4c916eb2862d0412a02d13f8ff13857abb405`.
Các lớp trọng tâm:

- `com.corelink.earfun.device.base.DeviceType`: mục TW600 tạo **TW600Config**,
  **TW600ProtocolParser**, **Qualcomm10Equailzer**, **QualcommOtaProtocol**.
- `com.corelink.earfun.device.config.headset.TW600Config`: cấu hình tính năng và
  danh mục EQ riêng TW600. Lớp này thiếu trong cây Java đã xuất trước đây, nhưng
  được đọc trực tiếp từ APK qua REA, không suy ra từ TW500/TW700.
- `com.corelink.earfun.device.config.base.BaseConfig`: danh sách preset dùng chung.
- `com.corelink.earfun.device.protocol.headset.QualcommProtocolParser` và
  `TW600ProtocolParser`: mã lệnh và ánh xạ riêng thiết bị.

Inventory TW600Config: Evidence
`ev_44fa0f8d399c939880ae150525a54b19dd7e740f22fb34159db61687801197d8`.
Một số phương thức chỉ có bản smali đầy đủ vì Java decompiler không phục hồi
được phần thân; kết luận dưới đây dùng phần thân smali đó, không chỉ tên hàm.

## Equalizer: khác biệt xác định được

`TW600Config.getEquailerList` chọn **`getPresets_10()`**, không chọn
`presets_10_new`. Danh mục gồm bốn nhóm: preset, bản chỉnh riêng, adaptive EQ và
nhóm chuyên gia `FERNANDA`.

- Bộ `presets_10` có **30 preset**. Tên và giá trị phải lấy từ đúng cấu hình và
  tài nguyên APK, không thay bằng ba mẫu tự đặt của bản Ubuntu trước.
- Hai mẫu riêng trong nhóm FERNANDA được TW600Config khai báo trực tiếp:

| ID | Tên tài nguyên | 10 giá trị chỉ số gain |
|---|---|---|
| 201 | `tw600_warm` | `[-2, 2, 0, 1, 2, 3, -1, -3, -4, 0]` |
| 202 | `tw700_hong_2` | `[-7, -3, -1, -2, 0, 2, 2, -1, -2, 1]` |

Các số là chỉ số của bảng gain thiết bị, **không phải số dB trực tiếp**.
`Qualcomm10Equailzer` dùng bảng mức không tuyến tính; ví dụ chỉ số −2 tương ứng
−1 dB và chỉ số +3 tương ứng +1,5 dB trong bảng hiện được dùng.

Evidence định tuyến danh mục và hai mẫu chuyên gia:
`ev_01fdd0dcc8430a2738448921a4747c9b4d23f670e6655dc63861c3e27bbf2c52`.

Nhóm adaptive dùng dữ liệu kết quả được truyền vào `getEquailerList` và
`getAdaptereq()`. Sự hiện diện của nhóm không cung cấp thuật toán đo/cá nhân hóa
thính lực hoàn chỉnh cho Ubuntu. Không được thay bằng một preset tùy ý rồi gọi
đó là adaptive EQ. Bản chỉnh riêng cần lưu/nạp, đổi tên/xóa và nhận diện bản nháp
đã sửa; tên preset đang chọn không phải bằng chứng EQ đã được gửi hay đọc lại.
Giao thức hiện có vẫn chưa xác nhận đọc lại đường cong EQ từ tai nghe.

## Các mục trong cấu hình TW600

`getDetailFunction` khai báo pin, tên thiết bị, chống ồn, game, EQ, nhóm điều
khiển và nhóm tùy chọn khác. `getCombineModeFunctionList` nhóm nhận biết đeo tai
và tùy chỉnh cảm ứng. `getOtherFunction` khai báo đa điểm, chất lượng Bluetooth,
micro, cài đặt giọng thông báo, tìm tai, phòng thử nghiệm, AI và dịch thuật.

Evidence:

- Detail: `ev_875ec032cb9dbfde5f9c5d179715acdc75f888cf08bb6e0f853adbcc307e4b09`.
- Combine: `ev_83df20c1ad04baf845f8c4fa2332f496b4124049985dcdff03abb7c2bfd04433`.
- Other: `ev_d21f41994cb3e352aed79e8636386ddc959fd6e44c6325ae67c64f89cb888ab6`.

Các điều khiển Ubuntu đã có ANC/xuyên âm, năm kiểu ANC, hai kiểu xuyên âm,
game, cảm biến đeo, tám thao tác bình thường, khóa trái/phải, đa điểm, codec,
micro và mức thông báo. Không nên gọi bản Ubuntu là bản thay thế 100% APK khi
các nhóm ở các phần tiếp theo còn thiếu.

## Cảm ứng: không suy diễn thao tác cuộc gọi

`TW600Config.getKeyFunction` chỉ cung cấp bốn loại thao tác bình thường cho mỗi
tai: chạm một, hai, ba lần và nhấn giữ. Do đó tám thao tác Ubuntu không bị thiếu
một nhóm cấu hình cuộc gọi được TW600Config công bố.

Parser Qualcomm dùng chung thực sự hiểu thêm mã thao tác cuộc gọi, nhưng điều
đó không đủ để cho rằng TW600 có giao diện cấu hình chúng trong APK. Không thêm
các lựa chọn đó chỉ dựa vào enum dùng chung.

Tùy chọn normal được APK giới hạn theo loại thao tác: single/double gồm âm lượng,
bài trước/sau, phát/tạm dừng, trợ lý, đổi chế độ nghe và không gán. Triple/long có
các lựa chọn tương tự cùng game/custom ANC; gọi lại chỉ được thêm khi số phiên
bản cấu hình lớn hơn 27. Giá trị phiên bản được tính từ chuỗi `x.y.z` thành
`100*x + 10*y + z`; cần dùng phiên bản đã tách đúng, không đưa nguyên chuỗi firmware
`TW600-...` vào hàm này.

Evidence `getKeyFunction`:
`ev_144fff4182f4ec3bb0f265aa585679d1d522595fa6ae43efc9450b82762006b9`.
Evidence `getKeyCustomFunction`:
`ev_3da5a6bbef398294b81fe3f3392061c8521509185dd11f467c5f829eecaeec51`.

## Giọng thông báo và ngôn ngữ

Parser dùng chung có getter **113**, setter **112** với các mã
`0=English`, `1=Chinese`, `2=German`, `3=Japanese`.
`QualcommProtocolParser.changeVoice`, `setData` và `WhenMappings` xác lập ánh xạ
này. Evidence phần setter:
`ev_fb11d1caf7b295528dedec2962d93e9c27d5b603811c1a02209de094df109627`.

Tuy nhiên luồng APK **riêng TW600** không được xác nhận là chỉ đổi một byte:
TW600Config không override `getInnerVoiceList`; BaseConfig trả `null`.
Trong `DeviceVoiceWizardFragment.showSelect$lambda$13`, nhánh danh sách giọng
có sẵn mới gọi trực tiếp `changeVoice`; nhánh `null` kiểm tra pin hai tai rồi
chạy **`DeviceVoiceWizardViewModel.startOta`** để cài gói giọng.

Vì vậy chỉ nên đọc và hiển thị ngôn ngữ nếu thiết bị xác nhận getter 113.
Không quảng bá một dropdown bốn ngôn ngữ dùng setter 112 là tương đương luồng
cài giọng APK của TW600. Cài/cập nhật gói giọng vẫn dùng EarFun Audio đến khi có
quy trình gói, kiểm tra và khôi phục riêng được xác minh.

`TW600Config.isCanJumpVoiceTip` trả true khi số phiên bản lớn hơn 40; đây là
điều kiện hiển thị lối tắt gợi ý trong APK, không chứng minh điều kiện của setter
112. Evidence:
`ev_178bcbebae491a6b7e1af9f18cbef6fe4c40f1d77db1b6601974a34a96e2fd81`.

## Google Fast Pair và LE Audio

Đây là hai trạng thái đối nghịch của **cùng một thiết lập TW600**, không phải
hai tính năng có thể cùng bật:

| Thao tác | Lệnh GAIA EarFun | Diễn giải |
|---|---|---|
| Đọc | getter 76 | `0`: Fast Pair bật / LE Audio tắt; `1`: LE Audio bật / Fast Pair tắt |
| Bật Fast Pair | setter 75, payload `0` | Tắt LE Audio |
| Tắt Fast Pair | setter 75, payload `1` | Bật LE Audio |
| Bật LE Audio | setter 75, payload `1` | Tắt Fast Pair |
| Tắt LE Audio | setter 75, payload `0` | Bật Fast Pair |

Nguồn: `TW600ProtocolParser.setGoogleFastPair`, `setLeAudio`, cùng parser phản
hồi 76 trong QualcommProtocolParser. Chỉ nhận trạng thái byte 0 hoặc 1; dữ liệu
khác phải hiện chưa biết. APK đặt thời gian chờ khởi động lại 20 giây trong hai
view holder. UI Ubuntu phải giải thích sự liên kết này, yêu cầu xác nhận và đọc
lại sau khi nối lại; không tự thử bật/tắt trong kiểm tra phần cứng.

Evidence setter Fast Pair:
`ev_2e5b5015ebe53cb705cdc4f58ca8a7a14dbee92b7ed98f70de8661dc62f76508`.
Evidence setter LE Audio:
`ev_d30210483b57a8179146b2f70cb68f026b09f85e4650c6bc86a1eed790ab91f5`.

`TW600Config.getLabFunctionList` công bố GOOGLE_FAST_PAIR, LE_AUDIO, AUARACAST.
Base HeadsetConfig trả map enable rỗng và TW600 không override hàm này trong
inventory; không có căn cứ đặt một ngưỡng API tùy ý để bật nút. Việc getter 76
trả dữ liệu hợp lệ là kiểm tra khả năng tối thiểu của bản Ubuntu.
Evidence menu lab:
`ev_3bf29e91eb9450ef1b32ef1b25258c6e8f68f83fdf863d1e0223dcf27c250506`.

## Auracast

Đây không phải một công tắc GAIA 75/76 bổ sung. View holder APK yêu cầu LE Audio
đang bật rồi mở trang quản lý nguồn phát. Thư viện Qualcomm Auracast dùng dịch vụ
GATT **GBSS** riêng, được khai báo trong
`com.qualcomm.qti.leaudio.auracast.data.Services`:

- Service: `baebd52b-d02d-4c9c-81b5-a6c0f7428eed`.
- Control point: `d3784627-ec77-4043-bcb5-b94267f760fe`.
- Receiver state: `b3bb9061-0e62-48d5-95b8-4c9c1c742837`.
- Report: `aee4ce2a-6fe0-4da3-b34c-5fcb8e592109`.

`AuracastCommandHandlerImpl` có quét/dừng quét, thêm/sửa/xóa nguồn và nhập mã
nguồn; giao tiếp qua control point và các phản hồi/notification. Ví dụ dừng quét
là payload một byte `01`; bắt đầu quét đóng gói thời hạn và bộ lọc. Chưa kiểm tra
runtime GBSS trên tai nghe, chưa đối chiếu đầy đủ parser báo cáo, đồng bộ nguồn
và hành vi LE Audio trên Ubuntu. Vì thế chưa thể gọi Auracast là đã triển khai,
và không được dùng một công tắc giả để che phần thiếu. Các UUID ở đây là dữ kiện
phân tích tĩnh, không phải xác nhận thiết bị hiện đang cung cấp dịch vụ đó.

## Các phần còn lại và giới hạn

| Nhóm | Trạng thái đánh giá |
|---|---|
| 30 preset + 2 chuyên gia | Có đủ dữ liệu tĩnh để tái tạo danh mục và giá trị; root triển khai/catalog kiểm tra riêng. |
| Adaptive/personalized EQ | Có nhóm và dữ liệu đầu vào, chưa phục hồi quy trình đo/cá nhân hóa đầy đủ. |
| Firmware OTA | TW600 dùng QualcommOtaProtocol; chưa triển khai bộ cập nhật/khôi phục Ubuntu. |
| Gói giọng thông báo | Luồng APK dùng OTA khi không có danh sách giọng tích hợp; không thay thế bằng setter chung. |
| AI, dịch thuật | Có trong menu TW600 APK; chưa đối chiếu đủ luồng tài khoản, API/dịch vụ và âm thanh để triển khai Ubuntu. |
| Cloud/account/content | Chưa được thay thế; không tự truy cập hay sao chép tài khoản/dữ liệu điện thoại. |
| Auto-off | TW600Config có danh sách 5/10/30/60/120 phút/không tắt, nhưng menu TW600 đã đọc không công bố mục này; không tự bật một chức năng chỉ vì có enum/parser dùng chung. |
| Cấu hình ANC/xuyên âm | APK công bố đúng 5 kiểu ANC và 2 kiểu xuyên âm; đã có tương ứng trong Ubuntu. Một số option có điều kiện phiên bản. |

Evidence danh sách auto-off:
`ev_f0195d538e77aeff90734da96887f5e543687564abb744a873847d1bfc53d918`.
Evidence danh sách ANC:
`ev_67a4068869009e66f1eda829c40c0d55c376def9904a0caafdd89298d17127bc`.
Evidence danh sách xuyên âm:
`ev_be0f9b0177d2bcc095170eb077a148242692439dce8c8ec87f20ff51da616a0e`.

Báo cáo này đánh dấu rõ những mục có bằng chứng và các phần còn thiếu; không
khẳng định đã thay thế toàn bộ EarFun Audio hoặc đã kiểm tra tất cả chức năng
trên phần cứng.


## Kiểm tra đọc trên phần cứng của chủ máy

Trong lượt bổ sung này, `000A0371` (getter113) không trả lời trên firmware
`TW600-3091_20250919_v0.4.1`, dẫn đến timeout4giây. Đây chỉ là quan sát trên
firmware này, không phải kết luận cho mọi bản TW600. Đã loại lệnh này khỏi
chuỗi đọc khi kết nối để không gây lỗi cho chức năng đang hoạt động. Giao diện
không gửi setter112 hoặc tự nhận biết ngôn ngữ; hướng dẫn dùng APK để cài gói
ngôn ngữ. Log gốc cục bộ: `artifacts/apk-parity-readonly.log`.
Getter76 trả0 trên thiết bị thử, tương ứng LE Audio tắt/Fast Pair bật theo
parser APK; không gửi setter75 trong kiểm tra.
