# Meeting Summary Feature Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add AI-powered meeting summary generation to saved transcripts, supporting offline (Gemma-3) and cloud (Claude/GPT) backends.

**Architecture:** Extend the Python sidecar with a `--mode summary` entry point that reads a transcript file, sends it through an LLM, and outputs structured JSON. Rust spawns this process and forwards results to the frontend via Tauri events. The frontend shows summary previews in the transcript list and a dedicated summary view.

**Tech Stack:** Python (mlx-lm, httpx), Rust/Tauri (serde_json, process spawn), Svelte 5

**Design doc:** `docs/plans/2026-04-13-meeting-summary-design.md`

---

## Task 1: Create Python summary module (`scripts/summary.py`)

**Files:**
- Create: `scripts/summary.py`

**Step 1: Create the summary module with offline (Gemma-3) support**

```python
#!/usr/bin/env python3
"""
Summary generation module for Auralis.
Reads a transcript file and generates a structured meeting summary.

Usage:
  python3 summary.py --input <transcript.txt> --tier <free|pro> --model <auto|gemma|claude|gpt|gemini>
"""

import sys
import os
import json
import argparse
from datetime import datetime, timezone

# Add parent dir for shared helpers
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from local_pipeline import emit, log, LANG_NAMES


def parse_transcript(filepath: str) -> list[dict]:
    """Parse a .txt transcript file into structured segments.

    Each line format: [HH:MM:SS] original (src → tgt) translated
    """
    segments = []
    with open(filepath, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            segments.append({"text": line})
    return segments


def build_prompt(segments: list[dict], tier: str, target_lang: str = "en") -> str:
    """Build the LLM prompt based on tier and segments."""
    lang_name = LANG_NAMES.get(target_lang, target_lang)
    transcript_text = "\n".join(s["text"] for s in segments)

    if tier == "pro":
        return f"""You are a meeting assistant. Analyze this meeting transcript and generate a structured summary.

Respond ONLY with valid JSON (no markdown, no code fences):
{{
  "key_points": ["point1", "point2", ...],
  "full_summary": "A 3-5 sentence summary of the meeting.",
  "action_items": [{{"task": "...", "assignee": "..." or null, "due": "..." or null}}],
  "decisions": ["decision1", "decision2", ...]
}}

Rules:
- key_points: 3-7 bullet points capturing the main topics
- full_summary: Write in {lang_name}
- action_items: Extract specific tasks, who owns them, and deadlines if mentioned
- decisions: Extract concrete decisions made during the meeting
- If no action items or decisions exist, use empty arrays

Transcript:
{transcript_text}"""
    else:
        return f"""You are a meeting assistant. Analyze this meeting transcript and generate a brief summary.

Respond ONLY with valid JSON (no markdown, no code fences):
{{
  "key_points": ["point1", "point2", ...],
  "full_summary": "A 2-3 sentence summary of the meeting."
}}

Rules:
- key_points: 3-5 bullet points capturing the main topics
- full_summary: Write in {lang_name}

Transcript:
{transcript_text}"""


def parse_json_response(text: str) -> dict:
    """Extract and parse JSON from LLM response (handles markdown fences)."""
    # Strip markdown code fences if present
    text = text.strip()
    if text.startswith("```"):
        lines = text.split("\n")
        # Remove first and last lines (```json and ```)
        lines = [l for l in lines if not l.strip().startswith("```")]
        text = "\n".join(lines)

    return json.loads(text)


def generate_offline(segments: list[dict], tier: str, target_lang: str) -> dict:
    """Generate summary using local Gemma-3 model via mlx-lm."""
    emit({"type": "status", "message": "Loading Gemma-3 model for summary..."})

    from mlx_lm import load as mlx_load, generate as mlx_generate

    model_id = "mlx-community/gemma-3-4b-it-4bit"
    model, tokenizer = mlx_load(model_id)

    prompt = build_prompt(segments, tier, target_lang)

    # Build messages for chat template
    messages = [{"role": "user", "content": prompt}]
    if hasattr(tokenizer, "apply_chat_template"):
        input_text = tokenizer.apply_chat_template(
            messages, tokenize=False, add_generation_prompt=True
        )
    else:
        input_text = prompt

    input_ids = tokenizer.encode(input_text)
    import mlx.core as mx
    input_ids = mx.array(input_ids)

    emit({"type": "status", "message": "Generating summary..."})

    output = mlx_generate(
        model, tokenizer, prompt=input_text, max_tokens=1024, temp=0.3, verbose=False
    )

    # Strip the prompt from the output
    response_text = output[len(input_text):] if output.startswith(input_text) else output

    result = parse_json_response(response_text)
    return result


def generate_cloud_claude(segments: list[dict], tier: str, target_lang: str, api_key: str) -> dict:
    """Generate summary using Claude API."""
    import httpx

    prompt = build_prompt(segments, tier, target_lang)

    emit({"type": "status", "message": "Calling Claude API..."})

    resp = httpx.post(
        "https://api.anthropic.com/v1/messages",
        headers={
            "x-api-key": api_key,
            "anthropic-version": "2023-06-01",
            "content-type": "application/json",
        },
        json={
            "model": "claude-haiku-4-5-20251001",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": prompt}],
        },
        timeout=30,
    )
    resp.raise_for_status()
    data = resp.json()
    response_text = data["content"][0]["text"]

    return parse_json_response(response_text)


def generate_cloud_openai(segments: list[dict], tier: str, target_lang: str, api_key: str) -> dict:
    """Generate summary using OpenAI API."""
    import httpx

    prompt = build_prompt(segments, tier, target_lang)

    emit({"type": "status", "message": "Calling OpenAI API..."})

    resp = httpx.post(
        "https://api.openai.com/v1/chat/completions",
        headers={
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
        },
        json={
            "model": "gpt-4o-mini",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": prompt}],
        },
        timeout=30,
    )
    resp.raise_for_status()
    data = resp.json()
    response_text = data["choices"][0]["message"]["content"]

    return parse_json_response(response_text)


def detect_target_lang(segments: list[dict]) -> str:
    """Detect target language from transcript lines. Look for '→ vi' pattern."""
    for seg in segments:
        if "→" in seg["text"]:
            # Extract target lang from pattern like "(en → vi)"
            parts = seg["text"].split("→")
            if len(parts) >= 2:
                after = parts[-1].strip().split(")")[0].strip()
                if after in LANG_NAMES:
                    return after
    return "en"


def run_summary(
    input_path: str,
    tier: str,
    model: str,
    api_key: str = "",
    openai_key: str = "",
):
    """Main entry point: load transcript, generate summary, emit result."""
    emit({"type": "status", "message": "Reading transcript..."})

    if not os.path.exists(input_path):
        emit({"type": "error", "message": f"File not found: {input_path}"})
        return

    segments = parse_transcript(input_path)
    if not segments:
        emit({"type": "error", "message": "Transcript is empty"})
        return

    target_lang = detect_target_lang(segments)

    # Determine model
    if model == "auto":
        # Prefer cloud if API key available, fall back to offline
        if api_key:
            model = "claude"
        elif openai_key:
            model = "gpt"
        else:
            model = "gemma"

    try:
        if model == "claude":
            result = generate_cloud_claude(segments, tier, target_lang, api_key)
            model_name = "claude-haiku-4.5"
        elif model == "gpt":
            result = generate_cloud_openai(segments, tier, target_lang, openai_key)
            model_name = "gpt-4o-mini"
        else:
            result = generate_offline(segments, tier, target_lang)
            model_name = "gemma-3-4b-it"
    except Exception as e:
        emit({"type": "error", "message": f"Summary generation failed: {e}"})
        return

    # Build output
    summary_data = {
        "version": 1,
        "transcript_file": os.path.basename(input_path),
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "model_used": model_name,
        "tier": tier,
        "summary": result,
    }

    emit({"type": "summary", "data": summary_data})
    emit({"type": "done"})


def main():
    parser = argparse.ArgumentParser(description="Auralis meeting summary generator")
    parser.add_argument("--input", required=True, help="Path to transcript .txt file")
    parser.add_argument("--tier", default="free", choices=["free", "pro"])
    parser.add_argument("--model", default="auto", choices=["auto", "gemma", "claude", "gpt"])
    parser.add_argument("--api-key", default="", help="Anthropic API key")
    parser.add_argument("--openai-key", default="", help="OpenAI API key")
    args = parser.parse_args()

    run_summary(
        input_path=args.input,
        tier=args.tier,
        model=args.model,
        api_key=args.api_key,
        openai_key=args.openai_key,
    )


if __name__ == "__main__":
    main()
```

**Step 2: Verify the module runs**

```bash
cd /Users/benq/Desktop/sentia-lab/auralis
python3 scripts/summary.py --help
```

Expected: Shows help text with `--input`, `--tier`, `--model` options.

**Step 3: Commit**

```bash
git add scripts/summary.py
git commit -m "feat: add Python summary generation module"
```

---

## Task 2: Add summary settings to Rust state

**Files:**
- Modify: `src-tauri/src/state.rs:14-59` (Settings struct)
- Modify: `src-tauri/src/state.rs:97-118` (Default impl)

**Step 1: Add summary fields to Settings struct**

Add after line 58 (`pub elevenlabs_api_key: String`):

```rust
    /// Summary provider: "auto", "gemma", "claude", or "gpt"
    #[serde(default = "default_summary_provider")]
    pub summary_provider: String,
    /// Anthropic API key for Claude summaries
    #[serde(default)]
    pub claude_api_key: String,
    /// OpenAI API key for GPT summaries
    #[serde(default)]
    pub openai_api_key: String,
```

Add default function after `default_tts_provider()`:

```rust
fn default_summary_provider() -> String {
    "auto".to_string()
}
```

Add to `Default` impl:

```rust
            summary_provider: default_summary_provider(),
            claude_api_key: String::new(),
            openai_api_key: String::new(),
```

**Step 2: Build to verify**

```bash
cd /Users/benq/Desktop/sentia-lab/auralis/src-tauri && cargo check
```

Expected: Compiles without errors.

**Step 3: Commit**

```bash
git add src-tauri/src/state.rs
git commit -m "feat: add summary provider settings to Rust state"
```

---

## Task 3: Add Rust summary commands

**Files:**
- Modify: `src-tauri/src/commands_transcripts.rs` — add 3 new commands
- Modify: `src-tauri/src/main.rs:63-93` — register commands

**Step 1: Add summary data types and commands to `commands_transcripts.rs`**

Add after the existing `TranscriptMeta` struct (line 32):

```rust
/// Summary data stored as sidecar .summary.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryData {
    pub version: u32,
    pub transcript_file: String,
    pub generated_at: String,
    pub model_used: String,
    pub tier: String,
    pub summary: serde_json::Value,
}

/// Metadata about an existing summary (for list preview)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryPreview {
    pub exists: bool,
    pub model_used: Option<String>,
    pub tier: Option<String>,
    pub key_points_count: Option<usize>,
    pub action_items_count: Option<usize>,
}
```

Add helper to derive summary filename:

```rust
/// Given "2026-04-13_14-30-15.txt", return "2026-04-13_14-30-15.summary.json"
fn summary_path_for(filename: &str) -> Result<PathBuf, String> {
    let base = filename.trim_end_matches(".txt");
    let summary_name = format!("{}.summary.json", base);
    safe_path(&summary_name)
}
```

Add the three commands:

```rust
/// Generate a summary for a saved transcript.
/// Spawns Python sidecar in summary mode and forwards events.
#[tauri::command]
pub async fn generate_summary(
    app: AppHandle,
    state: State<'_, AuralisState>,
    filename: String,
    tier: String,
) -> Result<(), String> {
    let transcript_path = safe_path(&filename)?;
    if !transcript_path.exists() {
        return Err(format!("Transcript not found: {}", filename));
    }

    // Read settings for provider and API keys
    let (provider, claude_key, openai_key) = {
        let s = state.settings.lock().await;
        (s.summary_provider.clone(), s.claude_api_key.clone(), s.openai_api_key.clone())
    };

    // Find Python
    let python = {
        use crate::commands_pipeline::find_python;
        find_python()
    };

    // Build command args
    let script_path = transcript_path.parent()
        .unwrap_or(transcript_path.parent().unwrap())
        .join("..").join("..").join("..").join("..").join("..")
        .join("scripts").join("summary.py");

    // Use bundled script path
    let resource_path = app.path().resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {}", e))?
        .join("scripts").join("summary.py");

    let script = if resource_path.exists() {
        resource_path.to_string_lossy().to_string()
    } else {
        // Dev mode fallback
        std::env::current_dir()
            .map(|d| d.join("scripts").join("summary.py").to_string_lossy().to_string())
            .unwrap_or_else(|_| "scripts/summary.py".to_string())
    };

    let mut args = vec![
        script,
        "--input".to_string(),
        transcript_path.to_string_lossy().to_string(),
        "--tier".to_string(),
        tier.clone(),
        "--model".to_string(),
        provider.clone(),
    ];

    if !claude_key.is_empty() {
        args.push("--api-key".to_string());
        args.push(claude_key);
    }
    if !openai_key.is_empty() {
        args.push("--openai-key".to_string());
        args.push(openai_key);
    }

    // Spawn Python process
    let app_stdout = app.clone();
    let child = std::process::Command::new(&python)
        .args(&args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn summary process: {}", e))?;

    use std::io::BufRead;
    let stdout = child.stdout.unwrap();
    let reader = std::io::BufReader::new(stdout);

    for line in reader.lines() {
        match line {
            Ok(l) => {
                if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&l) {
                    let msg_type = msg.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    match msg_type {
                        "status" => {
                            let _ = app_stdout.emit("summary-progress", &l);
                        }
                        "summary" => {
                            // Save the .summary.json sidecar file
                            if let Some(data) = msg.get("data") {
                                let sum_path = summary_path_for(&filename)?;
                                let json_str = serde_json::to_string_pretty(data)
                                    .map_err(|e| format!("Failed to serialize summary: {}", e))?;
                                std::fs::write(&sum_path, json_str)
                                    .map_err(|e| format!("Failed to write summary: {}", e))?;
                            }
                            let _ = app_stdout.emit("summary-result", &l);
                        }
                        "error" => {
                            let _ = app_stdout.emit("summary-error", &l);
                        }
                        _ => {}
                    }
                }
            }
            Err(_) => break,
        }
    }

    Ok(())
}

/// Load the summary JSON for a transcript.
#[tauri::command]
pub async fn load_summary(filename: String) -> Result<Option<SummaryData>, String> {
    let path = summary_path_for(&filename)?;
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read summary: {}", e))?;
    let data: SummaryData = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse summary: {}", e))?;
    Ok(Some(data))
}

/// Delete the summary JSON for a transcript.
#[tauri::command]
pub async fn delete_summary(filename: String) -> Result<String, String> {
    let path = summary_path_for(&filename)?;
    if !path.exists() {
        return Ok("No summary to delete".to_string());
    }
    std::fs::remove_file(&path)
        .map_err(|e| format!("Failed to delete summary: {}", e))?;
    Ok(format!("Deleted summary for {}", filename))
}

/// Check if a summary exists for a transcript (for list preview).
#[tauri::command]
pub async fn check_summary(filename: String) -> Result<SummaryPreview, String> {
    let path = summary_path_for(&filename)?;
    if !path.exists() {
        return Ok(SummaryPreview {
            exists: false,
            model_used: None,
            tier: None,
            key_points_count: None,
            action_items_count: None,
        });
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read summary: {}", e))?;
    let data: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse summary: {}", e))?;

    let summary = data.get("summary");
    Ok(SummaryPreview {
        exists: true,
        model_used: data.get("model_used").and_then(|v| v.as_str()).map(|s| s.to_string()),
        tier: data.get("tier").and_then(|v| v.as_str()).map(|s| s.to_string()),
        key_points_count: summary.and_then(|s| s.get("key_points")).and_then(|k| k.as_array()).map(|a| a.len()),
        action_items_count: summary.and_then(|s| s.get("action_items")).and_then(|k| k.as_array()).map(|a| a.len()),
    })
}
```

**Step 2: Register commands in `main.rs`**

Add to the `invoke_handler` array after `delete_transcript` (line 92):

```rust
            // Summaries
            generate_summary,
            load_summary,
            delete_summary,
            check_summary,
```

Also need to import `AppHandle` in commands_transcripts.rs — add at top:
```rust
use tauri::{AppHandle, Emitter, State};
use crate::state::AuralisState;
```

**Step 3: Build to verify**

```bash
cd /Users/benq/Desktop/sentia-lab/auralis/src-tauri && cargo check
```

Expected: Compiles without errors.

**Step 4: Commit**

```bash
git add src-tauri/src/commands_transcripts.rs src-tauri/src/main.rs
git commit -m "feat: add Rust summary commands (generate, load, delete, check)"
```

---

## Task 4: Add summary settings UI to SettingsView

**Files:**
- Modify: `src/components/SettingsView.svelte` — add summary settings in Translation tab

**Step 1: Add summary provider section to SettingsView**

In the Translation tab section, add a "Summary" subsection after the existing language settings:

```svelte
<div class="setting-group">
  <label class="setting-label">Summary Provider</label>
  <select bind:value={settings.summary_provider} onchange={saveCurrentSettings}>
    <option value="auto">Auto (offline preferred)</option>
    <option value="gemma">Gemma-3 (offline)</option>
    <option value="claude">Claude (cloud)</option>
    <option value="gpt">GPT-4o-mini (cloud)</option>
  </select>
</div>

{#if settings.summary_provider === 'claude' || settings.summary_provider === 'auto'}
  <div class="setting-group">
    <label class="setting-label">Anthropic API Key</label>
    <input
      type="password"
      bind:value={settings.claude_api_key}
      onchange={saveCurrentSettings}
      placeholder="sk-ant-..."
    />
  </div>
{/if}

{#if settings.summary_provider === 'gpt' || settings.summary_provider === 'auto'}
  <div class="setting-group">
    <label class="setting-label">OpenAI API Key</label>
    <input
      type="password"
      bind:value={settings.openai_api_key}
      onchange={saveCurrentSettings}
      placeholder="sk-..."
    />
  </div>
{/if}
```

**Step 2: Verify UI renders**

```bash
npm run tauri dev
```

Go to Settings > Translation tab. Should see summary provider dropdown.

**Step 3: Commit**

```bash
git add src/components/SettingsView.svelte
git commit -m "feat: add summary provider settings to Settings UI"
```

---

## Task 5: Update SavedTranscripts with summary preview and summary view

**Files:**
- Modify: `src/components/SavedTranscripts.svelte` — major update

**Step 1: Add summary state and types**

At the top of the `<script>` section, add:

```typescript
interface SummaryPreview {
  exists: boolean;
  model_used: string | null;
  tier: string | null;
  key_points_count: number | null;
  action_items_count: number | null;
}

interface SummaryData {
  version: number;
  transcript_file: string;
  generated_at: string;
  model_used: string;
  tier: string;
  summary: {
    key_points: string[];
    full_summary: string;
    action_items?: { task: string; assignee: string | null; due: string | null }[];
    decisions?: string[];
  };
}

interface TranscriptMetaWithSummary extends TranscriptMeta {
  summaryPreview?: SummaryPreview;
}

let transcriptsWithSummaries: TranscriptMetaWithSummary[] = $state([]);
let selectedSummary: SummaryData | null = $state(null);
let showingSummary = $state(false);
let generatingSummary = $state<string | null>(null);
let summaryStatus = $state('');
```

**Step 2: Update refreshList to also check for summaries**

```typescript
async function refreshList() {
  loading = true;
  try {
    const list = await invoke<TranscriptMeta[]>('list_transcripts');
    // Check summaries in parallel
    const enriched = await Promise.all(
      list.map(async (t) => {
        try {
          const preview = await invoke<SummaryPreview>('check_summary', { filename: t.filename });
          return { ...t, summaryPreview: preview };
        } catch {
          return { ...t, summaryPreview: { exists: false } as SummaryPreview };
        }
      })
    );
    transcriptsWithSummaries = enriched;
  } catch (err) {
    console.error('Failed to load transcripts:', err);
  }
  loading = false;
}
```

**Step 3: Add generate and view summary functions**

```typescript
async function handleGenerateSummary(filename: string) {
  generatingSummary = filename;
  summaryStatus = 'Generating summary...';

  let progressUnlisten: (() => void) | null = null;
  let resultUnlisten: (() => void) | null = null;
  let errorUnlisten: (() => void) | null = null;

  try {
    const { listen } = await import('@tauri-apps/api/event');

    progressUnlisten = await listen('summary-progress', (event: any) => {
      if (event.payload?.message) {
        summaryStatus = event.payload.message;
      }
    });

    resultUnlisten = await listen('summary-result', () => {
      // Summary saved — refresh and show
      refreshList();
      handleViewSummary(filename);
    });

    errorUnlisten = await listen('summary-error', (event: any) => {
      summaryStatus = `Error: ${event.payload?.message || 'Unknown error'}`;
      generatingSummary = null;
    });

    await invoke('generate_summary', { filename, tier: 'free' });
  } catch (err) {
    console.error('Failed to generate summary:', err);
    summaryStatus = `Failed: ${err}`;
  } finally {
    generatingSummary = null;
    progressUnlisten?.();
    resultUnlisten?.();
    errorUnlisten?.();
  }
}

async function handleViewSummary(filename: string) {
  try {
    const result = await invoke<SummaryData | null>('load_summary', { filename });
    if (result) {
      selectedSummary = result;
      showingSummary = true;
    }
  } catch (err) {
    console.error('Failed to load summary:', err);
  }
}

function handleBackFromSummary() {
  showingSummary = false;
  selectedSummary = null;
}
```

Update `handleBack` to account for summary view:

```typescript
function handleBack() {
  if (showingSummary) {
    handleBackFromSummary();
  } else if (selectedContent !== null) {
    selectedContent = null;
    selectedFilename = null;
  } else {
    onBack();
  }
}
```

Also update `handleDelete` to delete the summary too:

```typescript
async function handleDelete(filename: string) {
  deleting = filename;
  try {
    await invoke('delete_transcript', { filename });
    await invoke('delete_summary', { filename }).catch(() => {});
    transcriptsWithSummaries = transcriptsWithSummaries.filter((t) => t.filename !== filename);
    if (selectedFilename === filename) {
      selectedContent = null;
      selectedFilename = null;
      showingSummary = false;
      selectedSummary = null;
    }
  } catch (err) {
    console.error('Failed to delete transcript:', err);
  }
  deleting = null;
}
```

**Step 4: Update the template — summary badge in list cards**

Replace the existing `{#each transcripts as t}` block with `{#each transcriptsWithSummaries as t}`. Inside each card, add after the card-meta div:

```svelte
{#if t.summaryPreview?.exists}
  <div class="summary-badge" onclick={(e) => { e.stopPropagation(); handleViewSummary(t.filename); }}>
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/></svg>
    Summary: {t.summaryPreview.key_points_count || 0} points
  </div>
{:else}
  <button class="summary-btn" onclick={(e) => { e.stopPropagation(); handleGenerateSummary(t.filename); }} disabled={generatingSummary === t.filename}>
    {generatingSummary === t.filename ? summaryStatus : 'Generate Summary'}
  </button>
{/if}
```

**Step 5: Add summary view in the detail section**

After the `{#if selectedContent !== null}` block, add a summary view:

```svelte
{#if showingSummary && selectedSummary}
  <!-- Summary View -->
  <div class="summary-view">
    <div class="summary-header-info">
      <span class="summary-model">{selectedSummary.model_used}</span>
      <span class="summary-date">{new Date(selectedSummary.generated_at).toLocaleString()}</span>
    </div>

    <div class="summary-section">
      <h4>Key Points</h4>
      <ul>
        {#each selectedSummary.summary.key_points as point}
          <li>{point}</li>
        {/each}
      </ul>
    </div>

    <div class="summary-section">
      <h4>Summary</h4>
      <p>{selectedSummary.summary.full_summary}</p>
    </div>

    {#if selectedSummary.summary.action_items && selectedSummary.summary.action_items.length > 0}
      <div class="summary-section pro-section">
        <div class="pro-header">
          <h4>Action Items</h4>
          <span class="pro-badge">PRO</span>
        </div>
        <ul>
          {#each selectedSummary.summary.action_items as item}
            <li>
              {item.task}
              {#if item.assignee}<span class="assignee">→ {item.assignee}</span>{/if}
              {#if item.due}<span class="due">by {item.due}</span>{/if}
            </li>
          {/each}
        </ul>
      </div>
    {/if}

    {#if selectedSummary.summary.decisions && selectedSummary.summary.decisions.length > 0}
      <div class="summary-section pro-section">
        <div class="pro-header">
          <h4>Decisions</h4>
          <span class="pro-badge">PRO</span>
        </div>
        <ul>
          {#each selectedSummary.summary.decisions as decision}
            <li>{decision}</li>
          {/each}
        </ul>
      </div>
    {/if}

    <div class="summary-actions">
      <button class="btn-secondary" onclick={() => handleGenerateSummary(selectedFilename || '')}>
        Regenerate
      </button>
      <button class="btn-secondary" onclick={() => {
        navigator.clipboard.writeText(JSON.stringify(selectedSummary.summary, null, 2));
      }}>
        Copy JSON
      </button>
    </div>
  </div>
{/if}
```

**Step 6: Add CSS for summary components**

```css
.summary-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  margin-top: var(--space-xs);
  padding: 2px 8px;
  font-size: var(--font-size-xs);
  color: var(--accent);
  background: var(--accent-dim, rgba(99, 102, 241, 0.1));
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: opacity var(--transition-fast);
}
.summary-badge:hover { opacity: 0.8; }

.summary-btn {
  margin-top: var(--space-xs);
  padding: 2px 8px;
  font-size: var(--font-size-xs);
  color: var(--text-dim);
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: color var(--transition-fast), border-color var(--transition-fast);
}
.summary-btn:hover {
  color: var(--accent);
  border-color: var(--accent);
}
.summary-btn:disabled { opacity: 0.5; cursor: not-allowed; }

.summary-view {
  display: flex;
  flex-direction: column;
  gap: var(--space-md);
}

.summary-header-info {
  display: flex;
  justify-content: space-between;
  font-size: var(--font-size-xs);
  color: var(--text-dim);
}

.summary-section h4 {
  font-size: var(--font-size-xs);
  font-weight: 600;
  text-transform: uppercase;
  color: var(--text-dim);
  margin-bottom: var(--space-xs);
}

.summary-section ul {
  list-style: none;
  padding: 0;
}

.summary-section li {
  font-size: var(--font-size-sm);
  color: var(--text-primary);
  padding: 2px 0 2px 16px;
  position: relative;
}
.summary-section li::before {
  content: "•";
  position: absolute;
  left: 0;
  color: var(--text-dim);
}

.summary-section p {
  font-size: var(--font-size-sm);
  color: var(--text-primary);
  line-height: 1.6;
}

.pro-header {
  display: flex;
  align-items: center;
  gap: var(--space-xs);
}

.pro-badge {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 4px;
  background: var(--accent);
  color: white;
  font-weight: 600;
}

.assignee { color: var(--accent); font-size: var(--font-size-xs); }
.due { color: var(--text-dim); font-size: var(--font-size-xs); margin-left: 4px; }

.summary-actions {
  display: flex;
  gap: var(--space-sm);
  padding-top: var(--space-sm);
  border-top: 1px solid var(--border);
}

.btn-secondary {
  padding: 4px 12px;
  font-size: var(--font-size-xs);
  border-radius: var(--radius-sm);
  border: 1px solid var(--border);
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
}
.btn-secondary:hover { border-color: var(--text-dim); }
```

**Step 7: Verify the UI**

```bash
npm run tauri dev
```

Go to Saved Transcripts → should see "Generate Summary" buttons.

**Step 8: Commit**

```bash
git add src/components/SavedTranscripts.svelte
git commit -m "feat: add summary preview, generation, and view UI to SavedTranscripts"
```

---

## Task 6: Add auto-summary on transcript save

**Files:**
- Modify: `src/App.svelte:429-448` (clearSegments function)

**Step 1: Add summary generation after auto-save**

In the `clearSegments` function, after the `save_transcript` invoke succeeds (after line 441), add:

```typescript
        // Auto-generate summary
        invoke('generate_summary', { filename: savedFilename, tier: 'free' })
          .catch((err) => console.warn('Failed to auto-generate summary:', err));
```

The `save_transcript` command returns the filename — capture it:

Change line 433 from:
```typescript
        await invoke('save_transcript', {
```
to capture the result:
```typescript
        const savedFilename = await invoke<string>('save_transcript', {
```

**Step 2: Verify**

```bash
npm run tauri dev
```

Start a recording, speak, stop. Check that a `.summary.json` file appears in the transcripts directory.

**Step 3: Commit**

```bash
git add src/App.svelte
git commit -m "feat: auto-generate summary when transcript is saved"
```

---

## Task 7: Add "View Summary" button to transcript detail view

**Files:**
- Modify: `src/components/SavedTranscripts.svelte` — detail view section

**Step 1: Add summary button in detail view**

In the detail view section (after `<pre class="detail-text">`), add:

```svelte
<div class="detail-actions">
  {#if showingSummary}
    <button class="btn-secondary" onclick={handleBackFromSummary}>Back to Transcript</button>
  {:else}
    <button class="btn-secondary" onclick={() => handleViewSummary(selectedFilename || '')}>
      View Summary
    </button>
  {/if}
</div>
```

**Step 2: Commit**

```bash
git add src/components/SavedTranscripts.svelte
git commit -m "feat: add View Summary button in transcript detail"
```

---

## Task 8: End-to-end testing and polish

**Step 1: Test offline summary (Gemma-3)**

1. Open Auralis in dev mode
2. Record a short conversation
3. Stop recording → auto-save triggers
4. Go to Saved Transcripts → check if "Generate Summary" or summary badge appears
5. Click to view summary

**Step 2: Test cloud summary (Claude)**

1. Settings > Translation > Summary Provider > Claude
2. Add Anthropic API key
3. Record a conversation
4. Stop → auto-summary using Claude
5. View summary — should have key points + full summary

**Step 3: Fix any issues found during testing**

**Step 4: Final commit**

```bash
git add -A
git commit -m "feat: complete meeting summary feature with offline/cloud support"
```

---

## Summary of all commits

1. `feat: add Python summary generation module`
2. `feat: add summary provider settings to Rust state`
3. `feat: add Rust summary commands (generate, load, delete, check)`
4. `feat: add summary provider settings to Settings UI`
5. `feat: add summary preview, generation, and view UI to SavedTranscripts`
6. `feat: auto-generate summary when transcript is saved`
7. `feat: add View Summary button in transcript detail`
8. `feat: complete meeting summary feature with offline/cloud support`
