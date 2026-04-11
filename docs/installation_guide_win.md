# Installation Guide — Windows

Step-by-step guide to install and use **Auralis** on Windows 10/11.

---

## Requirements

- Windows 10 or later (x64)
- [Soniox](https://soniox.com) API key for cloud mode (pay-per-use, ~$0.12/hour)
- **TTS narration** (optional): Edge TTS (free, no API key) or premium providers

---

## Step 1 — Download

Download the latest `.exe` installer from: [**Releases — Windows**](https://github.com/nghiavan0610/auralis/releases/latest)

Choose the right version:
- **x64** — Most Windows PCs (Intel/AMD)

---

## Step 2 — Bypass SmartScreen

> ⚠️ The app is not signed with a certificate. Windows SmartScreen will block it on first run.

When you see the **"Windows protected your PC"** screen:

1. Click **"More info"**
2. Click **"Run anyway"**

---

## Step 3 — Install

The setup wizard will guide you:

1. Click **Next** to start
2. Choose install location (default is fine) → click **Next**
3. Wait for installation to complete → click **Next**
4. Check **"Run Auralis"** → click **Finish**

---

## Step 4 — Configure API Key & Languages

The app opens. Click the **gear icon** to open **Settings**.

Configure:

1. **Soniox API Key** — Paste your API key (required for cloud mode)
2. **Source** — Choose the source language
3. **Target** — Choose the target language (e.g., Vietnamese, English...)
4. **Audio Source** — Choose System Audio (computer sound) or Microphone

Click **Save** when done.

> 💡 **Where to get a Soniox API key?**
> 1. Go to [console.soniox.com](https://console.soniox.com) → create an account
> 2. Add funds ($10 minimum, lasts ~80+ hours at ~$0.12/hour)
> 3. Go to **API Keys** → create and copy your key

### Offline Mode (no API key needed)

1. In Settings, switch to **Offline** mode
2. Click **Setup Offline Mode** — the app downloads AI models automatically (~5 GB)
3. Wait for setup to complete, then click **Save**

---

## Step 5 — Enable TTS Narration (Optional)

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

## Step 6 — Start Translating!

1. Go back to the main screen
2. Click the **record button** to start
3. Play any audio on your PC (YouTube, Zoom, podcasts...)
4. Translations appear in real-time!

---

## Troubleshooting

### SmartScreen blocks the installer
→ Click **"More info"** → **"Run anyway"** (see Step 2).

### No translation text appears
→ Check your Soniox API key is correct in Settings (gear icon).

### No system audio captured
→ Make sure audio is playing on your PC. Some apps use exclusive audio mode — try a different source.

### App doesn't start
→ Make sure WebView2 Runtime is installed. It comes with Windows 10/11, but on older versions you may need to install it from [Microsoft](https://developer.microsoft.com/en-us/microsoft-edge/webview2/).

### Offline mode won't start
→ Make sure you have ~5 GB free disk space. Try clicking **Setup Offline Mode** again in Settings.

---

## Updating

Auralis includes **auto-update**. When a new version is available:

1. Open Settings → **About** tab
2. You'll see **"Update available"** with the new version
3. Click **Update** — the app downloads and installs automatically
4. Auralis will restart with the new version

No need to download installers manually for future updates!
