# Hướng dẫn cài đặt — Windows

Hướng dẫn từng bước cài đặt và sử dụng **Auralis** trên Windows 10/11.

---

## Yêu cầu

- Windows 10 trở lên (x64)
- API key từ [Soniox](https://soniox.com) cho chế độ cloud (trả theo dùng, ~$0.12/giờ)
- **Thuyết minh TTS** (tuỳ chọn): Edge TTS (miễn phí, không cần API key) hoặc nhà cung cấp cao cấp

---

## Bước 1 — Tải xuống

Tải file `.exe` mới nhất tại: [**Releases — Windows**](https://github.com/nghiavan0610/auralis/releases/latest)

Chọn phiên bản phù hợp:
- **x64** — Đa số PC Windows (Intel/AMD)

---

## Bước 2 — Bỏ qua SmartScreen

> ⚠️ Ứng dụng chưa có chữ ký số. Windows SmartScreen sẽ chặn lần chạy đầu tiên.

Khi thấy màn hình **"Windows protected your PC"**:

1. Nhấn **"More info"** (Thêm thông tin)
2. Nhấn **"Run anyway"** (Vẫn chạy)

---

## Bước 3 — Cài đặt

Trình cài đặt sẽ hướng dẫn bạn:

1. Nhấn **Next** để bắt đầu
2. Chọn thư mục cài đặt (để mặc định là được) → nhấn **Next**
3. Đợi cài đặt hoàn tất → nhấn **Next**
4. Tích **"Run Auralis"** → nhấn **Finish**

---

## Bước 4 — Cấu hình API Key và ngôn ngữ

Ứng dụng mở lên. Nhấn **icon bánh răng** để mở **Settings** (Cài đặt).

Cấu hình:

1. **Soniox API Key** — Dán API key (bắt buộc cho chế độ cloud)
2. **Source** — Chọn ngôn ngữ nguồn
3. **Target** — Chọn ngôn ngữ đích (VD: Vietnamese, English...)
4. **Audio Source** — Chọn System Audio (âm thanh máy tính) hoặc Microphone

Nhấn **Save** khi xong.

> 💡 **Lấy API key Soniox ở đâu?**
> 1. Vào [console.soniox.com](https://console.soniox.com) → tạo tài khoản
> 2. Nạp tiền ($10 tối thiểu, dùng rất lâu với ~$0.12/giờ)
> 3. Vào **API Keys** → tạo và copy key

### Chế độ Offline (không cần API key)

1. Trong Settings, chuyển sang chế độ **Offline**
2. Bấm **Setup Offline Mode** — app sẽ tự tải model AI (~5 GB)
3. Đợi hoàn tất, rồi bấm **Save**

---

## Bước 5 — Bật Thuyết Minh TTS (Tuỳ chọn)

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

## Bước 6 — Bắt đầu dịch!

1. Quay lại màn hình chính
2. Bấm **nút thu âm** để bắt đầu
3. Phát bất kỳ audio nào trên PC (YouTube, Zoom, podcast...)
4. Bản dịch xuất hiện real-time!

---

## Khắc phục sự cố

### SmartScreen chặn trình cài đặt
→ Nhấn **"More info"** → **"Run anyway"** (xem Bước 2).

### Không hiện bản dịch
→ Kiểm tra API key Soniox đã đúng trong Settings chưa.

### Không bắt được âm thanh hệ thống
→ Đảm bảo đang phát audio trên PC. Một số ứng dụng dùng exclusive audio mode — thử nguồn audio khác.

### Ứng dụng không mở
→ Đảm bảo WebView2 Runtime đã cài. Windows 10/11 thường có sẵn, nhưng phiên bản cũ cần cài từ [Microsoft](https://developer.microsoft.com/en-us/microsoft-edge/webview2/).

### Chế độ Offline không khởi động
→ Đảm bảo có ~5 GB dung lượng trống. Thử bấm **Setup Offline Mode** lại trong Settings.

---

## Cập nhật

Auralis có tính năng **tự động cập nhật**. Khi có bản mới:

1. Mở Settings → tab **About**
2. Bạn sẽ thấy **"Update available"** với phiên bản mới
3. Bấm **Update** — app sẽ tự tải và cài đặt
4. Auralis tự khởi động lại với bản mới

Không cần tải installer thủ công cho các bản cập nhật sau!
