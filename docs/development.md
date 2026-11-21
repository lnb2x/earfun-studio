# Hướng dẫn phát triển & đóng gói

## Yêu cầu môi trường

- **Rust toolchain**: `1.99.1` (stable), kèm thành phần `rustfmt` và `clippy`.
- **Windows**: MSVC build tools (Visual Studio Build Tools / Desktop development with C++).
- **Ubuntu / Linux**:

```sh
sudo apt update
sudo apt install -y pkg-config libdbus-1-dev libglib2.0-dev libgtk-3-dev libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libx11-dev libgl1-mesa-dev
```

## Chạy cục bộ & chẩn đoán chỉ đọc

```sh
# Khởi chạy ứng dụng ở chế độ phát triển
cargo run --locked
# Các cờ dòng lệnh hỗ trợ: --version / -V, --help / -h, --connect-last, --smoke-test

# Kiểm tra kết nối phần cứng ở chế độ chỉ đọc (không ghi cài đặt xuống tai nghe)
cargo run --locked --example diagnose
# Hoặc chỉ định địa chỉ MAC đã biết:
cargo run --locked --example diagnose -- AA:BB:CC:DD:EE:FF
```

## Cổng kiểm tra chất lượng (Bắt buộc trước khi PR)

Theo [`AGENTS.md`](../AGENTS.md):

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
EARFUN_HEADLESS=1 cargo run --locked -- --smoke-test
```

Lệnh `--smoke-test` chạy layout pass cho cả 6 trang trên cả giao diện **Sáng** và **Tối** tại `860×620` và `1060×760`, xuất ra 12 dòng `UI OK: …`.

## Chụp ảnh giao diện ngoại tuyến (dữ liệu minh họa)

[`examples/preview.rs`](../examples/preview.rs) tự cô lập thư mục cấu hình tạm (`earfun-preview-cfg`) để không làm lộ preset cá nhân thật:

```sh
cargo build --locked --features ui-capture --example preview
EFRAME_SCREENSHOT_TO="$PWD/preview-overview.png" ./target/debug/examples/preview 1060 760 0 --connected
# Thêm --dark cho giao diện Tối; đổi chỉ số trang 0..5 cho 6 trang
```

## Đóng gói cục bộ

- **Ubuntu** (`.tar.gz` và `.deb` trong `dist/`):
  ```sh
  ./packaging/build.sh
  ```
- **Windows** (`.exe` và `.zip` trong `dist/`):
  ```powershell
  pwsh ./packaging/build-windows.ps1
  ```

## Quy trình phát hành (Release)

1. Cập nhật phiên bản trong [`Cargo.toml`](../Cargo.toml) và chạy `cargo build` để làm mới `Cargo.lock`.
2. Viết ghi chú phát hành tại `docs/releases/v<X.Y.Z>.md` (nhắc rõ số phiên bản `<X.Y.Z>` trong nội dung).
3. Mở PR về nhánh mặc định (`codex/ubuntu-app`) và đảm bảo toàn bộ workflow CI xanh.
4. Sau khi hợp nhất (và đồng bộ `main`), tạo annotated tag khớp chính xác phiên bản và đẩy lên GitHub:
   ```sh
   git tag -a v<X.Y.Z> -m "EarFun Studio v<X.Y.Z>"
   git push origin v<X.Y.Z>
   ```
5. Workflow `.github/workflows/release.yml` sẽ chạy `preflight`, biên dịch & kiểm định gói Windows/Ubuntu, tạo `SHA256SUMS.txt` và xuất bản GitHub Release tự động.
