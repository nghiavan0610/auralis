# macOS Native TTS Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add Text-to-Speech to Auralis that automatically speaks translated text using macOS's built-in `say` command.

**Architecture:** The frontend calls a `speak_text` Tauri command whenever a translated segment arrives. The backend spawns macOS's `say` command as a child process, killing any previous speech before starting a new one. A `tts_enabled` setting (toggle in Display settings) controls whether speech is active. Voice discovery queries `say -v '?'` at startup to map languages to available macOS voices.

**Tech Stack:** Rust `std::process::Command` (calling macOS `say`), Tauri 2 commands/events, Svelte 5 settings UI

---

### Task 1: Add TTS Settings to Backend State

**Files:**
- Modify: `src-tauri/src/state.rs`

**Step 1: Add `tts_enabled` field to `Settings` struct**

In `src-tauri/src/state.rs`, add a new field to `Settings` (after `endpoint_delay`):

```rust
/// Whether TTS is enabled (speak translated text aloud)
#[serde(default = "default_tts_enabled")]
pub tts_enabled: bool,
```

**Step 2: Add default function**

Add after `fn default_endpoint_delay()`:

```rust
fn default_tts_enabled() -> bool {
    false
}
```

**Step 3: Add to `Default` impl**

In the `Default` impl for `Settings`, add:

```rust
tts_enabled: default_tts_enabled(),
```

**Step 4: Run tests**

Run: `cd src-tauri && cargo test`
Expected: All existing tests pass (new field has `#[serde(default)]` so it's backward-compatible with existing settings files).

**Step 5: Commit**

```bash
git add src-tauri/src/state.rs
git commit -m "feat: add tts_enabled setting to backend state"
```

---

### Task 2: Create TTS Command Module

**Files:**
- Create: `src-tauri/src/commands_tts.rs`
- Modify: `src-tauri/src/main.rs`

**Step 1: Create `commands_tts.rs`**

Create `src-tauri/src/commands_tts.rs` with the following content:

```rust
//! Tauri commands for macOS native Text-to-Speech
//!
//! Uses the macOS `say` command to speak translated text aloud.
//! Supports voice discovery (mapping language codes to macOS voices)
//! and interrupting previous speech when new text arrives.

use crate::state::AuralisState;
use std::process::Command;
use std::sync::Arc;
use tauri::State;

// ---------------------------------------------------------------------------
// Language → macOS voice mapping
// ---------------------------------------------------------------------------

/// Preferred macOS voice names for each supported language.
/// These are standard voices available on macOS 12+ (Monterey).
/// If a voice is not installed, `say` will fail silently.
const LANG_VOICES: &[(&str, &str)] = &[
    ("en", "Samantha"),
    ("es", "Monica"),
    ("fr", "Thomas"),
    ("de", "Anna"),
    ("zh", "Ting-Ting"),
    ("ja", "Kyoko"),
    ("ko", "Yuna"),
    ("pt", "Luciana"),
    ("ru", "Milena"),
    ("ar", "Maged"),
    // Note: Vietnamese (vi) and Hindi (hi) have no standard macOS voices.
    // TTS will be skipped for these languages until Piper TTS is added.
];

/// Find the best macOS voice for a given language code.
/// Returns `None` if no voice is available for that language.
fn voice_for_language(lang: &str) -> Option<&'static str> {
    LANG_VOICES
        .iter()
        .find(|(code, _)| *code == lang)
        .map(|(_, voice)| *voice)
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// Speak the given text using macOS `say` command.
///
/// If a previous speech is in progress, it is stopped first.
/// If no voice is available for the language, does nothing.
#[tauri::command]
pub async fn speak_text(
    state: State<'_, AuralisState>,
    text: String,
    language: String,
) -> Result<(), String> {
    // Check if TTS is enabled
    {
        let settings = state.settings.lock().await;
        if !settings.tts_enabled {
            return Ok(());
        }
    }

    if text.trim().is_empty() {
        return Ok(());
    }

    let voice = voice_for_language(&language);
    if voice.is_none() {
        tracing::debug!("No macOS voice available for language: {}", language);
        return Ok(());
    }
    let voice = voice.unwrap();

    // Stop any previous speech
    {
        let mut guard = state.tts_process.lock().map_err(|e| e.to_string())?;
        if let Some(ref mut child) = *guard {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    // Spawn new `say` command
    tracing::info!("TTS: speaking {} chars in {} (voice: {})", text.len(), language, voice);

    let child = Command::new("say")
        .arg("-v")
        .arg(voice)
        .arg(&text)
        .spawn()
        .map_err(|e| format!("Failed to start TTS: {}", e))?;

    // Store child process so we can kill it if interrupted
    {
        let mut guard = state.tts_process.lock().map_err(|e| e.to_string())?;
        *guard = Some(child);
    }

    // Spawn a background thread to clean up the child when it finishes
    let tts_process = state.tts_process.clone();
    std::thread::spawn(move || {
        let mut guard = tts_process.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(ref mut child) = *guard {
            let _ = child.wait();
            // Child finished — clear it
            *guard = None;
        }
    });

    Ok(())
}

/// Stop any currently playing TTS speech.
#[tauri::command]
pub async fn stop_tts(
    state: State<'_, AuralisState>,
) -> Result<(), String> {
    let mut guard = state.tts_process.lock().map_err(|e| e.to_string())?;
    if let Some(ref mut child) = *guard {
        let _ = child.kill();
        let _ = child.wait();
        *guard = None;
    }
    Ok(())
}

/// Discover available macOS TTS voices.
///
/// Returns a list of { language, voice_name } objects for all
/// voices installed on this system.
#[tauri::command]
pub async fn list_tts_voices() -> Result<Vec<serde_json::Value>, String> {
    let output = Command::new("say")
        .arg("-v")
        .arg("?")
        .output()
        .map_err(|e| format!("Failed to list voices: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut voices = Vec::new();

    for line in stdout.lines() {
        // Format: "en-US		Samantha		# Hello, my name is Samantha."
        let parts: Vec<&str> = line.splitn(3, char::is_whitespace).collect();
        if parts.len() >= 2 {
            let locale = parts[0].trim();
            let voice_name = parts[1].trim();
            if !locale.is_empty() && !voice_name.is_empty() {
                // Extract language code from locale (e.g., "en-US" → "en")
                let lang = locale.split('-').next().unwrap_or(locale);
                voices.push(serde_json::json!({
                    "language": lang,
                    "locale": locale,
                    "voice_name": voice_name,
                }));
            }
        }
    }

    Ok(voices)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voice_for_language_known() {
        assert_eq!(voice_for_language("en"), Some("Samantha"));
        assert_eq!(voice_for_language("ja"), Some("Kyoko"));
        assert_eq!(voice_for_language("zh"), Some("Ting-Ting"));
    }

    #[test]
    fn test_voice_for_language_unknown() {
        assert_eq!(voice_for_language("vi"), None);
        assert_eq!(voice_for_language("hi"), None);
    }

    #[test]
    fn test_voice_for_language_not_in_list() {
        assert_eq!(voice_for_language("xx"), None);
    }
}
```

**Step 2: Add `tts_process` field to `AuralisState`**

In `src-tauri/src/state.rs`, add to `AuralisState`:

```rust
pub struct AuralisState {
    // ... existing fields ...
    /// Currently running TTS process (if any)
    pub tts_process: Arc<std::sync::Mutex<Option<std::process::Child>>>,
}
```

In `AuralisState::new()`:

```rust
tts_process: Arc::new(std::sync::Mutex::new(None)),
```

**Step 3: Register TTS commands in `main.rs`**

In `src-tauri/src/main.rs`:

Add module declaration:
```rust
mod commands_tts;
```

Add import:
```rust
use commands_tts::*;
```

Add to `invoke_handler`:
```rust
// TTS
speak_text,
stop_tts,
list_tts_voices,
```

**Step 4: Run tests**

Run: `cd src-tauri && cargo test`
Expected: All tests pass, including the new `test_voice_for_language_*` tests.

**Step 5: Commit**

```bash
git add src-tauri/src/commands_tts.rs src-tauri/src/state.rs src-tauri/src/main.rs
git commit -m "feat: add macOS native TTS commands (speak_text, stop_tts, list_tts_voices)"
```

---

### Task 3: Add TTS Toggle to Settings UI

**Files:**
- Modify: `src/components/SettingsView.svelte`
- Modify: `src/App.svelte`
- Modify: `src/types.ts`

**Step 1: Update `src/types.ts`**

No changes needed — the existing `Segment` type already has `targetLang` which is what TTS uses.

**Step 2: Add `ttsEnabled` prop and local state to `SettingsView.svelte`**

In `SettingsView.svelte`, add to props (after `endpointDelay`):

```typescript
ttsEnabled = false,
```

Add to props type:

```typescript
ttsEnabled?: boolean;
```

Add to `onSave` callback type in props:

```typescript
tts_enabled: boolean;
```

Add local state:

```typescript
let localTtsEnabled = $state(false);
```

In the `$effect` that syncs props to local state, add:

```typescript
localTtsEnabled = ttsEnabled;
```

In `handleSave()`, add to the `onSave` call:

```typescript
tts_enabled: localTtsEnabled,
```

**Step 3: Add TTS toggle UI in Display tab**

In the Display tab section (after Max Lines), add:

```svelte
<!-- TTS -->
<div class="section-label" style="margin-top: var(--space-md);">Text-to-Speech</div>
<p class="section-desc">Automatically speak translated text aloud using macOS built-in voices. Not all languages are supported (Vietnamese and Hindi have no macOS voices).</p>
<div class="toggle-row">
  <span class="toggle-label">Speak translations</span>
  <button
    class="toggle"
    class:active={localTtsEnabled}
    onclick={() => localTtsEnabled = !localTtsEnabled}
    disabled={isTranslating}
  >
    <span class="toggle-thumb"></span>
  </button>
</div>
```

**Step 4: Add toggle CSS**

Add to the `<style>` section:

```css
/* Toggle switch */
.toggle-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-sm) 0;
}

.toggle-label {
  font-size: var(--font-size-sm);
  color: var(--text-primary);
}

.toggle {
  width: 40px;
  height: 22px;
  border-radius: 11px;
  border: none;
  background: rgba(255, 255, 255, 0.1);
  position: relative;
  cursor: pointer;
  transition: background 0.2s ease;
  flex-shrink: 0;
}

.toggle.active {
  background: var(--accent);
}

.toggle-thumb {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: white;
  transition: transform 0.2s ease;
  pointer-events: none;
}

.toggle.active .toggle-thumb {
  transform: translateX(18px);
}

.toggle:disabled {
  opacity: 0.4;
  cursor: default;
}
```

**Step 5: Update `App.svelte` to handle TTS setting**

In `App.svelte`, add state:

```typescript
let ttsEnabled = $state(false);
```

In `loadSettings()`, after loading `endpointDelay`:

```typescript
ttsEnabled = settings.tts_enabled as boolean;
```

In `persistSettings()`, add to the settings object:

```typescript
tts_enabled: ttsEnabled,
```

In `handleSettingsSave()`, add:

```typescript
ttsEnabled = settings.tts_enabled;
```

Pass to `SettingsView`:

```svelte
<SettingsView
  ...
  ttsEnabled={ttsEnabled}
  ...
/>
```

Update the `onSave` callback type to include `tts_enabled: boolean`.

**Step 6: Run tests**

Run: `cd src-tauri && cargo test`
Expected: All tests pass.

**Step 7: Commit**

```bash
git add src/components/SettingsView.svelte src/App.svelte
git commit -m "feat: add TTS toggle to display settings"
```

---

### Task 4: Wire TTS into Translation Flow

**Files:**
- Modify: `src/App.svelte`

**Step 1: Add `speakTranslation` helper function**

In `App.svelte`, add a helper that calls the TTS backend:

```typescript
async function speakTranslation(text: string, targetLang: string): Promise<void> {
  if (!ttsEnabled || !text.trim()) return;
  try {
    await invoke('speak_text', { text, language: targetLang });
  } catch (err) {
    console.warn('[Auralis] TTS failed:', err);
  }
}
```

**Step 2: Call TTS from cloud mode (Soniox)**

In the `SonioxClient` constructor in `startCloudMode()`, update `onTranslation`:

```typescript
onTranslation: (text: string, _is_final: boolean) => {
  if (text.trim()) {
    // Determine the target language for this translation
    // (In cloud mode, pairTranslation matches to pending segments)
    pairTranslation(text);
    speakTranslation(text, targetLanguage);
  }
},
```

**Step 3: Call TTS from offline mode**

In the `pipeline-result` event listener (the `type === 'result'` branch), after adding the translated segment, call TTS.

Find the section where `translated` is non-empty in the `type === 'result'` handler and add after `segments = segments;`:

```typescript
// Speak translated text if TTS is enabled
if (translated && target) {
  speakTranslation(translated, target);
}
```

**Step 4: Stop TTS when stopping translation**

In `handleStop()`, after stopping the pipeline, also stop TTS:

```typescript
async function handleStop(): Promise<void> {
  try {
    // Stop TTS
    try { await invoke('stop_tts'); } catch {}

    if (mode === 'cloud') {
      stopCloudMode();
      await invoke<string>('stop_audio_capture');
    } else {
      await stopOfflineMode();
    }

    isTranslating = false;
    statusMessage = 'Stopped';
  } catch (error) {
    errorMessage = `Failed to stop: ${error}`;
    statusMessage = 'Stop failed';
    isTranslating = false;
  }
}
```

**Step 5: Run tests**

Run: `cd src-tauri && cargo test`
Expected: All tests pass.

**Step 6: Manual verification**

Run: `npm run tauri dev`

Test flow:
1. Open Settings → Display tab → Enable "Speak translations" toggle
2. Save settings
3. Start recording (cloud or offline mode)
4. Speak a sentence in English
5. Verify the translated text appears in the transcript
6. Verify macOS speaks the translation aloud
7. Verify that speaking a new sentence interrupts the previous speech
8. Stop recording → verify TTS stops
9. Disable TTS toggle → verify no speech plays

**Step 7: Commit**

```bash
git add src/App.svelte
git commit -m "feat: wire TTS into translation flow for cloud and offline modes"
```

---

### Task 5: Cleanup and Integration Test

**Step 1: Verify settings persistence roundtrip**

Run the app, enable TTS, save, close, reopen. Verify TTS toggle is still enabled.

**Step 2: Verify backward compatibility**

Delete `~/.config/auralis/settings.json`, restart app. Verify app loads with `tts_enabled: false` (default).

**Step 3: Test language coverage**

For each supported language, verify whether TTS works:
- Should work: en, es, fr, de, zh, ja, ko, pt, ru, ar
- Should skip silently: vi, hi (no macOS voices)

**Step 4: Final commit**

```bash
git add -A
git commit -m "feat: complete macOS native TTS integration"
```

---

### Scope

**Included:**
- macOS native `say` command for TTS
- `tts_enabled` setting with UI toggle
- Auto-speak translated text in all modes (cloud + offline, one-way + two-way)
- Interrupt previous speech when new translation arrives
- Language → voice mapping with graceful fallback

**Not included (future work):**
- Piper TTS for offline voices (Vietnamese, Hindi, better quality)
- TTS voice selection UI
- TTS speed/rate control
- TTS volume control
- Manual "speak this segment" button (click-to-speak)
