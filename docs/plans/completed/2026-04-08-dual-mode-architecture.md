# Dual-Mode Translation Architecture Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the current whisper-rs + NLLB pipeline with a dual-mode architecture: Soniox cloud mode (~150-300ms latency) and offline MLX mode (whisper-large-v3-turbo + Opus-MT, ~0.5-1s latency).

**Architecture:** Audio is captured by Rust (cpal) and streamed to different consumers based on mode. In **cloud mode**, PCM audio is emitted to the Svelte frontend via Tauri events, where a Soniox WebSocket client handles STT+translation in a single API call. In **offline mode**, PCM audio is piped to a Python sidecar process that runs MLX Whisper for STT and Helsinki-NLP Opus-MT for translation, with JSON results returned via stdout. Both modes display results in the same DualPanel UI.

**Tech Stack:** Rust/Tauri 2, Svelte 5, Soniox WebSocket API, Python MLX (mlx-whisper), Helsinki-NLP/opus-mt-en-vi (via HuggingFace Transformers), cpal, tokio

---

## Reference: my-translator at `/Users/benq/Desktop/sentia-lab/my-translator`

This plan is based on the proven architecture in the my-translator project. Key reference files:
- Soniox client: `/Users/benq/Desktop/sentia-lab/my-translator/src/js/soniox.js`
- Audio streaming: `/Users/benq/Desktop/sentia-lab/my-translator/src-tauri/src/commands/audio.rs`
- Python sidecar: `/Users/benq/Desktop/sentia-lab/my-translator/scripts/local_pipeline.py`
- Sidecar management: `/Users/benq/Desktop/sentia-lab/my-translator/src-tauri/src/commands/local_pipeline.rs`
- Settings: `/Users/benq/Desktop/sentia-lab/my-translator/src-tauri/src/settings.rs`

---

### Task 1: Add audio streaming commands

Add new Tauri commands that capture audio from the microphone and stream PCM chunks to the frontend as Tauri events. This replaces the orchestrator-based audio consumption.

**Files:**
- Create: `src-tauri/src/commands/audio.rs`
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: Update state.rs to hold audio capture state**

Replace the orchestrator-based state with audio streaming state. Keep language settings.

```rust
// src-tauri/src/state.rs
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelStatus {
    pub stt_available: bool,
    pub stt_model: String,
    pub translation_available: bool,
    pub translation_model: String,
    pub vad_available: bool,
    pub vad_model: String,
    pub system_ready: bool,
}

impl Default for ModelStatus {
    fn default() -> Self {
        Self {
            stt_available: false,
            stt_model: "Whisper".to_string(),
            translation_available: false,
            translation_model: "NLLB".to_string(),
            vad_available: false,
            vad_model: "Silero".to_string(),
            system_ready: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub mode: String,           // "cloud" or "offline"
    pub soniox_api_key: String,
    pub source_language: String,
    pub target_language: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: "cloud".to_string(),
            soniox_api_key: String::new(),
            source_language: "en".to_string(),
            target_language: "vi".to_string(),
        }
    }
}

pub struct AuralisState {
    pub is_streaming: Arc<AtomicBool>,
    pub audio_data: Arc<Mutex<Vec<Vec<f32>>>>>,
    pub is_recording: Arc<Mutex<bool>>,
    pub stream_stop: Arc<std::sync::atomic::AtomicBool>,
    pub settings: Arc<Mutex<Settings>>,
    pub model_status: Arc<Mutex<ModelStatus>>,
}

impl AuralisState {
    pub fn new() -> Self {
        Self {
            is_streaming: Arc::new(AtomicBool::new(false)),
            audio_data: Arc::new(Mutex::new(Vec::new())),
            is_recording: Arc::new(Mutex::new(false)),
            stream_stop: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            settings: Arc::new(Mutex::new(Settings::default())),
            model_status: Arc::new(Mutex::new(ModelStatus::default())),
        }
    }

    pub async fn source_language(&self) -> String {
        self.settings.lock().await.source_language.clone()
    }

    pub async fn target_language(&self) -> String {
        self.settings.lock().await.target_language.clone()
    }

    pub async fn mode(&self) -> String {
        self.settings.lock().await.mode.clone()
    }

    pub async fn model_status(&self) -> ModelStatus {
        self.model_status.lock().await.clone()
    }
}

impl Default for AuralisState {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 2: Create audio.rs with streaming commands**

```rust
// src-tauri/src/commands/audio.rs
use crate::state::AuralisState;
use auralis::infrastructure::audio::capture::{MicrophoneCapture, AudioCaptureConfig};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

pub struct CaptureHandle {
    pub audio_data: Arc<Mutex<Vec<Vec<f32>>>>>,
    pub is_recording: Arc<Mutex<bool>>,
    pub stream_stop: Arc<std::sync::atomic::AtomicBool>,
}

/// Start audio capture and stream PCM to frontend
#[tauri::command]
pub async fn start_audio_capture(
    state: State<'_, AuralisState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    if state.is_streaming.load(Ordering::Relaxed) {
        return Err("Audio capture already running".to_string());
    }

    let config = AudioCaptureConfig::default();
    let mut capture = MicrophoneCapture::new(config)
        .map_err(|e| format!("Failed to create capture: {}", e))?;

    capture.start().await
        .map_err(|e| format!("Failed to start capture: {}", e))?;

    // Get audio buffer reference from the capture
    // MicrophoneCapture stores audio_data as Arc<Mutex<Vec<Vec<f32>>>>>
    // We need to share this between the capture and our streaming task
    let audio_data = state.audio_data.clone();
    let is_recording = state.is_recording.clone();
    let stream_stop = state.stream_stop.clone();
    let is_streaming = state.is_streaming.clone();

    // Clear buffer
    {
        let mut data = audio_data.lock().await;
        data.clear();
    }

    // Set recording flag
    {
        let mut rec = is_recording.lock().await;
        *rec = true;
    }

    state.is_streaming.store(true, Ordering::Relaxed);

    let _ = app_handle.emit("audio-capture", serde_json::json!({"is_capturing": true}));

    // The MicrophoneCapture's internal thread is already writing to audio_data.
    // We spawn a task that reads from audio_data and emits to frontend.
    let app = app_handle.clone();
    tokio::spawn(async move {
        while is_streaming.load(Ordering::Relaxed) {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;

            let chunks: Vec<Vec<f32>> = {
                let mut data = audio_data.lock().await;
                std::mem::take(&mut *data)
            };

            if chunks.is_empty() {
                continue;
            }

            // Combine chunks and convert f32 -> s16le PCM bytes
            let combined: Vec<f32> = chunks.into_iter().flatten().collect();
            let pcm_bytes: Vec<u8> = combined.iter()
                .flat_map(|s| {
                    let val = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
                    val.to_le_bytes()
                })
                .collect();

            if !pcm_bytes.is_empty() {
                let _ = app.emit("audio-data", pcm_bytes);
            }
        }
    });

    Ok(())
}

/// Stop audio capture
#[tauri::command]
pub async fn stop_audio_capture(
    state: State<'_, AuralisState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    if !state.is_streaming.load(Ordering::Relaxed) {
        return Err("Audio capture not running".to_string());
    }

    state.is_streaming.store(false, Ordering::Relaxed);

    {
        let mut rec = state.is_recording.lock().await;
        *rec = false;
    }

    state.stream_stop.store(false, Ordering::Relaxed);

    let _ = app_handle.emit("audio-capture", serde_json::json!({"is_capturing": false}));

    Ok(())
}
```

**Step 3: Update lib.rs to register new commands**

```rust
// src-tauri/src/lib.rs
pub mod state;
pub mod commands;
pub mod model_downloader;

// New modules for dual-mode architecture
pub mod commands_audio;
pub mod commands_settings;
pub mod commands_pipeline;

pub use state::{AuralisState, ModelStatus};
pub use commands::*;
```

**Step 4: Register commands in main.rs**

Update the Tauri builder in `src-tauri/src/main.rs` to include the new commands:

```rust
// In the invoke_handler list, add:
commands_audio::start_audio_capture,
commands_audio::stop_audio_capture,
commands_settings::get_settings,
commands_settings::save_settings,
commands_pipeline::start_local_pipeline,
commands_pipeline::stop_local_pipeline,
```

**Step 5: Verify compilation**

Run: `cd /Users/benq/Desktop/sentia-lab/auralis && cargo check`
Expected: Compiles with warnings (unused imports are OK)

**Step 6: Commit**

```bash
git add src-tauri/src/state.rs src-tauri/src/commands_audio.rs src-tauri/src/lib.rs src-tauri/src/main.rs
git commit -m "feat: add audio streaming state and commands for dual-mode architecture"
```

---

### Task 2: Create Soniox WebSocket client in TypeScript

Create a TypeScript client that connects to Soniox's WebSocket API, sends PCM audio, and receives transcription + translation tokens.

**Files:**
- Create: `src/js/soniox.ts`

**Step 1: Create soniox.ts**

```typescript
// src/js/soniox.ts

const SONIOX_ENDPOINT = 'wss://stt-rt.soniox.com/transcribe-websocket';
const SESSION_DURATION_MS = 3 * 60 * 1000; // Reset every 3 minutes
const KEEPALIVE_INTERVAL_MS = 15000;

export interface SonioxToken {
  text: string;
  is_final: boolean;
  translation_status: 'original' | 'translation' | 'none';
  speaker?: number;
  language?: string;
}

export interface SonioxConfig {
  api_key: string;
  source_language: string;
  target_language: string;
  translation_type: 'one_way' | 'two_way';
  onOriginal: (text: string, is_final: boolean) => void;
  onTranslation: (text: string, is_final: boolean) => void;
  onStatusChange: (status: string) => void;
  onError: (error: string) => void;
}

export class SonioxClient {
  private ws: WebSocket | null = null;
  private config: SonioxConfig;
  private sessionTimer: ReturnType<typeof setInterval> | null = null;
  private keepaliveTimer: ReturnType<typeof setInterval> | null = null;
  private running = false;

  constructor(config: SonioxConfig) {
    this.config = config;
  }

  connect(): void {
    this.running = true;
    this.doConnect();
  }

  disconnect(): void {
    this.running = false;
    this.cleanup();
    if (this.ws) {
      this.ws.close(1000, 'User disconnect');
      this.ws = null;
    }
  }

  sendAudio(pcmData: ArrayBuffer | Uint8Array): void {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(pcmData);
    }
  }

  private doConnect(): void {
    if (!this.running) return;

    this.config.onStatusChange('Connecting...');
    const ws = new WebSocket(SONIOX_ENDPOINT);

    ws.onopen = () => {
      // Send configuration
      const configMsg: Record<string, unknown> = {
        api_key: this.config.api_key,
        model: 'stt-rt-v4',
        audio_format: 'pcm_s16le',
        sample_rate: 16000,
        num_channels: 1,
        enable_endpoint_detection: true,
        max_endpoint_delay_ms: 1500,
        enable_language_identification: true,
        language_hints: [this.config.source_language],
        translation: {
          type: this.config.translation_type,
          target_language: this.config.target_language,
        },
      };

      if (this.config.translation_type === 'two_way') {
        configMsg.translation = {
          type: 'two_way',
          language_a: this.config.source_language,
          language_b: this.config.target_language,
        };
      }

      ws.send(JSON.stringify(configMsg));

      // Set up keepalive
      this.keepaliveTimer = setInterval(() => {
        if (ws.readyState === WebSocket.OPEN) {
          ws.send(JSON.stringify({ type: 'keepalive' }));
        }
      }, KEEPALIVE_INTERVAL_MS);

      // Set up session reset
      this.sessionTimer = setInterval(() => {
        this.doConnect(); // Make-before-break: new connection first
      }, SESSION_DURATION_MS);

      this.config.onStatusChange('Connected');
    };

    ws.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data);

        if (msg.tokens && Array.isArray(msg.tokens)) {
          for (const token of msg.tokens) {
            if (token.text === '<end>') continue;

            if (token.translation_status === 'original' || token.translation_status === 'none') {
              this.config.onOriginal(token.text, token.is_final);
            }

            if (token.translation_status === 'translation') {
              this.config.onTranslation(token.text, token.is_final);
            }
          }
        }
      } catch (e) {
        // Ignore parse errors for non-JSON messages
      }
    };

    ws.onerror = () => {
      this.config.onError('WebSocket error');
    };

    ws.onclose = (event) => {
      this.cleanup();

      if (!this.running) return;

      // Reconnect on abnormal close
      if (event.code === 1006 || event.code === 408) {
        this.config.onStatusChange('Reconnecting...');
        setTimeout(() => this.doConnect(), 2000);
      } else if (event.code !== 1000) {
        this.config.onError(`Connection closed: ${event.code} ${event.reason}`);
      }
    };

    // Close old connection if make-before-break
    const oldWs = this.ws;
    this.ws = ws;
    if (oldWs && oldWs !== ws && oldWs.readyState === WebSocket.OPEN) {
      oldWs.close(1000, 'Session reset');
    }
  }

  private cleanup(): void {
    if (this.keepaliveTimer) {
      clearInterval(this.keepaliveTimer);
      this.keepaliveTimer = null;
    }
    if (this.sessionTimer) {
      clearInterval(this.sessionTimer);
      this.sessionTimer = null;
    }
  }
}
```

**Step 2: Commit**

```bash
mkdir -p src/js
git add src/js/soniox.ts
git commit -m "feat: add Soniox WebSocket client for cloud STT+translation"
```

---

### Task 3: Add settings management

Add commands to load/save settings (mode, API key, languages) to a JSON file.

**Files:**
- Create: `src-tauri/src/commands_settings.rs`

**Step 1: Create commands_settings.rs**

```rust
// src-tauri/src/commands_settings.rs
use crate::state::{AuralisState, Settings};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::State;

fn settings_path() -> PathBuf {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."));
    config_dir.join("auralis").join("settings.json")
}

fn load_settings_from_file() -> Settings {
    let path = settings_path();
    if path.exists() {
        if let Ok(data) = std::fs::read_to_string(&path) {
            if let Ok(settings) = serde_json::from_str(&data) {
                return settings;
            }
        }
    }
    Settings::default()
}

fn save_settings_to_file(settings: &Settings) -> Result<(), String> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config dir: {}", e))?;
    }
    let data = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    std::fs::write(&path, data)
        .map_err(|e| format!("Failed to write settings: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AuralisState>) -> Result<Settings, String> {
    // First try to load from file, then fall back to in-memory
    let file_settings = load_settings_from_file();
    let mut current = state.settings.lock().await;

    // If in-memory is default and file has values, use file
    if current.soniox_api_key.is_empty() && !file_settings.soniox_api_key.is_empty() {
        *current = file_settings.clone();
    }

    Ok(current.clone())
}

#[tauri::command]
pub async fn save_settings(
    state: State<'_, AuralisState>,
    settings: Settings,
) -> Result<String, String> {
    save_settings_to_file(&settings)?;

    let mut current = state.settings.lock().await;
    *current = settings;

    Ok("Settings saved".to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelStatus {
    pub stt_available: bool,
    pub stt_model: String,
    pub translation_available: bool,
    pub translation_model: String,
    pub vad_available: bool,
    pub vad_model: String,
    pub system_ready: bool,
}
```

**Step 2: Commit**

```bash
git add src-tauri/src/commands_settings.rs
git commit -m "feat: add settings management with JSON persistence"
```

---

### Task 4: Create Python sidecar for offline mode

Create a Python script that reads PCM audio from stdin, transcribes with MLX Whisper, translates with Opus-MT, and outputs JSON results to stdout.

**Files:**
- Create: `scripts/local_pipeline.py`

**Step 1: Create the sidecar script**

```python
#!/usr/bin/env python3
"""Offline MLX pipeline for Auralis: Whisper STT + Opus-MT translation.

Reads PCM s16le (16kHz mono) from stdin.
Outputs JSON results to stdout.

Usage:
    python local_pipeline.py --source-lang en --target-lang vi

Protocol:
    stdin:  Raw PCM s16le bytes (continuous stream)
    stdout: JSON lines, one of:
        {"type":"status","message":"Loading models..."}
        {"type":"ready"}
        {"type":"result","original":"...","translated":"...","timing":{"asr":0.0,"translate":0.0,"total":0.0}}
        {"type":"done"}
"""

import argparse
import json
import sys
import time
import wave
import tempfile
import os
import threading
import numpy as np
from pathlib import Path

# Audio constants
SAMPLE_RATE = 16000
CHANNELS = 1
SAMPLE_WIDTH = 2  # s16le = 2 bytes
CHUNK_SECONDS = 3
STRIDE_SECONDS = 2
CHUNK_SAMPLES = SAMPLE_RATE * CHUNK_SECONDS
STRIDE_SAMPLES = SAMPLE_RATE * STRIDE_SECONDS
CHUNK_BYTES = CHUNK_SAMPLES * SAMPLE_WIDTH
STRIDE_BYTES = STRIDE_SAMPLES * SAMPLE_WIDTH
SILENCE_THRESHOLD = 100  # RMS below this = silence


def emit(msg: dict):
    """Write a JSON message to stdout and flush."""
    sys.stdout.write(json.dumps(msg) + '\n')
    sys.stdout.flush()


class LocalPipeline:
    def __init__(self, source_lang: str, target_lang: str,
                 asr_model: str = "mlx-community/whisper-large-v3-turbo"):
        self.source_lang = source_lang
        self.target_lang = target_lang
        self.asr_model = asr_model
        self.audio_buffer = bytearray()
        self.processed_pos = 0
        self.lock = threading.Lock()
        self.whisper_model = None
        self.translator_tokenizer = None
        self.translator_model = None
        self.prev_original = ""
        self.prev_translation = ""

    def load_models(self):
        """Load Whisper and Opus-MT models."""
        emit({"type": "status", "message": "Loading Whisper model..."})

        # Import here so we can report status before heavy imports
        import mlx_whisper

        # Warm up Whisper with a dummy transcription
        emit({"type": "status", "message": "Warming up Whisper..."})

        # Load Opus-MT for translation
        emit({"type": "status", "message": "Loading Opus-MT translation model..."})
        from transformers import MarianMTModel, MarianTokenizer

        # Map language codes to Opus-MT model names
        opus_models = {
            ("en", "vi"): "Helsinki-NLP/opus-mt-en-vi",
            ("en", "es"): "Helsinki-NLP/opus-mt-en-es",
            ("en", "fr"): "Helsinki-NLP/opus-mt-en-fr",
            ("en", "de"): "Helsinki-NLP/opus-mt-en-de",
            ("en", "zh"): "Helsinki-NLP/opus-mt-en-zh",
            ("en", "ja"): "Helsinki-NLP/opus-mt-en-ja",
            ("vi", "en"): "Helsinki-NLP/opus-mt-vi-en",
            ("es", "en"): "Helsinki-NLP/opus-mt-es-en",
            ("fr", "en"): "Helsinki-NLP/opus-mt-fr-en",
            ("de", "en"): "Helsinki-NLP/opus-mt-de-en",
            ("zh", "en"): "Helsinki-NLP/opus-mt-zh-en",
            ("ja", "en"): "Helsinki-NLP/opus-mt-ja-en",
        }

        model_name = opus_models.get((self.source_lang, self.target_lang))
        if not model_name:
            emit({"type": "status", "message": f"No Opus-MT model for {self.source_lang}->{self.target_lang}, using en->vi"})
            model_name = "Helsinki-NLP/opus-mt-en-vi"

        self.translator_tokenizer = MarianTokenizer.from_pretrained(model_name)
        self.translator_model = MarianMTModel.from_pretrained(model_name)

        emit({"type": "status", "message": "Warming up translator..."})
        # Warm up translator
        inputs = self.translator_tokenizer("Hello", return_tensors="pt", padding=True)
        _ = self.translator_model.generate(**inputs, max_new_tokens=20)

        emit({"type": "ready"})

    def transcribe(self, pcm_bytes: bytes) -> str:
        """Transcribe PCM audio using MLX Whisper."""
        import mlx_whisper

        # Write PCM to temp WAV file (mlx_whisper needs a file path)
        with tempfile.NamedTemporaryFile(suffix='.wav', delete=False) as f:
            temp_path = f.name
            with wave.open(f, 'wb') as wf:
                wf.setnchannels(CHANNELS)
                wf.setsampwidth(SAMPLE_WIDTH)
                wf.setframerate(SAMPLE_RATE)
                wf.writeframes(pcm_bytes)

        try:
            result = mlx_whisper.transcribe(
                temp_path,
                path_or_hf_repo=self.asr_model,
                language=self.source_lang if self.source_lang != "auto" else None,
            )
            return result.get("text", "").strip()
        finally:
            os.unlink(temp_path)

    def translate(self, text: str) -> str:
        """Translate text using Opus-MT."""
        if not text.strip():
            return ""

        inputs = self.translator_tokenizer(text, return_tensors="pt", padding=True)
        outputs = self.translator_model.generate(**inputs, max_new_tokens=200)
        return self.translator_tokenizer.decode(outputs[0], skip_special_tokens=True)

    def deduplicate(self, prev: str, current: str, min_overlap: int = 3) -> str:
        """Remove overlapping text between consecutive chunks."""
        if not prev or not current:
            return current

        # Character-level dedup for CJK, word-level for others
        if any('\u4e00' <= c <= '\u9fff' for c in prev):
            # Character-based
            max_check = min(len(prev), len(current), 100)
            best = 0
            for i in range(min_overlap, max_check + 1):
                if prev[-i:] == current[:i]:
                    best = i
            return current[best:] if best >= min_overlap else current
        else:
            # Word-based
            prev_words = prev.split()
            curr_words = current.split()
            max_check = min(len(prev_words), len(curr_words), 30)
            best = 0
            for i in range(min_overlap, max_check + 1):
                if prev_words[-i:] == curr_words[:i]:
                    best = i
            return ' '.join(curr_words[best:]) if best >= min_overlap else current

    def process_chunk(self, pcm_bytes: bytes):
        """Process one audio chunk: transcribe + translate."""
        t_start = time.time()

        # Check for silence
        samples = np.frombuffer(pcm_bytes, dtype=np.int16).astype(np.float32) / 32768.0
        rms = np.sqrt(np.mean(samples ** 2)) * 32768
        if rms < SILENCE_THRESHOLD:
            return

        # Transcribe
        t_asr_start = time.time()
        original = self.transcribe(pcm_bytes)
        t_asr = time.time() - t_asr_start

        if not original:
            return

        # Deduplicate
        new_text = self.deduplicate(self.prev_original, original)
        if not new_text.strip():
            return

        # Translate only the new portion
        t_trans_start = time.time()
        translated = self.translate(new_text)
        t_trans = time.time() - t_trans_start

        # Deduplicate translation
        new_translation = self.deduplicate(self.prev_translation, translated, min_overlap=2)

        total = time.time() - t_start

        # Update state
        self.prev_original = original
        self.prev_translation = translated

        emit({
            "type": "result",
            "original": new_text,
            "translated": new_translation,
            "timing": {
                "asr": round(t_asr, 2),
                "translate": round(t_trans, 2),
                "total": round(total, 2),
            }
        })

    def add_audio(self, data: bytes):
        """Add audio data to buffer (called from stdin reader thread)."""
        with self.lock:
            self.audio_buffer.extend(data)

    def process_loop(self):
        """Main processing loop that runs in the main thread."""
        while True:
            time.sleep(0.5)  # Check every 500ms

            with self.lock:
                buf_len = len(self.audio_buffer)
                if buf_len - self.processed_pos >= CHUNK_BYTES:
                    chunk = bytes(self.audio_buffer[self.processed_pos:self.processed_pos + CHUNK_BYTES])
                    self.processed_pos += STRIDE_BYTES

                    # Trim buffer to prevent unbounded growth
                    if self.processed_pos > CHUNK_BYTES * 2:
                        self.audio_buffer = self.audio_buffer[self.processed_pos:]
                        self.processed_pos = 0
                else:
                    continue

            self.process_chunk(chunk)


def main():
    parser = argparse.ArgumentParser(description='Auralis offline MLX pipeline')
    parser.add_argument('--source-lang', default='en', help='Source language code')
    parser.add_argument('--target-lang', default='vi', help='Target language code')
    parser.add_argument('--asr-model', default='mlx-community/whisper-large-v3-turbo',
                        help='MLX Whisper model (HuggingFace repo)')
    args = parser.parse_args()

    pipeline = LocalPipeline(
        source_lang=args.source_lang,
        target_lang=args.target_lang,
        asr_model=args.asr_model,
    )

    # Load models
    pipeline.load_models()

    # Read stdin in background thread
    def stdin_reader():
        while True:
            data = sys.stdin.buffer.read(4096)
            if not data:
                break
            pipeline.add_audio(data)
        emit({"type": "done"})

    reader_thread = threading.Thread(target=stdin_reader, daemon=True)
    reader_thread.start()

    # Process in main thread
    pipeline.process_loop()


if __name__ == '__main__':
    main()
```

**Step 2: Commit**

```bash
mkdir -p scripts
git add scripts/local_pipeline.py
git commit -m "feat: add Python sidecar for offline MLX Whisper + Opus-MT"
```

---

### Task 5: Add Rust sidecar management commands

Add Tauri commands that spawn the Python sidecar, pipe audio to it, and forward results to the frontend.

**Files:**
- Create: `src-tauri/src/commands_pipeline.rs`

**Step 1: Create commands_pipeline.rs**

```rust
// src-tauri/src/commands_pipeline.rs
use crate::state::AuralisState;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::io::Write;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

pub struct PipelineState {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    running: Arc<AtomicBool>,
}

/// Start the local Python pipeline for offline mode
#[tauri::command]
pub async fn start_local_pipeline(
    state: State<'_, AuralisState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    if state.is_streaming.load(Ordering::Relaxed) {
        return Err("Pipeline already running".to_string());
    }

    let settings = state.settings.lock().await;
    let source_lang = settings.source_language.clone();
    let target_lang = settings.target_language.clone();
    drop(settings);

    // Find Python executable (check for venv first)
    let script_path = std::path::PathBuf::from("../scripts/local_pipeline.py");
    let python = find_python()?;

    let mut child = Command::new(&python)
        .arg(&script_path)
        .arg("--source-lang").arg(&source_lang)
        .arg("--target-lang").arg(&target_lang)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start pipeline: {}", e))?;

    let stdin = child.stdin.take()
        .ok_or("Failed to get stdin")?;
    let stdout = child.stdout.take()
        .ok_or("Failed to get stdout")?;

    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    // Read stdout in background, forward to frontend
    let app = app_handle.clone();
    std::thread::spawn(move || {
        use std::io::BufRead;
        let reader = std::io::BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(text) => {
                    if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&text) {
                        let msg_type = json_val.get("type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");

                        match msg_type {
                            "result" => {
                                // Forward to frontend as translation result
                                let _ = app.emit("pipeline-result", json_val);
                            }
                            "status" => {
                                let msg = json_val.get("message")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                let _ = app.emit("pipeline-status", msg);
                            }
                            "ready" => {
                                let _ = app.emit("pipeline-status", "Pipeline ready");
                            }
                            "done" => break,
                            _ => {}
                        }
                    }
                }
                Err(_) => break,
            }
        }
        running_clone.store(false, Ordering::Relaxed);
    });

    // Store pipeline state
    // Note: We need to store child + stdin somewhere accessible.
    // For simplicity, we store them in a global Mutex.
    // In production, add a pipeline field to AuralisState.
    *PIPELINE_STATE.lock().await = Some(PipelineState {
        child: Some(child),
        stdin: Some(stdin),
        running: running.clone(),
    });

    state.is_streaming.store(true, Ordering::Relaxed);

    // Also start audio capture
    // Audio chunks from the frontend (via audio-data events) will be
    // forwarded to the pipeline's stdin by the streaming task
    start_audio_for_pipeline(state, app_handle, running).await?;

    Ok(())
}

/// Stop the local pipeline
#[tauri::command]
pub async fn stop_local_pipeline(
    state: State<'_, AuralisState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    state.is_streaming.store(false, Ordering::Relaxed);

    let mut pipeline = PIPELINE_STATE.lock().await;
    if let Some(ref mut p) = *pipeline {
        p.running.store(false, Ordering::Relaxed);
        // Close stdin to signal Python to stop
        p.stdin.take();
        // Kill the process
        if let Some(ref mut child) = p.child {
            let _ = child.kill();
        }
    }
    *pipeline = None;

    let _ = app_handle.emit("pipeline-status", "Pipeline stopped");

    Ok(())
}

/// Send audio data to the pipeline's stdin
/// Called from the audio streaming task
pub async fn send_to_pipeline(data: &[u8]) -> Result<(), String> {
    let pipeline = PIPELINE_STATE.lock().await;
    if let Some(ref p) = *pipeline {
        if let Some(ref mut stdin) = p.stdin {
            stdin.write_all(data)
                .map_err(|e| format!("Failed to write to pipeline: {}", e))?;
            stdin.flush()
                .map_err(|e| format!("Failed to flush pipeline: {}", e))?;
        }
    }
    Ok(())
}

// Global pipeline state (in production, move into AuralisState)
static PIPELINE_STATE: once_cell::sync::Lazy<tokio::sync::Mutex<Option<PipelineState>>> =
    once_cell::sync::Lazy::new(|| tokio::sync::Mutex::new(None));

fn find_python() -> Result<String, String> {
    // Check for MLX venv first
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let venv_python = config_dir.join("auralis").join("mlx-env").join("bin").join("python3");

    if venv_python.exists() {
        return Ok(venv_python.to_string_lossy().to_string());
    }

    // Fall back to system python3
    which_python("python3")
        .or_else(|_| which_python("python"))
}

fn which_python(name: &str) -> Result<String, String> {
    let output = std::process::Command::new("which")
        .arg(name)
        .output()
        .map_err(|e| format!("Failed to find python: {}", e))?;

    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Ok(path);
        }
    }

    Err("Python not found".to_string())
}

/// Start audio capture and forward to pipeline stdin
async fn start_audio_for_pipeline(
    state: State<'_, AuralisState>,
    app_handle: AppHandle,
    pipeline_running: Arc<AtomicBool>,
) -> Result<(), String> {
    use auralis::infrastructure::audio::capture::{MicrophoneCapture, AudioCaptureConfig};

    let config = AudioCaptureConfig::default();
    let mut capture = MicrophoneCapture::new(config)
        .map_err(|e| format!("Failed to create capture: {}", e))?;
    capture.start().await
        .map_err(|e| format!("Failed to start capture: {}", e))?;

    let audio_data = state.audio_data.clone();
    let is_recording = state.is_recording.clone();
    {
        let mut rec = is_recording.lock().await;
        *rec = true;
    }

    let _ = app_handle.emit("audio-capture", serde_json::json!({"is_capturing": true}));

    // Stream audio to pipeline
    let is_streaming = state.is_streaming.clone();
    tokio::spawn(async move {
        while is_streaming.load(Ordering::Relaxed) && pipeline_running.load(Ordering::Relaxed) {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;

            let chunks: Vec<Vec<f32>> = {
                let mut data = audio_data.lock().await;
                std::mem::take(&mut *data)
            };

            if chunks.is_empty() {
                continue;
            }

            let combined: Vec<f32> = chunks.into_iter().flatten().collect();
            let pcm_bytes: Vec<u8> = combined.iter()
                .flat_map(|s| {
                    let val = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
                    val.to_le_bytes()
                })
                .collect();

            if !pcm_bytes.is_empty() {
                let _ = send_to_pipeline(&pcm_bytes).await;
            }
        }
    });

    Ok(())
}
```

**Step 2: Add once_cell dependency to Cargo.toml**

Add to `src-tauri/Cargo.toml` dependencies:
```toml
once_cell = "1"
```

**Step 3: Commit**

```bash
git add src-tauri/src/commands_pipeline.rs src-tauri/Cargo.toml
git commit -m "feat: add Rust sidecar management for offline MLX pipeline"
```

---

### Task 6: Update Svelte frontend for dual-mode

Update the App.svelte to support mode switching (cloud/offline), integrate the Soniox client, and handle results from both modes uniformly.

**Files:**
- Modify: `src/App.svelte`

**Step 1: Rewrite App.svelte with dual-mode support**

Replace the entire content of `src/App.svelte` with:

```svelte
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import StatusIndicator from './components/StatusIndicator.svelte';
  import DualPanel from './components/DualPanel.svelte';
  import ModelDownloader from './components/ModelDownloader.svelte';
  import { SonioxClient } from './js/soniox';

  // Settings
  let mode = $state<'cloud' | 'offline'>('cloud');
  let sonioxApiKey = $state('');
  let sourceLanguage = $state('en');
  let targetLanguage = $state('vi');
  let isTranslating = $state(false);
  let statusMessage = $state('Ready');
  let errorMessage = $state('');
  let audioCapturing = $state(false);
  let sonioxStatus = $state('');

  // Results - unified format for both modes
  interface TranscriptEntry {
    text: string;
    language: string;
    timestamp: number;
    is_final: boolean;
  }

  interface TranslationEntry {
    original: string;
    translated: string;
    source_lang: string;
    target_lang: string;
    timestamp: number;
  }

  let transcriptions = $state<TranscriptEntry[]>([]);
  let translations = $state<TranslationEntry[]>([]);

  // Soniox client instance
  let sonioxClient: SonioxClient | null = null;

  // Current partial text from Soniox (non-final tokens)
  let sonioxPartialOriginal = $state('');
  let sonioxPartialTranslation = $state('');

  const unlisteners: Array<() => void> = [];

  onMount(async () => {
    try {
      // Load settings
      const settings = await invoke<{
        mode: string;
        soniox_api_key: string;
        source_language: string;
        target_language: string;
      }>('get_settings');

      if (settings.mode === 'offline') mode = 'offline';
      if (settings.soniox_api_key) sonioxApiKey = settings.soniox_api_key;
      if (settings.source_language) sourceLanguage = settings.source_language;
      if (settings.target_language) targetLanguage = settings.target_language;

      // Listen for pipeline results (offline mode)
      const pipelineResultUnlisten = await listen<any>('pipeline-result', (event) => {
        const data = event.payload;
        if (data.type === 'result') {
          const now = Date.now();
          transcriptions = [...transcriptions, {
            text: data.original,
            language: sourceLanguage,
            timestamp: now,
            is_final: true,
          }];
          translations = [...translations, {
            original: data.original,
            translated: data.translated,
            source_lang: sourceLanguage,
            target_lang: targetLanguage,
            timestamp: now,
          }];

          // Keep last 50
          if (transcriptions.length > 50) transcriptions = transcriptions.slice(-50);
          if (translations.length > 50) translations = translations.slice(-50);
        }
      });

      const pipelineStatusUnlisten = await listen<string>('pipeline-status', (event) => {
        statusMessage = event.payload;
      });

      const audioCaptureUnlisten = await listen<any>('audio-capture', (event) => {
        audioCapturing = event.payload.is_capturing;
      });

      // For cloud mode: listen for audio data and forward to Soniox
      const audioDataUnlisten = await listen<Uint8Array>('audio-data', (event) => {
        if (mode === 'cloud' && sonioxClient) {
          sonioxClient.sendAudio(event.payload);
        }
      });

      unlisteners.push(
        pipelineResultUnlisten,
        pipelineStatusUnlisten,
        audioCaptureUnlisten,
        audioDataUnlisten,
      );

    } catch (error) {
      errorMessage = `Failed to initialize: ${error}`;
    }
  });

  onDestroy(() => {
    unlisteners.forEach((fn) => fn());
    if (sonioxClient) {
      sonioxClient.disconnect();
    }
  });

  async function handleStart() {
    errorMessage = '';
    isTranslating = true;

    // Save settings
    await invoke('save_settings', {
      settings: {
        mode,
        soniox_api_key: sonioxApiKey,
        source_language: sourceLanguage,
        target_language: targetLanguage,
      }
    });

    if (mode === 'cloud') {
      await startCloudMode();
    } else {
      await startOfflineMode();
    }
  }

  async function handleStop() {
    if (mode === 'cloud') {
      await stopCloudMode();
    } else {
      await stopOfflineMode();
    }
    isTranslating = false;
    statusMessage = 'Stopped';
  }

  async function startCloudMode() {
    if (!sonioxApiKey) {
      errorMessage = 'Soniox API key is required for cloud mode';
      isTranslating = false;
      return;
    }

    // Create Soniox client
    sonioxClient = new SonioxClient({
      api_key: sonioxApiKey,
      source_language: sourceLanguage,
      target_language: targetLanguage,
      translation_type: 'one_way',
      onOriginal: (text, is_final) => {
        if (is_final) {
          transcriptions = [...transcriptions, {
            text: sonioxPartialOriginal + text,
            language: sourceLanguage,
            timestamp: Date.now(),
            is_final: true,
          }];
          sonioxPartialOriginal = '';
          if (transcriptions.length > 50) transcriptions = transcriptions.slice(-50);
        } else {
          sonioxPartialOriginal += text;
        }
      },
      onTranslation: (text, is_final) => {
        if (is_final) {
          const original = transcriptions.length > 0
            ? transcriptions[transcriptions.length - 1].text
            : '';
          translations = [...translations, {
            original,
            translated: sonioxPartialTranslation + text,
            source_lang: sourceLanguage,
            target_lang: targetLanguage,
            timestamp: Date.now(),
          }];
          sonioxPartialTranslation = '';
          if (translations.length > 50) translations = translations.slice(-50);
        } else {
          sonioxPartialTranslation += text;
        }
      },
      onStatusChange: (status) => {
        sonioxStatus = status;
        statusMessage = `Cloud: ${status}`;
      },
      onError: (error) => {
        errorMessage = `Soniox: ${error}`;
      },
    });

    // Start audio capture (Rust sends PCM to frontend via audio-data events)
    try {
      await invoke('start_audio_capture');
      sonioxClient.connect();
      statusMessage = 'Cloud mode active';
    } catch (error) {
      errorMessage = `Failed to start cloud mode: ${error}`;
      isTranslating = false;
    }
  }

  async function stopCloudMode() {
    if (sonioxClient) {
      sonioxClient.disconnect();
      sonioxClient = null;
    }
    try {
      await invoke('stop_audio_capture');
    } catch (error) {
      console.error('Failed to stop audio capture:', error);
    }
  }

  async function startOfflineMode() {
    try {
      statusMessage = 'Starting offline pipeline...';
      await invoke('start_local_pipeline');
      statusMessage = 'Offline mode active';
    } catch (error) {
      errorMessage = `Failed to start offline mode: ${error}`;
      isTranslating = false;
    }
  }

  async function stopOfflineMode() {
    try {
      await invoke('stop_local_pipeline');
    } catch (error) {
      console.error('Failed to stop pipeline:', error);
    }
  }

  function getStatusIndicator() {
    if (errorMessage) return 'error';
    if (isTranslating) return 'processing';
    return 'idle';
  }
</script>

<div class="container">
  <header>
    <h1>Auralis</h1>
    <p>Real-time Speech Translation</p>
  </header>

  {#if errorMessage}
    <div class="error">{errorMessage}</div>
  {/if}

  <ModelDownloader />

  <!-- Mode selector -->
  <div class="mode-selector">
    <label>
      <input type="radio" name="mode" value="cloud" bind:group={mode} disabled={isTranslating} />
      Cloud (Soniox) ~150ms
    </label>
    <label>
      <input type="radio" name="mode" value="offline" bind:group={mode} disabled={isTranslating} />
      Offline (MLX) ~1s
    </label>
  </div>

  <!-- Settings -->
  <div class="settings">
    {#if mode === 'cloud'}
      <div class="setting-row">
        <label for="api-key">Soniox API Key:</label>
        <input id="api-key" type="password" bind:value={sonioxApiKey} disabled={isTranslating}
               placeholder="Enter your Soniox API key" />
        <a href="https://soniox.com/" target="_blank" class="link">Get API Key</a>
      </div>
    {/if}

    <div class="setting-row">
      <label for="source-language">Source:</label>
      <select id="source-language" bind:value={sourceLanguage} disabled={isTranslating}>
        <option value="en">English</option>
        <option value="vi">Vietnamese</option>
        <option value="es">Spanish</option>
        <option value="fr">French</option>
        <option value="de">German</option>
        <option value="zh">Chinese</option>
        <option value="ja">Japanese</option>
      </select>

      <span style="margin: 0 0.5rem; color: rgba(255,255,255,0.5);">&rarr;</span>

      <label for="target-language">Target:</label>
      <select id="target-language" bind:value={targetLanguage} disabled={isTranslating}>
        <option value="vi">Vietnamese</option>
        <option value="en">English</option>
        <option value="es">Spanish</option>
        <option value="fr">French</option>
        <option value="de">German</option>
        <option value="zh">Chinese</option>
        <option value="ja">Japanese</option>
      </select>

      <div style="flex:1;"></div>

      {#if !isTranslating}
        <button class="primary" onclick={handleStart}
                disabled={mode === 'cloud' && !sonioxApiKey}>
          Start
        </button>
      {:else}
        <button class="danger" onclick={handleStop}>
          Stop
        </button>
      {/if}
    </div>
  </div>

  <!-- Dual panel results -->
  <DualPanel
    {sourceLanguage}
    {targetLanguage}
    {transcriptions}
    {translations}
  />

  <!-- Status bar -->
  <div class="status-bar">
    <span>
      <StatusIndicator status={getStatusIndicator()} text={isTranslating ? 'Active' : 'Idle'} />
    </span>
    <span>Mode: {mode === 'cloud' ? 'Cloud (Soniox)' : 'Offline (MLX)'}</span>
    <span>Audio: {audioCapturing ? 'Capturing' : 'Idle'}</span>
    <span>{statusMessage}</span>
  </div>
</div>

<style>
  .container {
    max-width: 1400px;
    margin: 0 auto;
    padding: 2rem;
    width: 100%;
  }

  header { margin-bottom: 1rem; }
  h1 { font-size: 2.2em; line-height: 1.1; margin-bottom: 0.5rem; }
  p { color: rgba(255, 255, 255, 0.6); }

  .error {
    margin: 1rem 0;
    padding: 0.75rem;
    color: #f44336;
    background-color: rgba(244, 67, 54, 0.1);
    border-radius: 6px;
  }

  .mode-selector {
    display: flex;
    gap: 1.5rem;
    margin: 1rem 0;
    padding: 0.75rem 1rem;
    background-color: rgba(255, 255, 255, 0.03);
    border-radius: 8px;
  }

  .mode-selector label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
    color: rgba(255, 255, 255, 0.8);
  }

  .mode-selector input[type="radio"] {
    accent-color: #646cff;
  }

  .settings {
    margin: 1rem 0;
    padding: 1rem;
    background-color: rgba(255, 255, 255, 0.03);
    border-radius: 8px;
  }

  .setting-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 0.5rem;
  }

  .setting-row:last-child { margin-bottom: 0; }

  label {
    font-weight: 500;
    color: rgba(255, 255, 255, 0.8);
    white-space: nowrap;
  }

  input[type="password"] {
    flex: 1;
    padding: 0.5rem;
    border-radius: 6px;
    border: 1px solid rgba(255, 255, 255, 0.2);
    background-color: rgba(255, 255, 255, 0.05);
    color: rgba(255, 255, 255, 0.9);
    font-size: 0.9rem;
  }

  .link {
    color: #646cff;
    text-decoration: none;
    font-size: 0.85rem;
  }

  .link:hover { text-decoration: underline; }

  select {
    padding: 0.5rem;
    border-radius: 6px;
    border: 1px solid rgba(255, 255, 255, 0.2);
    background-color: rgba(255, 255, 255, 0.05);
    color: rgba(255, 255, 255, 0.9);
    font-size: 0.9rem;
  }

  button {
    border-radius: 8px;
    border: 1px solid transparent;
    padding: 0.6em 1.2em;
    font-size: 1em;
    font-weight: 500;
    cursor: pointer;
  }

  button.primary { background-color: #646cff; color: white; }
  button.primary:hover:not(:disabled) { background-color: #535bf2; }
  button.danger { background-color: #f44336; color: white; }
  button.danger:hover:not(:disabled) { background-color: #d32f2f; }
  button:disabled { opacity: 0.5; cursor: not-allowed; }

  .status-bar {
    margin-top: 1.5rem;
    display: flex;
    gap: 2rem;
    font-size: 0.85rem;
    color: rgba(255, 255, 255, 0.6);
    align-items: center;
  }
</style>
```

**Step 2: Commit**

```bash
git add src/App.svelte
git commit -m "feat: dual-mode Svelte UI with cloud (Soniox) and offline (MLX) support"
```

---

### Task 7: Wire up main.rs and verify compilation

Update the Tauri app entry point to register all new commands and verify everything compiles.

**Files:**
- Modify: `src-tauri/src/main.rs`

**Step 1: Update main.rs**

Read the current `src-tauri/src/main.rs` and add all new commands to the `invoke_handler`:

```rust
// src-tauri/src/main.rs
// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(auralis_app_lib::AuralisState::new())
        .invoke_handler(tauri::generate_handler![
            // Existing commands
            auralis_app_lib::greet,
            auralis_app_lib::get_model_status,
            auralis_app_lib::check_model_exists,
            auralis_app_lib::get_model_resolution,
            auralis_app_lib::set_model_path,
            auralis_app_lib::download_model,
            auralis_app_lib::download_all_models,

            // New audio streaming commands
            auralis_app_lib::commands_audio::start_audio_capture,
            auralis_app_lib::commands_audio::stop_audio_capture,

            // New settings commands
            auralis_app_lib::commands_settings::get_settings,
            auralis_app_lib::commands_settings::save_settings,

            // New pipeline commands
            auralis_app_lib::commands_pipeline::start_local_pipeline,
            auralis_app_lib::commands_pipeline::stop_local_pipeline,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**Step 2: Verify compilation**

Run: `cd /Users/benq/Desktop/sentia-lab/auralis && cargo check`
Expected: Compiles successfully (may have warnings)

Fix any compilation errors before proceeding.

**Step 3: Commit**

```bash
git add src-tauri/src/main.rs src-tauri/src/lib.rs
git commit -m "feat: wire up dual-mode commands in Tauri app"
```

---

### Task 8: End-to-end testing with Soniox cloud mode

Test the cloud mode pipeline end-to-end with real audio and Soniox API.

**Step 1: Get a Soniox API key**

1. Go to https://soniox.com/
2. Sign up for a free account
3. Get an API key from the dashboard

**Step 2: Run the app**

Run: `cd /Users/benq/Desktop/sentia-lab/auralis && npm run tauri dev`

**Step 3: Test cloud mode**

1. Enter Soniox API key
2. Select English → Vietnamese
3. Click "Start"
4. Speak into the microphone
5. Verify transcriptions appear in the Source panel
6. Verify Vietnamese translations appear in the Translation panel
7. Verify latency is under 500ms (text appears quickly after speaking)

**Step 4: Test offline mode (if MLX is set up)**

First set up MLX environment:
```bash
python3 -m venv ~/.config/auralis/mlx-env
source ~/.config/auralis/mlx-env/bin/activate
pip install mlx-whisper transformers numpy
```

Then test:
1. Switch to "Offline" mode
2. Click "Start"
3. Speak into the microphone
4. Verify transcriptions and translations appear (with ~3s latency)

**Step 5: Commit any fixes**

```bash
git add -A
git commit -m "fix: address issues found during end-to-end testing"
```

---

## Summary of New Files

| File | Purpose |
|------|---------|
| `src-tauri/src/commands_audio.rs` | Audio capture streaming commands |
| `src-tauri/src/commands_settings.rs` | Settings load/save commands |
| `src-tauri/src/commands_pipeline.rs` | Python sidecar management |
| `src/js/soniox.ts` | Soniox WebSocket client |
| `scripts/local_pipeline.py` | Offline MLX Whisper + Opus-MT pipeline |

## Summary of Modified Files

| File | Change |
|------|--------|
| `src-tauri/src/state.rs` | Replace orchestrator state with streaming state + settings |
| `src-tauri/src/lib.rs` | Add new command module exports |
| `src-tauri/src/main.rs` | Register new commands |
| `src/App.svelte` | Dual-mode UI with mode selector, Soniox integration |
| `src-tauri/Cargo.toml` | Add once_cell dependency |

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                         Auralis App                         │
├─────────────────────────────────────────────────────────────┤
│  Svelte Frontend                                            │
│  ┌──────────┐  ┌──────────────────┐  ┌──────────────────┐  │
│  │ Mode     │  │ Soniox WS Client │  │ DualPanel UI     │  │
│  │ Selector │  │ (cloud mode)     │  │ (both modes)     │  │
│  └──────────┘  └──────────────────┘  └──────────────────┘  │
│        │               │                      ▲             │
│        │    PCM audio──┘        results────────┘             │
│        │               │                      ▲             │
├────────┼───────────────┼──────────────────────┼─────────────┤
│  Rust  │    Tauri Events (audio-data)         │             │
│        │               │         pipeline-result            │
│  ┌─────▼───────────────▼──────────────────────▼──────────┐  │
│  │  Audio Capture (cpal)                                 │  │
│  │  start_audio_capture / stop_audio_capture             │  │
│  └───────────────────────────────────────────────────────┘  │
│        │                                    ▲                │
│        │ PCM (offline mode)    JSON results │                │
│  ┌─────▼────────────────────────────────────▼────────────┐  │
│  │  Python Sidecar (offline mode only)                   │  │
│  │  MLX Whisper → Opus-MT                                │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Latency Expectations

| Mode | Component | Latency |
|------|-----------|---------|
| Cloud | Soniox integrated STT+translate | ~150-300ms |
| Offline | Whisper-large-v3-turbo (3s chunk) | ~300-500ms |
| Offline | Opus-MT translation | ~30-80ms |
| Offline | **Total E2E** | **~3.5-3.6s** |
