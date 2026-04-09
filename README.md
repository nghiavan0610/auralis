<div align="center">
  <img src="assets/banner.png" alt="Auralis" width="640" />
</div>

# Auralis

Compact overlay app for real-time speech translation with dual-mode architecture: **cloud** (Soniox, ~150ms) and **offline** (MLX Whisper + Opus-MT, ~3s).

Features a 600x400 borderless glassmorphism window with custom control bar, system audio capture (YouTube, meetings), and two-way translation.

## Architecture

```
+----------------------------------------------------------+
|            Svelte 5 Frontend (Overlay UI)                 |
|  ControlBar | Transcript | SettingsView                   |
+----------------------------------------------------------+
|  Rust/Tauri Backend                                       |
|  Audio Capture (cpal + ScreenCaptureKit)                  |
|    - Microphone, System audio, or Both (mixed)           |
|  Settings persistence | Python sidecar management         |
+----------------------------------------------------------+
|  Cloud Mode (Soniox)    |  Offline Mode (Python MLX)     |
|  WebSocket STT+translate |  MLX Whisper -> Opus-MT        |
|  ~150-300ms latency     |  ~3s latency                    |
+----------------------------------------------------------+
```

### Audio Sources

- **Microphone** - Default input via cpal
- **System audio** - All system output via macOS ScreenCaptureKit (excludes app's own audio to prevent TTS feedback)
- **Both** - Mic + system audio mixed by averaging PCM samples

### Offline Pipeline Optimizations

- **1.5s chunks / 1.0s stride** - Fast sliding window transcription
- **Exact prefix dedup** - Handles 1-word boundary overlaps from sliding window
- **Garbage suppression** - Detects and filters Whisper hallucination loops
- **Stale audio flush** - Drains buffer accumulated during model loading (~16s)
- **initial_prompt context** - Feeds previous transcript tail for better continuity
- **Configurable endpoint delay** - Silence threshold before translation

## Quick Start

### Prerequisites

- **Rust**: 1.70+
- **Node.js**: 18+
- **macOS** (primary target)

### Install & Run

```bash
git clone <repo-url> && cd auralis
npm install
npm run tauri dev
```

### Cloud Mode (Soniox)

1. Get a free API key at [soniox.com](https://soniox.com/)
2. Click the gear icon to open Settings
3. Select **Cloud** mode, enter API key, choose languages
4. Click the record button to start

### Offline Mode (MLX)

1. Open Settings (gear icon)
2. Select **Offline** mode
3. Click **Setup Offline Mode** - the app creates the Python environment and installs dependencies automatically
4. Click the record button to start

### Audio Source

In Settings, choose from:
- **Microphone** - Your voice input
- **System** - YouTube, meetings, any app audio (requires Screen Recording permission)
- **Both** - Mixed mic + system audio

## Supported Languages

English, Vietnamese, Spanish, French, German, Chinese, Japanese, Korean, Portuguese, Russian, Arabic, Hindi

Two-way translation available (auto-detects source language).

## Tech Stack

- **Backend**: Rust, Tokio, Tauri 2, cpal, ScreenCaptureKit (macOS)
- **Frontend**: Svelte 5 (runes), Vite, TypeScript
- **Cloud STT+Translation**: Soniox WebSocket API (stt-rt-v4)
- **Offline STT**: MLX Whisper (whisper-large-v3-turbo)
- **Offline Translation**: Helsinki-NLP Opus-MT (via HuggingFace Transformers)
- **Python Sidecar**: stdin/stdout JSON protocol, configurable chunk/stride/endpoint-delay

## Project Structure

```
auralis/
├── src-tauri/              # Tauri app
│   └── src/
│       ├── main.rs              # App entry point
│       ├── lib.rs               # Command registration
│       ├── commands_audio.rs    # Audio streaming (mic/system/both)
│       ├── commands_settings.rs # Settings persistence
│       ├── commands_pipeline.rs # Offline pipeline + audio write loop
│       ├── audio/
│       │   ├── mod.rs           # PCM conversion, audio mixing
│       │   └── system_audio.rs  # ScreenCaptureKit system audio
│       └── state.rs             # App state + settings
├── src/                    # Svelte frontend
│   ├── App.svelte               # Main overlay shell
│   ├── app.css                  # Glassmorphism design system
│   ├── types.ts                 # Shared TypeScript types
│   ├── js/
│   │   ├── soniox.ts            # Soniox WebSocket client
│   │   └── lang.ts              # Language name mappings
│   └── components/
│       ├── ControlBar.svelte    # Drag region + window controls
│       ├── Transcript.svelte    # Single/dual view transcript
│       ├── SettingsView.svelte  # Settings overlay with tabs
│       └── ModelDownloader.svelte # Offline setup widget
├── scripts/
│   └── local_pipeline.py   # Offline MLX + Opus-MT Python sidecar
└── Cargo.toml
```

## Running Tests

```bash
cargo test
```

## License

MIT
