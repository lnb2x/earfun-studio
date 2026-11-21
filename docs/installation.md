# Hướng dẫn cài đặt EarFun Studio

Tài liệu hướng dẫn tải, kiểm tra tính toàn vẹn gói phát hành và cài đặt **EarFun Studio** trên **Windows 10/11** và **Ubuntu Linux**.

Tải bản phát hành mới nhất tại: [GitHub Releases](https://github.com/lnb2x/earfun-studio/releases/latest).

---

## Kiểm tra mã băm SHA256 (Khuyến nghị)

Mỗi bản phát hành đều đính kèm tệp `SHA256SUMS.txt`. Hãy đối chiếu trước khi chạy tệp cài đặt:

### Trên Windows (PowerShell)

```powershell
Get-FileHash .\EarFunStudio-Windows-<phiên-bản>-x86_64.exe -Algorithm SHA256
Get-Content .\SHA256SUMS.txt
```

### Trên Ubuntu / Linux

```sh
sha256sum -c SHA256SUMS.txt --ignore-missing
```

---

## Windows 10 / 11 (64-bit)

1. Tải `EarFunStudio-Windows-<phiên-bản>-x86_64.exe` (tệp thực thi đơn lẻ) hoặc giải nén `EarFunStudio-Windows-<phiên-bản>-x86_64.zip`.
2. Nếu Windows Defender SmartScreen hiển thị cảnh báo vì tệp thực thi chưa có chữ ký số thương mại, kiểm tra lại mã SHA256 khớp `SHA256SUMS.txt` trước khi chọn **More info → Run anyway**.
3. Mở **Settings → Bluetooth & devices** và ghép đôi **EarFun Air Pro 4** (cả hai tai lấy ra khỏi hộp sạc).
4. Chạy `EarFunStudio-Windows-<phiên-bản>-x86_64.exe` (hoặc `EarFunStudio.exe` trong gói ZIP). Có thể truyền `--connect-last` để yêu cầu kết nối lại thiết bị gần nhất khi mở cửa sổ.

> [!TIP]
> Kiểm tra phiên bản dòng lệnh (chuyển hướng luồng chuẩn trên GUI subsystem):
> `Start-Process .\EarFunStudio-Windows-<phiên-bản>-x86_64.exe -ArgumentList '--version' -Wait -RedirectStandardOutput ver.txt; Get-Content ver.txt`

---

## Ubuntu Linux (64-bit)

### Cách 1: Cài từ gói `.deb` (khuyến nghị)

```sh
sudo apt update
sudo apt install ./earfun-studio_<phiên-bản>_amd64.deb
earfun-studio
```

Kiểm tra phiên bản:

```sh
earfun-studio --version
```

Gỡ gói `.deb`:

```sh
sudo apt remove earfun-studio
```

### Cách 2: Dùng gói portable `.tar.gz`

```sh
tar -xzf EarFunStudio-Ubuntu-<phiên-bản>-x86_64.tar.gz
cd EarFunStudio-Ubuntu
./install.sh   # Đăng ký biểu tượng menu người dùng
./run.sh       # Chạy ứng dụng
```

Gỡ biểu tượng menu người dùng (giữ nguyên cấu hình cá nhân):

```sh
./uninstall.sh
```

---

## Vị trí lưu cấu hình & sao lưu an toàn

- **Windows**: `%APPDATA%\earfun-studio\settings.json`
- **Ubuntu / Linux**: `${XDG_CONFIG_HOME:-~/.config}/earfun-studio/settings.json` (quyền tệp `0600`)

Nếu tệp cấu hình bị hỏng cú pháp, quá lớn (`> 1 MiB`) hoặc chứa trường sai định dạng, EarFun Studio sao lưu tệp hiện tại sang `settings.json.corrupt.bak` (hoặc `settings.json.corrupt-<thời-gian>.bak` nếu đã có bản sao lưu trước đó) để bảo toàn preset EQ cá nhân của bạn.
