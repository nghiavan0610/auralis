# Audio Chunking VAD Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace fixed-window audio chunking with VAD-based endpoint detection that shows provisional text while speaking and finalizes translations on pauses.

**Architecture:** Rust-side energy VAD detects speech/silence transitions. While speaking, 1s chunks are sent to Python for provisional Whisper transcription. On silence pause (configurable endpoint delay), a flush signal triggers final transcription + translation. Frontend shows provisional text immediately and replaces with final results.

**Tech Stack:** Rust (cpal audio capture, VAD energy threshold), Python (MLX Whisper, Opus-MT), Svelte 5 (frontend), Tauri 2 events

---

### Task 1: Add `endpoint_delay` setting to Rust state

**Files:**
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/commands_settings.rs` (the `save_settings` and `load_settings` commands)

**Step 1:** Add `endpoint_delay` field to `Settings` struct in `src-tauri/src/state.rs`.

Add after the `max_lines` field (line 36):

```rust
/// Endpoint delay in seconds (0.5–3.0). How long silence must persist before finalizing a segment.
#[serde(default = "default_endpoint_delay")]
pub endpoint_delay: f64,
```

Add the default function after `default_max_lines`:

```rust
fn default_endpoint_delay() -> f64 {
    1.0
}
```

Add to `Default` impl:

```rust
endpoint_delay: default_endpoint_delay(),
```

Add getter to `AuralisState` impl:

```rust
/// Get the endpoint delay setting
pub async fn endpoint_delay(&self) -> f64 {
    self.settings.lock().await.endpoint_delay
}
```

**Step 2:** Update `save_settings` and `load_settings` in `src-tauri/src/commands_settings.rs` to include `endpoint_delay` in the settings JSON. Read the file first to see the current format.

**Step 3:** Run `cargo check` to verify.

**Step 4:** Commit.

---

### Task 2: Add endpoint delay slider to Settings UI

**Files:**
- Modify: `src/components/SettingsView.svelte`

**Step 1:** Add `endpointDelay` prop and local state.

In the props destructuring, add:
```typescript
endpointDelay = 1.0,
```

In the local state section, add:
```typescript
let localEndpointDelay = $state(1.0);
```

In the `$effect()` that syncs props, add:
```typescript
localEndpointDelay = endpointDelay;
```

**Step 2:** Add the slider in the Translation tab, after the Audio Source section (before the API key conditional block). Insert before `{#if localMode === 'cloud'}`:

```svelte
<!-- Endpoint delay -->
<div class="section-label" style="margin-top: var(--space-sm);">Endpoint Delay</div>
<p class="section-desc">How long to wait after silence before finalizing a segment. Lower = faster response, higher = more complete sentences.</p>
<div class="slider-row">
  <input
    type="range"
    min="5"
    max="30"
    step="1"
    value={Math.round(localEndpointDelay * 10)}
    oninput={() => localEndpointDelay = Number(Math.round(localEndpointDelay * 10) || 10) / 10}
    class="slider"
  />
  <span class="slider-value">{localEndpointDelay.toFixed(1)}s</span>
</div>
```

Wait — the slider binding is wrong. Use a separate variable like the opacity slider does. Add `let localEndpointTenths = $state(10);` and sync in the effect: `localEndpointTenths = Math.round(endpointDelay * 10);`. The slider binds to `localEndpointTenths` and updates `localEndpointDelay = localEndpointTenths / 10`.

**Step 3:** Add `endpoint_delay: localEndpointDelay` to `handleSave()`'s `onSave` callback object and the TypeScript type.

**Step 4:** In `src/App.svelte`, add `endpointDelay` to the settings state, the load/save handlers, and pass it to SettingsView as a prop.

**Step 5:** Verify with `npm run build`.

**Step 6:** Commit.

---

### Task 3: Add VAD to Rust mic capture loop

**Files:**
- Modify: `src-tauri/src/commands_pipeline.rs` (the `_ =>` "microphone" arm of the match, lines ~463–540)

**Step 1:** Add VAD state variables inside the `tokio::spawn` block, before the while loop.

After `let _capture = capture;` (line ~490), add:

```rust
// VAD state
let mut is_speaking = false;
let mut silence_start: Option<std::time::Instant> = None;
let mut speech_buffer: Vec<u8> = Vec::new();
let mut last_provisional_time = std::time::Instant::now();
let endpoint_delay_ms = (state.endpoint_delay().await * 1000.0) as u64;
```

Note: `endpoint_delay` must be read BEFORE the `tokio::spawn` since `State` can't be moved into the async block. Read it above and capture as a `u64`.

**Step 2:** Replace the simple "write pcm to stdin" logic inside the while loop with VAD-aware logic.

The current logic (lines ~500–530) drains pcm_bytes and writes to stdin. Replace with:

```rust
let pcm_bytes = {
    let mut data = audio_data.lock().unwrap_or_else(|e| e.into_inner());
    let chunks: Vec<Vec<f32>> = data.drain(..).collect();
    let mut all_samples = Vec::new();
    for chunk in chunks {
        all_samples.extend(chunk);
    }
    f32_to_pcm_s16le(&all_samples)
};

if pcm_bytes.is_empty() {
    continue;
}

// Compute RMS for VAD
let rms = compute_rms_pcm(&pcm_bytes);
let speech_threshold: f32 = 30.0;

if rms >= speech_threshold {
    // Speech detected
    speech_buffer.extend_from_slice(&pcm_bytes);

    if !is_speaking {
        is_speaking = true;
        silence_start = None;
    }

    // Send provisional chunk every ~1s of accumulated audio
    let one_second_bytes = 16000 * 2; // 16kHz * s16le
    if speech_buffer.len() >= one_second_bytes && last_provisional_time.elapsed() >= std::time::Duration::from_millis(800) {
        let write_result = {
            let mut guard = match PIPELINE.lock() {
                Ok(g) => g,
                Err(e) => {
                    tracing::error!("Pipeline mutex poisoned: {}", e);
                    break;
                }
            };
            match guard.as_mut() {
                Some(ps) => ps.stdin.write_all(&speech_buffer).and_then(|_| ps.stdin.flush()),
                None => break,
            }
        };
        if let Err(e) = write_result {
            tracing::error!("stdin write error: {}", e);
            break;
        }
        last_provisional_time = std::time::Instant::now();
    }
} else {
    // Silence detected
    if is_speaking {
        if silence_start.is_none() {
            silence_start = Some(std::time::Instant::now());
        }
        if let Some(start) = silence_start {
            if start.elapsed() >= std::time::Duration::from_millis(endpoint_delay_ms) {
                // Endpoint! Flush accumulated speech + send FLUSH marker
                is_speaking = false;
                silence_start = None;

                // Write remaining speech buffer
                if !speech_buffer.is_empty() {
                    let write_result = {
                        let mut guard = match PIPELINE.lock() {
                            Ok(g) => g,
                            Err(e) => {
                                tracing::error!("Pipeline mutex poisoned: {}", e);
                                break;
                            }
                        };
                        match guard.as_mut() {
                            Some(ps) => {
                                ps.stdin.write_all(&speech_buffer)?;
                                // Send FLUSH marker: 0xFF byte
                                ps.stdin.write_all(&[0xFF])?;
                                ps.stdin.flush()
                            }
                            None => std::io::Result::Ok(()),
                        }
                    };
                    if let Err(e) = write_result {
                        tracing::error!("flush write error: {}", e);
                        break;
                    }
                    speech_buffer.clear();
                }
            }
        }
    }
}
```

Note: the `?` operator won't work in the match arm. Use `and_then` chains instead, like the existing code.

**Step 3:** Add the `compute_rms_pcm` helper function at the top of the file (near the existing `f32_to_pcm_s16le`):

```rust
/// Compute RMS of PCM s16le bytes for voice activity detection.
fn compute_rms_pcm(pcm_bytes: &[u8]) -> f32 {
    let samples: Vec<i16> = pcm_bytes
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f64 = samples.iter().map(|&s| (s as f64) * (s as f64)).sum();
    (sum_sq / samples.len() as f64).sqrt() as f32
}
```

**Step 4:** Also apply the same VAD pattern to the "both" arm (lines ~347–456) and the "system" arm (lines ~286–346). The "both" arm has the same mic + system audio logic. The system arm doesn't use mic data so VAD doesn't apply there — leave it as-is.

For the "both" arm: apply the same VAD to the mic portion. The system audio portion can bypass VAD (always pass through).

**Step 5:** Run `cargo check` and fix any compilation errors.

**Step 6:** Commit.

---

### Task 4: Update Python pipeline to handle FLUSH signal and emit provisional results

**Files:**
- Modify: `scripts/local_pipeline.py`

**Step 1:** Update the stdin protocol docstring at the top of the file. Add:

```
Flush signal: 0xFF byte signals end of utterance. Python reads stdin in binary,
checks for 0xFF marker, and finalizes the current segment on flush.
```

**Step 2:** Update `_stdin_reader` to detect the `0xFF` flush marker.

Replace the `_stdin_reader` method (lines 397–409) with:

```python
def _stdin_reader(self) -> None:
    """Continuously read PCM bytes from stdin. Detect 0xFF flush marker."""
    try:
        while self.running:
            data = sys.stdin.buffer.read(4096)
            if not data:
                break  # EOF
            # Split on 0xFF flush marker
            parts = data.split(b'\xff')
            for i, part in enumerate(parts):
                if part:
                    with self.lock:
                        self.audio_buffer.extend(part)
                if i < len(parts) - 1:
                    # A flush marker was found
                    with self.lock:
                        self.flush_requested = True
    except Exception as exc:
        log(f"stdin reader error: {exc}")
    finally:
        self.running = False
```

Add `self.flush_requested = False` to `__init__` (after `self.running = True`).

**Step 3:** Update `run()` main loop to handle provisional chunks and flushes.

Replace the `run()` method (lines 415–449) with:

```python
def run(self) -> None:
    reader = threading.Thread(target=self._stdin_reader, daemon=True)
    reader.start()

    processed_pos = 0
    provisional_bytes = SAMPLE_RATE * 1 * SAMPLE_WIDTH  # 1 second of audio

    while self.running:
        time.sleep(0.3)

        with self.lock:
            available = len(self.audio_buffer) - processed_pos
            do_flush = self.flush_requested
            if do_flush:
                self.flush_requested = False

        # On flush: process all remaining audio as a final segment
        if do_flush:
            with self.lock:
                remaining = len(self.audio_buffer) - processed_pos
                if remaining > SAMPLE_RATE * SAMPLE_WIDTH * 0.5:  # at least 0.5s
                    chunk = bytes(self.audio_buffer[processed_pos:])
                    processed_pos = len(self.audio_buffer)
                    self._process_final(chunk)
            continue

        # While speaking: send provisional chunks
        if available >= provisional_bytes:
            with self.lock:
                chunk = bytes(
                    self.audio_buffer[processed_pos : processed_pos + provisional_bytes]
                )
            processed_pos += provisional_bytes
            self._process_provisional(chunk)

    # Drain remaining
    with self.lock:
        remaining = len(self.audio_buffer) - processed_pos
        if remaining > SAMPLE_RATE * SAMPLE_WIDTH:
            chunk = bytes(self.audio_buffer[processed_pos:])
            self._process_final(chunk)

    emit({"type": "done"})
    log("Pipeline stopped.")
```

**Step 4:** Add `_process_provisional` method. This does a quick Whisper transcription and emits provisional text WITHOUT translation:

```python
def _process_provisional(self, pcm_bytes: bytes) -> None:
    """Quick transcription for provisional display (no translation)."""
    rms = self._compute_rms(pcm_bytes)
    if rms < SILENCE_THRESHOLD:
        return

    wav_path = self._save_pcm_as_wav(pcm_bytes)
    try:
        transcript, detected_lang = self._transcribe(wav_path)
        if not transcript:
            return
        emit({
            "type": "provisional",
            "text": transcript,
            "source_lang": detected_lang,
        })
    finally:
        try:
            os.unlink(wav_path)
        except OSError:
            pass
```

**Step 5:** Rename `_process_chunk` to `_process_final` and keep it as-is (it already does full transcription + translation + dedup). Just rename the method.

**Step 6:** Commit.

---

### Task 5: Handle provisional events in frontend

**Files:**
- Modify: `src/App.svelte`

**Step 1:** In the `pipeline-result` event listener (around line 425), add handling for `type === "provisional"`.

After `if (data.type === 'result') { ... }`, add an else-if:

```typescript
} else if (data.type === 'provisional') {
    // Show provisional text immediately (no translation yet)
    provisionalText = data.text ?? '';
    provisionalLang = data.source_lang ?? '';
}
```

This reuses the existing `provisionalText` state variable that the Transcript component already renders.

**Step 2:** Clear provisional text when a final result arrives. In the same `data.type === 'result'` block, add at the end:

```typescript
provisionalText = '';
provisionalLang = '';
```

**Step 3:** Verify with `npm run build`.

**Step 4:** Commit.

---

### Task 6: Integration test

**Step 1:** Stop any running dev server.

**Step 2:** Run `npm run tauri dev` and verify:
- Settings shows Endpoint Delay slider (0.5s–3.0s, default 1.0s)
- Save settings works (check `cargo check` logs for `endpoint_delay`)
- Start offline recording, speak at normal volume
- Provisional text appears in left column while speaking (gray/dim)
- After ~1s pause, final result appears with translation
- Endpoint delay slider changes behavior (0.5s = quick cut, 3.0s = waits longer)

**Step 3:** Commit final state.

---

### Execution Order

1. Task 1 (state.rs setting) — foundation
2. Task 2 (settings UI) — depends on Task 1
3. Task 3 (Rust VAD) — depends on Task 1
4. Task 4 (Python pipeline) — independent of Tasks 2-3
5. Task 5 (frontend) — depends on Task 4
6. Task 6 (integration test) — depends on all

Tasks 3 and 4 can be done in parallel. Tasks 2 and 4 can be done in parallel.
