# Audio Chunking & VAD Design

**Date:** 2026-04-08
**Status:** Approved

## Problem

Current pipeline uses fixed 2s windows with RMS silence gating. This causes:
- Must wait full 2s before any text appears
- No sentence boundary awareness — text can be cut mid-sentence
- User must speak loudly to pass the high RMS threshold
- No provisional/intermediate feedback while speaking

## Design: Sliding Window + Silence-based Endpoints

### Architecture

```
Mic audio → Rust VAD
              │
              ├─ Speech detected → accumulate buffer, every 1s send chunk to Python
              │                                     Python: Whisper → emit "provisional" result
              │                                     Frontend: show gray text immediately
              │
              └─ Silence for Xms (endpoint delay) → send FLUSH to Python
                                                     Python: finalize segment → translate → emit "final" result
                                                     Frontend: replace gray text with colored translation
```

### 1. Rust-side Voice Activity Detection (VAD)

Location: `commands_pipeline.rs` audio capture loop.

**Current behavior:** Drains mic data every 200ms, writes raw PCM to Python stdin.

**New behavior:**
- Compute RMS on each 200ms audio drain
- Track speech state: `is_speaking` flag + `silence_start` timestamp
- **Speech start:** RMS rises above threshold → `is_speaking = true`
- **While speaking:** Accumulate audio in a local buffer. Every ~1s of accumulated audio, write chunk to Python stdin
- **Speech end:** RMS stays below threshold for `endpoint_delay` ms → `is_speaking = false`, send flush signal to Python, send accumulated buffer
- **Flush signal:** Send a single `0xFF` byte followed by 4-byte little-endian length of 0 to Python stdin, signaling end of utterance. Python reads stdin in binary mode and checks for this marker.

**Settings:**
- `endpoint_delay`: float, 0.5–3.0s (default 1.0s) — stored in `Settings` in `state.rs`
- Silence threshold stays hardcoded at a reasonable level (~30 RMS)

### 2. Python Pipeline Changes

Location: `scripts/local_pipeline.py`

**Two modes of operation:**

**On audio chunk (while speaking):**
- Run Whisper on the chunk
- Emit provisional event:
  ```json
  {"type": "provisional", "text": "Hello, how are", "source_lang": "en"}
  ```
- No translation yet for speed

**On flush signal:**
- Take full accumulated audio since last flush
- Run Whisper for final transcription
- Deduplicate against previous final text
- Detect language (two-way mode)
- Translate
- Emit final result:
  ```json
  {"type": "result", "original": "Hello, how are you?", "translated": "Xin chào, bạn khỏe không?", "source_lang": "en", "target_lang": "vi"}
  ```
- Clear provisional state

**Stdin protocol change:**
- Binary PCM bytes as before
- `0xFF` byte = flush signal (Python reads this specially)

**Chunk timing:**
- Provisional chunks: every ~1s of speech
- Final: on flush (silence detected by Rust VAD)

### 3. Frontend Changes

Location: `src/App.svelte`

**Handle provisional events:**
- Listen for `pipeline-result` events where `type === "provisional"`
- Display provisional text in a temporary segment (dim/gray styling, typing indicator)
- When `type === "result"` arrives, replace provisional with finalized segment (original + translation)

**Transcript component:**
- Already supports `provisionalText` prop — reuse this for the VAD provisional text
- Provisional text shows with typing indicator in left column only

### 4. Settings: Endpoint Delay

Location: `SettingsView.svelte` > Translation tab

**New control:**
- Slider: "Endpoint Delay" — range 0.5s to 3.0s, step 0.1s, default 1.0s
- Description: "How long to wait after silence before finalizing a segment. Lower = faster but may cut sentences short."
- Saved as `endpoint_delay` in settings, passed through to Rust VAD

**State changes:**
- `state.rs`: Add `endpoint_delay: f64` field to `Settings` (default 1.0)
- `commands_pipeline.rs`: Read `endpoint_delay` from state, use in VAD silence timing

### Scope Exclusions

- No WebRTC VAD or neural VAD — simple energy threshold is sufficient
- No audio preprocessing (noise reduction, echo cancellation)
- No speaker diarization beyond language-based detection
- Cloud mode (Soniox) already has its own chunking — not affected
