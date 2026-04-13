# Meeting Summary Feature Design

## Overview

Generate intelligent meeting summaries from saved transcripts. Supports both offline (Gemma-3) and cloud (Claude/GPT/Gemini) LLM backends. Free tier provides key points and summary; Pro tier adds action items and decisions.

## Decisions

- **Trigger**: Auto-generate on save + manual regeneration
- **AI engine**: Hybrid — Gemma-3 offline, cloud LLM when online
- **Storage**: Sidecar `.summary.json` file next to each `.txt` transcript
- **Architecture**: Python sidecar extension (Approach 1)

## Data Architecture

### File Layout

```
~/Library/Application Support/auralis/transcripts/
├── 2026-04-13_14-30-15.txt
├── 2026-04-13_14-30-15.summary.json   # sidecar
├── 2026-04-13_16-00-00.txt
└── 2026-04-13_16-00-00.summary.json
```

### Summary JSON Schema

```json
{
  "version": 1,
  "transcript_file": "2026-04-13_14-30-15.txt",
  "generated_at": "2026-04-13T14:45:00Z",
  "model_used": "gemma-3-4b-it",
  "tier": "free",
  "summary": {
    "key_points": [
      "Discussed Q2 roadmap priorities",
      "Agreed on new feature launch timeline",
      "Reviewed customer feedback from beta"
    ],
    "full_summary": "The team met to discuss Q2 roadmap priorities...",
    "action_items": [
      {"task": "Prepare design specs", "assignee": "John", "due": null},
      {"task": "Schedule follow-up", "assignee": null, "due": "2026-04-20"}
    ],
    "decisions": [
      "Launch feature X by end of April",
      "Postpone feature Y to Q3"
    ]
  }
}
```

### New Rust Commands

- `generate_summary(filename, tier)` — spawn Python sidecar in summary mode
- `load_summary(filename)` — read and return `.summary.json`
- `delete_summary(filename)` — delete `.summary.json`

## Summary Generation Pipeline

### Flow

```
Session ends → Auto-save transcript → Trigger summary generation
                                          │
                                    ┌─────┴──────┐
                                    │  Provider?  │
                                    └─────┬──────┘
                                 offline   │   cloud
                                    │      │      │
                          ┌─────────┘      │      └─────────┐
                          ▼                ▼                 ▼
                    Gemma-3-4B       Claude/GPT/Gemini
                    (local MLX)      (API call)
                          │                │
                          ▼                ▼
                     Parse JSON ←←←←→ Parse JSON
                          │
                          ▼
                   Save .summary.json
                          │
                          ▼
                   Emit event to frontend
                          │
                          ▼
                   Show summary in UI
```

### Python Sidecar (`--mode summary`)

```bash
python local_pipeline.py --mode summary --input <file.txt> --tier <free|pro> --model <auto|gemma|claude|gpt|gemini>
```

- **Offline (Gemma-3)**: Load transcript → build prompt → mlx-lm inference → parse JSON → emit
- **Cloud (Claude/GPT/Gemini)**: Load transcript → build prompt → HTTP POST → parse JSON → emit

### Prompt Strategy

- **Free**: "Extract 3-5 key points and write a 2-3 sentence summary..."
- **Pro**: "Extract key points, action items with assignees, decisions made, and detailed summary..."
- Summary language matches the transcript's target language

### Tier Logic

- Free: `key_points` + `full_summary` only
- Pro: Everything including `action_items` + `decisions`
- Tier passed to Python, which adjusts prompt
- Tier check in Rust before spawning Python

## UI/UX

### Saved Transcripts List (enhanced)

Each card shows summary preview if available:
- "Summary available: 3 key points, 2 actions" badge
- "Generate Summary" button if no summary yet
- "View Summary" button if summary exists

### Summary View (new)

Dedicated panel with:
- Key points section (free)
- Full summary paragraph (free)
- Action items section (pro, locked for free users)
- Decisions section (pro, locked for free users)
- Regenerate / Copy / Export buttons

### Generation Progress

Loading state with progress bar during summary generation.

### Navigation

```
SavedTranscripts (list)
  → Click "View" on transcript
    → Transcript detail (existing)
      → "View Summary" / "Generate Summary"
        → Summary view (new)
```

## Tech Stack

### New Dependencies

| Layer | What | Why |
|-------|------|-----|
| Python | `httpx` | HTTP client for cloud LLM APIs |
| Python | No new ML deps | Reuse `mlx-lm` + `transformers` |
| Rust | No new deps | Reuse existing `serde_json` |
| Frontend | No new deps | Svelte 5 native |

### Files to Create

- `scripts/summary.py` — Summary generation module

### Files to Modify

- `scripts/local_pipeline.py` — Add `--mode summary` CLI arg
- `src-tauri/src/commands_transcripts.rs` — Add summary commands
- `src-tauri/src/main.rs` — Register new commands
- `src/components/SavedTranscripts.svelte` — Summary UI
- `src/components/SettingsView.svelte` — Summary provider settings

### Settings Addition

```json
{
  "summary_provider": "auto",
  "claude_api_key": "",
  "openai_api_key": "",
  "gemini_api_key": ""
}
```

`auto`: Gemma-3 if offline, first available cloud key otherwise.

### Cost Estimate

- Claude Haiku 3.5: ~$0.001 per summary
- GPT-4o-mini: ~$0.0005 per summary
- Gemma-3 offline: Free
