# Hướng Dẫn Cài Đặt Auralis

Hướng dẫn từng bước cài đặt và sử dụng **Auralis** trên macOS.

---

## Yêu cầu

- macOS 13 trở lên
- **Chế độ Cloud**: API key của [Soniox](https://soniox.com) (trả theo dùng, ~$0.12/giờ)
- **Chế độ Offline**: ~5 GB dung lượng ổ cứng (cho mô hình AI, tải một lần)
- **Thuyết minh TTS** (tuỳ chọn): Edge TTS (miễn phí, không cần API key) hoặc nhà cung cấp cao cấp

---

## Bước 1 — Tải về

Tải file `.dmg` mới nhất tại: [**Releases — macOS**](https://github.com/nghiavan0610/auralis/releases/latest)

Chọn đúng file:
- `Auralis_x.x.x_aarch64.dmg` — Apple Silicon (chip M1/M2/M3/M4)
- `Auralis_x.x.x_x64.dmg` — Intel Mac

---

## Bước 2 — Cài đặt

1. Mở file `.dmg` vừa tải
2. Kéo **Auralis** vào thư mục **Applications**
3. Eject DMG

---

## Bước 3 — Mở lần đầu

Mở Auralis từ Applications.

> ⚠️ App chưa có chữ ký số Apple. macOS sẽ chặn lần mở đầu tiên.

Cách mở:

1. Bạn sẽ thấy thông báo **"Auralis is damaged and can't be opened"** hoặc tương tự
2. Vào **Cài đặt hệ thống > Quyền riêng tư & Bảo mật**
3. Kéo xuống và bấm **Open Anyway** bên cạnh cảnh báo bảo mật
4. Xác nhận bằng cách bấm **Open** trong hộp thoại

> Bạn chỉ cần làm việc này một lần. Sau lần đầu, macOS sẽ nhớ app.

---

## Bước 4 — Cấp quyền

Lần đầu mở app, macOS sẽ hỏi cấp quyền:

### Microphone (bắt buộc)

1. Bấm **OK** khi được hỏi quyền truy cập micro
2. Nếu bỏ lỡ, vào **Cài đặt hệ thống > Quyền riêng tư & Bảo mật > Microphone**
3. Tìm **Auralis** và bật **ON**

### Screen Recording (bắt buộc cho âm thanh hệ thống)

1. Khi được hỏi, bấm **Open System Settings**
2. Tìm **Auralis** trong danh sách Screen Recording
3. Bật **ON**
4. Bấm **Quit & Reopen** khi được yêu cầu

> Quyền Screen Recording cần thiết để bắt âm thanh hệ thống (YouTube, Zoom, cuộc họp, v.v.)

---

## Bước 5 — Lấy API Key Soniox

Soniox cung cấp nhận diện giọng nói real-time cho chế độ cloud.

1. Vào [console.soniox.com](https://console.soniox.com) → tạo tài khoản
2. Nạp tiền:
   - Click **Billing** ở thanh bên trái
   - Thêm phương thức thanh toán
   - Nạp tiền ($10 tối thiểu — dùng được ~80+ giờ với ~$0.12/giờ)
3. Tạo API key:
   - Click **API Keys** ở thanh bên trái
   - Click **Create API Key**
   - Copy key (dạng `soniox_...`)

> 💡 Soniox tính ~$0.12/giờ audio. $10 ≈ 80+ giờ dịch.

---

## Bước 6 — Cấu hình App

1. Bấm **icon bánh răng** để mở **Settings** (Cài đặt)
2. Vào tab **Translation**
3. Dán **Soniox API key**
4. Chọn ngôn ngữ:
   - **Source** — ngôn ngữ đang nói
   - **Target** — ngôn ngữ cần dịch sang
5. Chọn nguồn audio:
   - **Microphone** — giọng nói của bạn
   - **System** — âm thanh máy tính (YouTube, cuộc họp)
   - **Both** — mix mic + âm thanh hệ thống
6. Bấm **Save**

### Chế độ Offline (không cần API key)

1. Trong Settings, chuyển sang chế độ **Offline**
2. Bấm **Setup Offline Mode** — app sẽ tự tải model AI (~5 GB)
3. Đợi hoàn tất, rồi bấm **Save**

> Chế độ Offline hoạt động không cần internet nhưng độ trễ cao hơn (~3 giây). Chế độ Cloud real-time (~150ms).

---

## Bước 7 — Bật Thuyết Minh TTS (Tuỳ chọn)

Muốn bản dịch được **đọc thành lời**? Bật TTS:

1. Trong Settings, vào tab **TTS**
2. Bật **Enable TTS**
3. Chọn nhà cung cấp:

| Nhà cung cấp | Chi phí | Chất lượng | Cài đặt |
|---------------|---------|------------|---------|
| **Web Speech** | Miễn phí | Cơ bản | Tích hợp sẵn |
| **Edge TTS** | Miễn phí | Tự nhiên | Không cần API key |
| **Google Cloud** | Có gói miễn phí | Cao | Cần Google Cloud API key |
| **ElevenLabs** | ~$5/tháng trở lên | Cao cấp | Cần ElevenLabs API key |

4. Chọn giọng nói cho ngôn ngữ đích
5. Bấm **Save**

---

## Bước 8 — Bắt đầu dịch!

1. Quay lại màn hình chính
2. Bấm **nút thu âm** để bắt đầu
3. Phát bất kỳ audio nào trên máy (YouTube, Zoom, podcast...)
4. Bản dịch xuất hiện real-time!

---

## Xử lý sự cố

### "Auralis is damaged and can't be opened"
→ Vào **Cài đặt hệ thống > Quyền riêng tư & Bảo mật > Open Anyway** (xem Bước 3).

### Không có bản dịch / không hiện text
→ Kiểm tra API key Soniox đã đúng trong Settings chưa. Kiểm tra quyền micro/screen recording.

### Lỗi "No microphone found"
→ Mac Mini không có mic tích hợp. Cần kết nối mic ngoài (USB, headset, AirPods).

### Không bắt được âm thanh hệ thống
→ Đảm bảo đã cấp quyền **Screen Recording** (xem Bước 4). App cần quyền này để bắt âm thanh hệ thống.

### Chế độ Offline không khởi động
→ Đảm bảo có ~5 GB dung lượng trống. Thử bấm **Setup Offline Mode** lại trong Settings.

---

## Cập nhật

Auralis có tính năng **tự động cập nhật**. Khi có bản mới:

1. Mở Settings → tab **About**
2. Bạn sẽ thấy **"Update available"** với phiên bản mới
3. Bấm **Update** — app sẽ tự tải và cài đặt
4. Auralis tự khởi động lại với bản mới

Không cần tải DMG thủ công cho các bản cập nhật sau!
