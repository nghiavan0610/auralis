# Auralis

Real-time speech translation desktop app with dual-mode architecture: **cloud** (Soniox, ~150ms) and **offline** (MLX Whisper + Opus-MT, ~1s).

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│                    Svelte Frontend                        │
│  Mode Selector | Soniox Client | DualPanel UI            │
├──────────────────────────────────────────────────────────┤
│  Rust/Tauri Backend                                       │
│  Audio Capture (cpal) → PCM streaming via Tauri events   │
│  Settings persistence | Python sidecar management         │
├──────────────────────────────────────────────────────────┤
│  Cloud Mode (Soniox)    │  Offline Mode (Python MLX)     │
│  WebSocket STT+translate │  MLX Whisper → Opus-MT        │
│  ~150-300ms latency     │  ~3.5s latency                 │
└──────────────────────────────────────────────────────────┘
```

## Quick Start

### Prerequisites

- **Rust**: 1.70+
- **Node.js**: 18+
- **macOS** (primary target; Windows/Linux supported via Tauri)

### Install & Run

```bash
git clone <repo-url> && cd auralis
npm install
npm run tauri dev
```

### Cloud Mode (Soniox)

1. Get a free API key at [soniox.com](https://soniox.com/)
2. Enter the key in the app, select languages, click **Start**

### Offline Mode (MLX)

Set up the Python environment first:

```bash
python3 -m venv ~/.config/auralis/mlx-env
source ~/.config/auralis/mlx-env/bin/activate
pip install mlx-whisper transformers numpy
```

Then select **Offline** mode in the app and click **Start**. Models download automatically on first use.

## Supported Languages

English, Vietnamese, Spanish, French, German, Chinese, Japanese, Korean, Portuguese, Russian, Arabic, Hindi

## Tech Stack

- **Backend**: Rust, Tokio, Tauri 2, cpal
- **Frontend**: Svelte 5, Vite, TypeScript
- **Cloud STT+Translation**: Soniox WebSocket API (stt-rt-v4)
- **Offline STT**: MLX Whisper (whisper-large-v3-turbo)
- **Offline Translation**: Helsinki-NLP Opus-MT (via HuggingFace Transformers)
- **Python Sidecar**: stdin/stdout JSON protocol

## Project Structure

```
auralis/
├── src/                    # Rust library (audio capture, domain, logging)
├── src-tauri/              # Tauri app (commands, state, pipeline)
│   └── src/
│       ├── main.rs         # App entry point
│       ├── commands.rs     # Core Tauri commands
│       ├── commands_audio.rs    # Audio streaming
│       ├── commands_settings.rs # Settings persistence
│       └── commands_pipeline.rs # Offline pipeline management
├── src/                    # Svelte frontend
│   ├── App.svelte          # Main UI with mode selector
│   ├── js/soniox.ts        # Soniox WebSocket client
│   └── components/         # UI components
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
