# Cross-Platform TTS Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace macOS-only `say` TTS with cross-platform Web Speech API, moving all TTS logic to the frontend.

**Architecture:** Frontend-only TTS engine in `src/js/tts.ts` using `window.speechSynthesis`. Backend persists `tts_voice` and `tts_rate` settings but does no TTS processing. The old `commands_tts.rs` and all `say`-based Rust code is deleted.

**Tech Stack:** Web Speech API (`window.speechSynthesis`), Svelte 5 runes, existing Rust settings persistence

---

### Task 1: Update backend Settings — add tts_voice, tts_rate; remove tts_process

**Files:**
- Modify: `src-tauri/src/state.rs`

**Step 1: Add new settings fields**

In `src-tauri/src/state.rs`, add to `Settings` struct (after `tts_enabled`):

```rust
/// Selected TTS voice name (empty = auto-detect)
#[serde(default)]
pub tts_voice: String,
/// TTS speech rate (0.5–2.0, default 1.0)
#[serde(default = "default_tts_rate")]
pub tts_rate: f64,
```

Add the default function (after `default_tts_enabled`):

```rust
fn default_tts_rate() -> f64 {
    1.0
}
```

Add to `Default` impl:

```rust
tts_voice: String::new(),
tts_rate: default_tts_rate(),
```

**Step 2: Remove tts_process from AuralisState**

Remove from `AuralisState` struct:
```rust
/// Currently running TTS process (if any)
pub tts_process: Arc<std::sync::Mutex<Option<std::process::Child>>>,
```

Remove from `AuralisState::new()`:
```rust
tts_process: Arc::new(std::sync::Mutex::new(None)),
```

Remove the `use std::process::Child` import if no longer needed (check `PipelineState` still uses it — it does via `Child` and `ChildStdin`, so keep `std::process::{Child, ChildStdin}`).

**Step 3: Run tests**

Run: `cd src-tauri && cargo test`
Expected: All tests pass. New fields have `#[serde(default)]` so backward-compatible.

**Step 4: Commit**

```bash
git add src-tauri/src/state.rs
git commit -m "refactor: add tts_voice/tts_rate settings, remove tts_process"
```

---

### Task 2: Delete commands_tts.rs and clean up main.rs

**Files:**
- Delete: `src-tauri/src/commands_tts.rs`
- Modify: `src-tauri/src/main.rs`

**Step 1: Delete the TTS commands module**

Delete the file: `src-tauri/src/commands_tts.rs`

**Step 2: Remove from main.rs**

In `src-tauri/src/main.rs`:

Remove the module declaration:
```rust
mod commands_tts;
```

Remove the import:
```rust
use commands_tts::*;
```

Remove from `invoke_handler`:
```rust
// TTS
speak_text,
stop_tts,
list_tts_voices,
```

**Step 3: Run tests and build**

Run: `cd src-tauri && cargo test && cargo build`
Expected: All tests pass, build succeeds.

**Step 4: Commit**

```bash
git add -A src-tauri/
git commit -m "refactor: remove macOS say TTS backend (commands_tts.rs)"
```

---

### Task 3: Create frontend TTSEngine module

**Files:**
- Create: `src/js/tts.ts`

**Step 1: Create tts.ts**

Create `src/js/tts.ts` with the following content:

```typescript
/**
 * Cross-platform TTS engine using Web Speech API.
 *
 * Provides speech synthesis using the browser's built-in voices.
 * Works on macOS, Windows, and Linux via Tauri's WebView.
 */

export interface TTSVoice {
  name: string;
  lang: string;
  local: boolean;
}

class TTSEngine {
  private synth: SpeechSynthesis | null = null;
  private currentUtterance: SpeechSynthesisUtterance | null = null;
  private _rate: number = 1.0;
  private _voice: string = ''; // empty = auto
  private voices: SpeechSynthesisVoice[] = [];
  private voicesLoaded: Promise<void>;

  constructor() {
    this.synth = window.speechSynthesis ?? null;

    // Voices load asynchronously on some browsers
    this.voicesLoaded = new Promise((resolve) => {
      if (!this.synth) {
        resolve();
        return;
      }

      const loaded = this.synth.getVoices();
      if (loaded.length > 0) {
        this.voices = loaded;
        resolve();
        return;
      }

      const handler = () => {
        this.voices = this.synth!.getVoices();
        resolve();
      };
      this.synth.addEventListener('voiceschanged', handler, { once: true });

      // Safety timeout — some platforms never fire voiceschanged
      setTimeout(resolve, 2000);
    });
  }

  /** Speak text aloud. Interrupts any current speech. */
  async speak(text: string, lang: string): Promise<void> {
    if (!this.synth || !text.trim()) return;

    await this.voicesLoaded;
    this.stop();

    const utterance = new SpeechSynthesisUtterance(text);

    // Set rate
    utterance.rate = this._rate;

    // Pick voice: user preference > best match for language > default
    const voice = this.pickVoice(lang);
    if (voice) {
      utterance.voice = voice;
    } else {
      // Fallback: set just the lang attribute
      utterance.lang = lang;
    }

    this.currentUtterance = utterance;
    this.synth.speak(utterance);
  }

  /** Stop any currently playing speech. */
  stop(): void {
    if (this.synth) {
      this.synth.cancel();
    }
    this.currentUtterance = null;
  }

  /** Get available voices, optionally filtered by language code prefix. */
  async getVoices(lang?: string): Promise<TTSVoice[]> {
    await this.voicesLoaded;

    let filtered = this.voices;
    if (lang) {
      const prefix = lang.toLowerCase();
      filtered = this.voices.filter((v) =>
        v.lang.toLowerCase().startsWith(prefix)
      );
    }

    return filtered.map((v) => ({
      name: v.name,
      lang: v.lang,
      local: v.localService,
    }));
  }

  /** Set speech rate (0.5–2.0). */
  setRate(rate: number): void {
    this._rate = Math.max(0.5, Math.min(2.0, rate));
  }

  /** Get current rate. */
  get rate(): number {
    return this._rate;
  }

  /** Set preferred voice by name. Empty string = auto-detect. */
  setVoice(name: string): void {
    this._voice = name;
  }

  /** Get current voice preference. */
  get voice(): string {
    return this._voice;
  }

  /** Pick the best voice for a given language. */
  private pickVoice(lang: string): SpeechSynthesisVoice | null {
    const prefix = lang.toLowerCase();

    // 1. User-selected voice that matches the language
    if (this._voice) {
      const match = this.voices.find(
        (v) => v.name === this._voice && v.lang.toLowerCase().startsWith(prefix)
      );
      if (match) return match;
    }

    // 2. Local voice for the language
    const local = this.voices.find(
      (v) => v.localService && v.lang.toLowerCase().startsWith(prefix)
    );
    if (local) return local;

    // 3. Any voice for the language
    const any = this.voices.find((v) =>
      v.lang.toLowerCase().startsWith(prefix)
    );
    return any ?? null;
  }
}

// Singleton instance
export const tts = new TTSEngine();
```

**Step 2: Verify no type errors**

Run: `npx svelte-check --threshold error`
Expected: No errors in tts.ts

**Step 3: Commit**

```bash
git add src/js/tts.ts
git commit -m "feat: add frontend TTS engine using Web Speech API"
```

---

### Task 4: Wire TTSEngine into App.svelte

**Files:**
- Modify: `src/App.svelte`

**Step 1: Import tts engine and add new state**

At the top of `src/App.svelte`, add import:

```typescript
import { tts } from './js/tts';
```

Add new state variables (after `let ttsEnabled = $state(false);`):

```typescript
let ttsVoice = $state('');
let ttsRate = $state(1.0);
```

**Step 2: Replace speakTranslation function**

Replace the existing `speakTranslation` function (around line 387):

```typescript
// OLD:
async function speakTranslation(text: string, targetLang: string): Promise<void> {
    if (!ttsEnabled || !text.trim()) return;
    try {
      await invoke('speak_text', { text, language: targetLang });
    } catch (err) {
      console.warn('[Auralis] TTS failed:', err);
    }
  }

// NEW:
function speakTranslation(text: string, targetLang: string): void {
    if (!ttsEnabled || !text.trim()) return;
    tts.speak(text, targetLang);
  }
```

**Step 3: Replace stop_tts in handleStop**

In `handleStop()`, replace:

```typescript
// OLD:
try { await invoke('stop_tts'); } catch {}

// NEW:
tts.stop();
```

**Step 4: Sync tts engine settings with state**

Add an effect to sync voice/rate to the engine (after the existing opacity/font effect):

```typescript
$effect(() => {
    tts.setVoice(ttsVoice);
    tts.setRate(ttsRate);
  });
```

**Step 5: Update loadSettings to load new fields**

In `loadSettings()`, update the settings type to include:
```typescript
tts_voice?: string;
tts_rate?: number;
```

After `ttsEnabled = settings.tts_enabled as boolean;` add:
```typescript
ttsVoice = settings.tts_voice ?? '';
ttsRate = settings.tts_rate ?? 1.0;
```

**Step 6: Update persistSettings to save new fields**

In `persistSettings()`, add to the settings object:
```typescript
tts_voice: ttsVoice,
tts_rate: ttsRate,
```

**Step 7: Update handleSettingsSave**

In `handleSettingsSave()`, update the parameter type to include:
```typescript
tts_voice: string;
tts_rate: number;
```

Add to the function body:
```typescript
ttsVoice = settings.tts_voice;
ttsRate = settings.tts_rate;
```

**Step 8: Update SettingsView props**

Update the `SettingsView` usage to pass new props:
```svelte
ttsVoice={ttsVoice}
ttsRate={ttsRate}
```

**Step 9: Verify**

Run: `npx svelte-check --threshold error`
Expected: No errors in App.svelte

**Step 10: Commit**

```bash
git add src/App.svelte
git commit -m "feat: wire Web Speech TTS engine into App.svelte"
```

---

### Task 5: Update SettingsView — voice selector + speed slider in TTS tab

**Files:**
- Modify: `src/components/SettingsView.svelte`

**Step 1: Add new props**

Add to the props destructuring (after `ttsEnabled = false`):
```typescript
ttsVoice = '',
ttsRate = 1.0,
```

Add to the props type:
```typescript
ttsVoice?: string;
ttsRate?: number;
```

Add to `onSave` callback type:
```typescript
tts_voice: string;
tts_rate: number;
```

**Step 2: Add local state**

Add local state variables (after `let localTtsEnabled = $state(false);`):
```typescript
let localTtsVoice = $state('');
let localTtsRate = $state(1.0);
let localTtsRateTenths = $state(10);
let availableVoices: Array<{ name: string; lang: string; local: boolean }> = $state([]);
```

**Step 3: Import tts and load voices**

Add import at top:
```typescript
import { tts } from '../js/tts';
```

Add to the `$effect` that syncs props:
```typescript
localTtsVoice = ttsVoice;
localTtsRate = ttsRate;
localTtsRateTenths = Math.round(ttsRate * 10);
```

Add a new effect to load voices when TTS tab is active:
```typescript
$effect(() => {
    if (activeTab === 'tts') {
      tts.getVoices(localTarget).then((v) => {
        availableVoices = v;
      });
    }
  });
```

**Step 4: Update handleSave**

Add to `handleSave()`:
```typescript
tts_voice: localTtsVoice,
tts_rate: localTtsRate,
```

**Step 5: Update TTS tab content**

Replace the TTS tab content (the `{:else if activeTab === 'tts'}` block) with:

```svelte
{:else if activeTab === 'tts'}
      <div class="settings-section">
        <!-- Toggle card -->
        <div class="toggle-card">
          <div class="toggle-card-info">
            <span class="toggle-card-label">Speak translations aloud</span>
            <span class="toggle-card-desc">Uses your system's built-in voices. Works offline.</span>
          </div>
          <button
            class="toggle"
            class:active={localTtsEnabled}
            onclick={() => localTtsEnabled = !localTtsEnabled}
            disabled={isTranslating}
            aria-label="Toggle text-to-speech"
          >
            <span class="toggle-thumb"></span>
          </button>
        </div>

        <!-- Voice selector -->
        <div class="slider-card">
          <div class="slider-card-header">
            <span class="slider-card-label">Voice</span>
          </div>
          <select bind:value={localTtsVoice} disabled={isTranslating || !localTtsEnabled}>
            <option value="">Auto (best for language)</option>
            {#each availableVoices as voice}
              <option value={voice.name}>{voice.name} ({voice.lang}){voice.local ? '' : ' ☁'}</option>
            {/each}
          </select>
        </div>

        <!-- Speed slider -->
        <div class="slider-card">
          <div class="slider-card-header">
            <span class="slider-card-label">Speed</span>
            <span class="slider-card-value">{localTtsRate.toFixed(1)}x</span>
          </div>
          <input
            type="range"
            min="5"
            max="20"
            step="1"
            bind:value={localTtsRateTenths}
            oninput={() => localTtsRate = localTtsRateTenths / 10}
            class="slider"
            style="--fill: {((localTtsRateTenths - 5) / 15) * 100}%"
          />
        </div>
      </div>
```

**Step 6: Add toggle-card CSS**

Add to the `<style>` section:

```css
/* Toggle card */
.toggle-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-md);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
  transition: border-color 0.2s ease;
}

.toggle-card:hover {
  border-color: var(--border-hover);
}

.toggle-card-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.toggle-card-label {
  font-size: var(--font-size-sm);
  font-weight: 500;
  color: var(--text-primary);
}

.toggle-card-desc {
  font-size: var(--font-size-xs);
  color: var(--text-dim);
}
```

**Step 7: Verify**

Run: `npx svelte-check --threshold error`
Expected: No errors in SettingsView.svelte

**Step 8: Commit**

```bash
git add src/components/SettingsView.svelte
git commit -m "feat: TTS settings — voice selector + speed slider"
```

---

### Task 6: Final cleanup and verification

**Step 1: Remove tts_process from state.rs if not already done**

Verify `tts_process` is fully removed from `src-tauri/src/state.rs`.

**Step 2: Verify Rust build**

Run: `cd src-tauri && cargo test`
Expected: All tests pass, no warnings about unused TTS code.

**Step 3: Verify frontend build**

Run: `npx svelte-check --threshold error`
Expected: Only pre-existing ModelDownloader error, no new errors.

**Step 4: Run dev server for visual verification**

Run: `npm run tauri dev`

Test:
1. Settings → TTS tab → toggle shows, voice selector populates, speed slider works
2. Enable TTS → start recording → translated text is spoken aloud
3. New translation interrupts previous speech
4. Stop recording → speech stops
5. Disable TTS → no speech plays
6. Voice selector shows voices filtered by target language
7. Speed slider changes speech rate

**Step 5: Commit any remaining fixes**

```bash
git add -A
git commit -m "feat: cross-platform TTS via Web Speech API — Phase 1 complete"
```
