# Installation Guide

Step-by-step guide to install and use **Auralis** on macOS.

---

## Requirements

- macOS 13 or later
- **Cloud mode**: [Soniox](https://soniox.com) API key (pay-per-use, ~$0.12/hour)
- **Offline mode**: ~5 GB free disk space (for AI models, one-time download)
- **TTS narration** (optional): Edge TTS (free, no API key) or premium providers

---

## Step 1 — Download

Download the latest `.dmg` from: [**Releases — macOS**](https://github.com/nghiavan0610/auralis/releases/latest)

Choose the right file:
- `Auralis_x.x.x_aarch64.dmg` — Apple Silicon (M1/M2/M3/M4)
- `Auralis_x.x.x_x64.dmg` — Intel Mac

---

## Step 2 — Install

1. Open the `.dmg` file
2. Drag **Auralis** into the **Applications** folder
3. Eject the DMG

---

## Step 3 — First Launch

Open Auralis from Applications.

> ⚠️ The app is not signed with an Apple certificate yet. macOS will block it on first launch.

To open it:

1. You'll see **"Auralis is damaged and can't be opened"** or similar
2. Go to **System Settings > Privacy & Security**
3. Scroll down and click **Open Anyway** next to the security warning
4. Confirm by clicking **Open** in the dialog

> You only need to do this once. After the first launch, macOS will remember the app.

---

## Step 4 — Grant Permissions

On first launch, macOS will ask for permissions:

### Microphone (required)

1. Click **OK** when prompted for microphone access
2. If you missed the prompt, go to **System Settings > Privacy & Security > Microphone**
3. Find **Auralis** and toggle it **ON**

### Screen Recording (required for system audio)

1. When prompted, click **Open System Settings**
2. Find **Auralis** in the Screen Recording list
3. Toggle it **ON**
4. Click **Quit & Reopen** when asked

> Screen Recording permission is needed to capture system audio (YouTube, Zoom, meetings, etc.)

---

## Step 5 — Get a Soniox API Key

Soniox provides real-time speech recognition for cloud mode.

1. Go to [console.soniox.com](https://console.soniox.com) → create an account
2. Add billing:
   - Click **Billing** in the left sidebar
   - Add a payment method
   - Add funds ($10 minimum — lasts ~80+ hours at $0.12/hour)
3. Create API key:
   - Click **API Keys** in the left sidebar
   - Click **Create API Key**
   - Copy the key (format: `soniox_...`)

> 💡 Soniox charges ~$0.12/hour of audio processed. $10 ≈ 80+ hours of translation.

---

## Step 6 — Configure the App

1. Click the **gear icon** to open **Settings**
2. Go to the **Translation** tab
3. Paste your **Soniox API key**
4. Choose your languages:
   - **Source** — the language being spoken
   - **Target** — the language to translate to
5. Choose audio source:
   - **Microphone** — your voice
   - **System** — computer audio (YouTube, meetings)
   - **Both** — mixed mic + system audio
6. Click **Save**

### Offline Mode (no API key needed)

1. In Settings, switch to **Offline** mode
2. Click **Setup Offline Mode** — the app downloads AI models automatically (~5 GB)
3. Wait for setup to complete, then click **Save**

> Offline mode works without internet but has higher latency (~3s). Cloud mode is real-time (~150ms).

---

## Step 7 — Enable TTS Narration (Optional)

Want translations **read aloud**? Enable TTS:

1. In Settings, go to the **TTS** tab
2. Toggle **Enable TTS**
3. Choose a provider:

| Provider | Cost | Quality | Setup |
|----------|------|---------|-------|
| **Web Speech** | Free | Basic | Built-in, no config |
| **Edge TTS** | Free | Natural | No API key needed |
| **Google Cloud** | Free tier | High | Google Cloud API key |
| **ElevenLabs** | ~$5/mo+ | Premium | ElevenLabs API key |

4. Choose a voice for your target language
5. Click **Save**

---

## Step 8 — Start Translating!

1. Go back to the main screen
2. Click the **record button** to start
3. Play any audio on your Mac (YouTube, Zoom, podcasts...)
4. Translations appear in real-time!

---

## Troubleshooting

### "Auralis is damaged and can't be opened"
→ Go to **System Settings > Privacy & Security > Open Anyway** (see Step 3).

### No translation text appears
→ Check your Soniox API key is correct in Settings (gear icon). Also verify microphone/screen recording permissions.

### "No microphone found"
→ Mac Mini has no built-in microphone. Connect an external mic (USB, headset, AirPods).

### No system audio captured
→ Make sure **Screen Recording** permission is granted (see Step 4). The app needs this to capture system audio.

### Offline mode won't start
→ Make sure you have ~5 GB free disk space. Try clicking **Setup Offline Mode** again in Settings.

---

## Updating

Auralis includes **auto-update**. When a new version is available:

1. Open Settings → **About** tab
2. You'll see **"Update available"** with the new version
3. Click **Update** — the app downloads and installs automatically
4. Auralis will restart with the new version

No need to download DMG files manually for future updates!
