# Cross-Platform TTS Design

**Goal:** Replace macOS-only `say` TTS with a cross-platform system supporting both macOS and Windows, using a 4-tier provider architecture.

**Architecture:** Frontend-only TTS engine using Web Speech API as default, with future cloud provider upgrades. All TTS logic lives in `src/js/tts.ts` — no Rust TTS commands needed.

---

## Provider Stack

| Provider | Type | Quality | Cost | Languages | vi/hi | Latency |
|----------|------|---------|------|-----------|-------|---------|
| Web Speech API | Offline (built-in) | Decent | Free | System-dependent | Partial | ~0ms |
| Edge TTS | Cloud (free) | Good | Free (no key) | 40+ | Yes | ~200ms |
| Google Cloud TTS | Cloud (freemium) | Great | 1M chars/mo free | 40+ | Yes | ~300ms |
| ElevenLabs | Cloud (premium) | Best | 10k chars/mo free | 29 | No | ~250ms |

---

## Phase 1: Web Speech API (this implementation)

### What changes

**Remove (Rust backend):**
- Delete `src-tauri/src/commands_tts.rs`
- Remove `tts_process` from `AuralisState`
- Remove `speak_text`, `stop_tts`, `list_tts_voices` from `main.rs` invoke_handler

**Add (frontend):**
- New `src/js/tts.ts` — TTSEngine class wrapping `window.speechSynthesis`
- Voice selection dropdown in TTS settings tab
- Speed slider (0.5x–2.0x) in TTS settings tab

**Modify:**
- `src/App.svelte` — replace `invoke('speak_text')` with `tts.speak()`
- `src-tauri/src/state.rs` — add `tts_voice` and `tts_rate` to Settings, remove `tts_process` from AuralisState
- `src-tauri/src/main.rs` — remove TTS command registrations

### TTSEngine API (`src/js/tts.ts`)

```typescript
class TTSEngine {
  speak(text: string, lang: string): void    // speak with best voice for lang
  stop(): void                                // cancel current speech
  getVoices(lang?: string): Voice[]           // list available voices
  setRate(rate: number): void                 // 0.5–2.0 speed
}
```

- Auto-picks the best voice for the target language
- Interrupts previous speech when new text arrives
- Skips silently if no voice available for the language

### Settings (persisted via existing backend)

- `tts_enabled: boolean` — already exists
- `tts_voice: string` — selected voice name (new)
- `tts_rate: number` — speech speed 0.5–2.0, default 1.0 (new)

### TTS Settings Tab UI

- Toggle: Enable/disable TTS (exists)
- Voice selector: dropdown filtered by target language
- Speed slider: 0.5x – 2.0x (card-style, matching Display tab design)
- Language support info

---

## Phase 2: Edge TTS (future)

- Free cloud TTS, no API key required
- 40+ languages including vi/hi
- Streaming audio via HTTP endpoint
- Provider selector in TTS settings

## Phase 3: Google Cloud + ElevenLabs (future)

- Cloud TTS provider selection in settings
- API key input per provider
- Premium voice selection
- Auto-fallback to Web Speech API when offline

---

## Scope

**Phase 1 included:**
- Web Speech API TTS engine (frontend only)
- Voice selection per language
- Speed control
- Remove macOS `say` backend
- Cross-platform (macOS + Windows)

**Phase 1 not included:**
- Edge TTS, Google Cloud TTS, ElevenLabs (Phase 2–3)
- Click-to-replay transcript segments
- Piper TTS (dropped — 4 providers cover all needs)
