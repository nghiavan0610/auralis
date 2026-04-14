#!/usr/bin/env python3
"""
Meeting summary generation module for Auralis.

Reads saved transcript files and generates structured meeting summaries
using either a local Gemma-3 LLM (offline) or cloud APIs (Claude / GPT).

Protocol (JSON over stdout, same as local_pipeline.py):
  stdout -> JSON lines:
    {"type":"status","message":"Reading transcript..."}
    {"type":"summary","data":{...}}
    {"type":"error","message":"..."}
    {"type":"done"}
  stderr -> log messages

Usage:
  python3 summary.py --input <file.txt> --tier <free|pro> --model <auto|gemma|claude|gpt> --api-key <key> --openai-key <key>
"""

import sys
import os
import json
import re
import time
import argparse
import urllib.request
import urllib.error

# Suppress tokenizers parallelism warning
os.environ["TOKENIZERS_PARALLELISM"] = "false"

# ---------------------------------------------------------------------------
# Language names (shared with local_pipeline.py)
# ---------------------------------------------------------------------------

LANG_NAMES = {
    "vi": "Vietnamese", "en": "English", "ja": "Japanese",
    "ko": "Korean", "zh": "Chinese", "fr": "French",
    "de": "German", "es": "Spanish", "th": "Thai",
    "pt": "Portuguese", "ru": "Russian", "ar": "Arabic",
    "hi": "Hindi", "it": "Italian", "nl": "Dutch",
}

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def log(msg):
    """Log to stderr so it does not interfere with the stdout protocol."""
    print(f"[summary] {msg}", file=sys.stderr, flush=True)


def emit(data):
    """Write a JSON object to stdout as a single line and flush."""
    print(json.dumps(data, ensure_ascii=False), flush=True)


# ---------------------------------------------------------------------------
# Transcript parsing
# ---------------------------------------------------------------------------

# Transcript line format: [HH:MM:SS] original (src_lang -> tgt_lang) translated
# The arrow is Unicode RIGHTWARDS ARROW (U+2192)
LINE_PATTERN = re.compile(
    r'^\[[^\]]+\]\s+'          # [HH:MM:SS]
    r'(.+?)\s+'                # original text (captured lazily)
    r'\((\w+)\s*\u2192\s*(\w+)\)\s+'  # (src -> tgt)
    r'(.+)$'                   # translated text
)


def parse_transcript(filepath: str) -> list[dict]:
    """Parse a transcript file into a list of segment dicts.

    Each segment dict has keys: original, translated, source_lang, target_lang.
    Returns an empty list if the file is empty or cannot be read.
    """
    if not os.path.isfile(filepath):
        return []

    segments = []
    try:
        with open(filepath, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                m = LINE_PATTERN.match(line)
                if m:
                    segments.append({
                        "original": m.group(1).strip(),
                        "translated": m.group(4).strip(),
                        "source_lang": m.group(2),
                        "target_lang": m.group(3),
                    })
                else:
                    # Malformed line -- include as raw text for context
                    segments.append({
                        "original": line,
                        "translated": "",
                        "source_lang": "",
                        "target_lang": "",
                    })
    except Exception as exc:
        log(f"Error reading transcript: {exc}")
        return []

    return segments


def detect_target_language(segments: list[dict]) -> str:
    """Detect the most common target language from transcript segments.

    Looks at the (src -> tgt) pattern in parsed lines.
    Returns the most frequent target lang code, defaults to 'en'.
    """
    if not segments:
        return "en"

    lang_counts: dict[str, int] = {}
    for seg in segments:
        tgt = seg.get("target_lang", "")
        if tgt:
            lang_counts[tgt] = lang_counts.get(tgt, 0) + 1

    if not lang_counts:
        # Fall back: try detecting from raw text using the arrow pattern
        return "en"

    return max(lang_counts, key=lang_counts.get)


def build_transcript_text(segments: list[dict]) -> str:
    """Build a readable transcript string for the LLM prompt.

    Includes both original and translated text per segment.
    """
    lines = []
    for seg in segments:
        orig = seg.get("original", "")
        trans = seg.get("translated", "")
        if trans:
            lines.append(f"- {orig}  /  {trans}")
        else:
            lines.append(f"- {orig}")
    return "\n".join(lines)


# ---------------------------------------------------------------------------
# Prompt construction
# ---------------------------------------------------------------------------

def build_prompt(transcript_text: str, target_lang: str, tier: str) -> str:
    """Build the LLM prompt for summary generation.

    Free tier: key_points (3-5 bullets) + full_summary (2-3 sentences).
    Pro tier: key_points + full_summary + action_items + decisions.
    """
    lang_name = LANG_NAMES.get(target_lang, target_lang)

    if tier == "pro":
        schema_desc = (
            'Return a JSON object with exactly these keys:\n'
            '  "key_points": array of 3-5 strings (main discussion points),\n'
            '  "full_summary": string (2-3 sentence overview),\n'
            '  "action_items": array of objects with keys "task", "assignee", "due"\n'
            '    (use empty string for unknown assignee/due),\n'
            '  "decisions": array of strings (decisions made during the meeting).\n'
        )
    else:
        schema_desc = (
            'Return a JSON object with exactly these keys:\n'
            '  "key_points": array of 3-5 strings (main discussion points),\n'
            '  "full_summary": string (2-3 sentence overview).\n'
        )

    prompt = (
        f"You are a meeting assistant. Analyze the following meeting transcript "
        f"and generate a structured summary.\n\n"
        f"IMPORTANT: Write ALL output text (key points, summary, action items, decisions) "
        f"in {lang_name}.\n\n"
        f"{schema_desc}\n"
        f"Output ONLY the JSON object, no other text.\n\n"
        f"Transcript:\n{transcript_text}"
    )
    return prompt


# ---------------------------------------------------------------------------
# Output cleaning
# ---------------------------------------------------------------------------

def strip_code_fences(text: str) -> str:
    """Remove markdown code fences (```json ... ```) from LLM output."""
    text = text.strip()
    # Remove opening fence with optional language tag
    if text.startswith("```"):
        # Strip first line (```json or ```)
        text = re.sub(r'^```\w*\n?', '', text)
    # Remove closing fence
    if text.endswith("```"):
        text = text[:-3].strip()
    return text


def parse_llm_json(raw: str) -> dict:
    """Parse the LLM output into a dict, handling code fences and whitespace.

    Returns a default empty structure on failure.
    """
    cleaned = strip_code_fences(raw)

    # Try to extract JSON object from the text
    # Sometimes the LLM adds extra text before/after the JSON
    brace_start = cleaned.find("{")
    brace_end = cleaned.rfind("}")
    if brace_start != -1 and brace_end != -1 and brace_end > brace_start:
        cleaned = cleaned[brace_start:brace_end + 1]

    try:
        return json.loads(cleaned)
    except json.JSONDecodeError as exc:
        log(f"Failed to parse LLM JSON output: {exc}")
        log(f"Raw output (first 500 chars): {raw[:500]}")
        return {}


# ---------------------------------------------------------------------------
# Model selection
# ---------------------------------------------------------------------------

def resolve_model(model_arg: str, api_key: str = "", openai_key: str = "") -> str:
    """Resolve the --model argument to a concrete model identifier.

    'auto' falls back to gemma (offline-first).
    """
    if model_arg == "auto":
        return "gemma"
    return model_arg


# ---------------------------------------------------------------------------
# Offline generation: Gemma-3 via mlx_lm
# ---------------------------------------------------------------------------

def generate_gemma(prompt: str) -> str:
    """Generate summary using local Gemma-3-4B-IT via mlx_lm.

    Loads the quantized model, runs generation, returns raw text.
    Uses the same QAT model as translation to avoid duplicate downloads.
    """
    from mlx_lm import load, generate

    emit({"type": "status", "message": "Loading Gemma-3-4B model..."})
    log("Loading mlx-community/gemma-3-4b-it-qat-4bit...")
    t0 = time.time()

    model, tokenizer = load("mlx-community/gemma-3-4b-it-qat-4bit")
    log(f"Model loaded in {time.time() - t0:.1f}s")

    # Wrap prompt in Gemma chat format
    gemma_prompt = (
        "<start_of_turn>user\n"
        f"{prompt}\n"
        "<end_of_turn>\n"
        "<start_of_turn>model\n"
    )

    emit({"type": "status", "message": "Generating summary..."})
    log("Generating summary with Gemma-3...")
    t1 = time.time()

    result = generate(
        model,
        tokenizer,
        prompt=gemma_prompt,
        max_tokens=2048,
        verbose=False,
    )

    log(f"Generation completed in {time.time() - t1:.1f}s")

    # Clean special tokens
    result = result.split("<end_of_turn>")[0].strip()
    return result


# ---------------------------------------------------------------------------
# Cloud generation: Claude via Anthropic Messages API
# ---------------------------------------------------------------------------

def generate_claude(prompt: str, api_key: str) -> str:
    """Generate summary using Claude Haiku via the Anthropic Messages API.

    Uses urllib to avoid requiring httpx as a hard dependency.
    Falls back to httpx if available for better HTTP/2 support.
    """
    emit({"type": "status", "message": "Calling Claude API..."})
    log("Generating summary with Claude Haiku...")

    url = "https://api.anthropic.com/v1/messages"
    body = json.dumps({
        "model": "claude-haiku-4-5-20251001",
        "max_tokens": 1024,
        "messages": [
            {"role": "user", "content": prompt},
        ],
    }).encode("utf-8")

    headers = {
        "Content-Type": "application/json",
        "x-api-key": api_key,
        "anthropic-version": "2023-06-01",
    }

    t0 = time.time()

    try:
        req = urllib.request.Request(url, data=body, headers=headers, method="POST")
        with urllib.request.urlopen(req, timeout=60) as resp:
            data = json.loads(resp.read().decode("utf-8"))
            text = "".join(
                block.get("text", "")
                for block in data.get("content", [])
                if block.get("type") == "text"
            )
            log(f"Claude API completed in {time.time() - t0:.1f}s")
            return text.strip()
    except urllib.error.HTTPError as exc:
        error_body = exc.read().decode("utf-8", errors="replace") if exc.fp else ""
        raise RuntimeError(f"Claude API error {exc.code}: {error_body}")
    except urllib.error.URLError as exc:
        raise RuntimeError(f"Claude API connection error: {exc.reason}")


# ---------------------------------------------------------------------------
# Cloud generation: GPT via OpenAI Chat Completions API
# ---------------------------------------------------------------------------

def generate_gpt(prompt: str, api_key: str) -> str:
    """Generate summary using GPT-4o-mini via the OpenAI Chat Completions API.

    Uses urllib to avoid requiring httpx as a hard dependency.
    """
    emit({"type": "status", "message": "Calling OpenAI API..."})
    log("Generating summary with GPT-4o-mini...")

    url = "https://api.openai.com/v1/chat/completions"
    body = json.dumps({
        "model": "gpt-4o-mini",
        "max_tokens": 1024,
        "messages": [
            {"role": "user", "content": prompt},
        ],
    }).encode("utf-8")

    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {api_key}",
    }

    t0 = time.time()

    try:
        req = urllib.request.Request(url, data=body, headers=headers, method="POST")
        with urllib.request.urlopen(req, timeout=60) as resp:
            data = json.loads(resp.read().decode("utf-8"))
            text = data.get("choices", [{}])[0].get("message", {}).get("content", "")
            log(f"OpenAI API completed in {time.time() - t0:.1f}s")
            return text.strip()
    except urllib.error.HTTPError as exc:
        error_body = exc.read().decode("utf-8", errors="replace") if exc.fp else ""
        raise RuntimeError(f"OpenAI API error {exc.code}: {error_body}")
    except urllib.error.URLError as exc:
        raise RuntimeError(f"OpenAI API connection error: {exc.reason}")


# ---------------------------------------------------------------------------
# Output construction
# ---------------------------------------------------------------------------

MODEL_LABELS = {
    "gemma": "AI (Offline)",
    "claude": "AI (Cloud)",
    "gpt": "AI (Advanced)",
}


def build_summary_result(
    parsed: dict,
    tier: str,
    model_used: str,
    transcript_file: str,
) -> dict:
    """Build the final summary JSON result object.

    Fills in defaults for any missing fields from the LLM output.
    """
    summary = {
        "key_points": parsed.get("key_points", []),
        "full_summary": parsed.get("full_summary", ""),
    }

    if tier == "pro":
        # Normalize action_items to expected schema
        raw_items = parsed.get("action_items", [])
        action_items = []
        for item in raw_items:
            if isinstance(item, dict):
                action_items.append({
                    "task": item.get("task", ""),
                    "assignee": item.get("assignee", ""),
                    "due": item.get("due", ""),
                })
            elif isinstance(item, str):
                action_items.append({
                    "task": item,
                    "assignee": "",
                    "due": "",
                })
        summary["action_items"] = action_items
        summary["decisions"] = parsed.get("decisions", [])

    from datetime import datetime, timezone
    return {
        "version": 1,
        "transcript_file": transcript_file,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "model_used": MODEL_LABELS.get(model_used, model_used),
        "tier": tier,
        "summary": summary,
    }


# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(
        description="Auralis meeting summary generator",
    )
    parser.add_argument(
        "--input",
        required=True,
        help="Path to the transcript .txt file",
    )
    parser.add_argument(
        "--tier",
        choices=["free", "pro"],
        default="free",
        help="Summary tier: free (key_points + summary) or pro (+ action_items + decisions)",
    )
    parser.add_argument(
        "--model",
        choices=["auto", "gemma", "claude", "gpt"],
        default="auto",
        help="Model to use: auto (offline-first), gemma (local), claude (Anthropic), gpt (OpenAI)",
    )
    args = parser.parse_args()

    # --- Resolve API keys from environment variables ---
    api_key = os.environ.get("ANTHROPIC_API_KEY", "")
    openai_key = os.environ.get("OPENAI_API_KEY", "")

    # --- Resolve model ---
    model = resolve_model(args.model, api_key=api_key, openai_key=openai_key)
    log(f"Model: {model}, Tier: {args.tier}, Input: {args.input}")

    # --- Validate inputs ---
    if model == "claude" and not api_key:
        emit({"type": "error", "message": "Anthropic API key required for --model claude. Set ANTHROPIC_API_KEY env var."})
        emit({"type": "done"})
        sys.exit(1)

    if model == "gpt" and not openai_key:
        emit({"type": "error", "message": "OpenAI API key required for --model gpt. Set OPENAI_API_KEY env var."})
        emit({"type": "done"})
        sys.exit(1)

    # --- Read transcript ---
    emit({"type": "status", "message": "Reading transcript..."})

    if not os.path.isfile(args.input):
        emit({"type": "error", "message": f"Transcript file not found: {args.input}"})
        emit({"type": "done"})
        sys.exit(1)

    segments = parse_transcript(args.input)

    if not segments:
        emit({"type": "error", "message": "Transcript is empty or contains no parseable segments."})
        emit({"type": "done"})
        sys.exit(1)

    log(f"Parsed {len(segments)} segments from {args.input}")

    # --- Detect language ---
    target_lang = detect_target_language(segments)
    target_lang_name = LANG_NAMES.get(target_lang, target_lang)
    log(f"Detected target language: {target_lang} ({target_lang_name})")

    # --- Build prompt ---
    transcript_text = build_transcript_text(segments)
    prompt = build_prompt(transcript_text, target_lang, args.tier)

    # --- Generate summary ---
    try:
        if model == "gemma":
            raw_output = generate_gemma(prompt)
        elif model == "claude":
            raw_output = generate_claude(prompt, api_key)
        elif model == "gpt":
            raw_output = generate_gpt(prompt, openai_key)
        else:
            emit({"type": "error", "message": f"Unknown model: {model}"})
            emit({"type": "done"})
            sys.exit(1)
    except Exception as exc:
        log(f"Generation error: {exc}")
        emit({"type": "error", "message": str(exc)})
        emit({"type": "done"})
        sys.exit(1)

    # --- Parse LLM output ---
    emit({"type": "status", "message": "Parsing summary..."})
    parsed = parse_llm_json(raw_output)

    if not parsed:
        emit({"type": "error", "message": "Failed to parse LLM output as JSON. Raw output logged to stderr."})
        emit({"type": "done"})
        sys.exit(1)

    # --- Build and emit result ---
    transcript_filename = os.path.basename(args.input)
    result = build_summary_result(parsed, args.tier, model, transcript_filename)

    emit({"type": "summary", "data": result})
    emit({"type": "done"})
    log("Summary generation complete.")


if __name__ == "__main__":
    main()
