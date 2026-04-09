#!/usr/bin/env python3
"""
Local translation pipeline sidecar for Auralis.
Receives PCM audio via stdin, transcribes with MLX Whisper, translates with Opus-MT.
Outputs JSON results via stdout.

Protocol:
  stdin  -> raw PCM s16le 16kHz mono bytes (continuous stream, no flush markers)
  stdout -> JSON lines:
    {"type":"status","message":"Loading Whisper model..."}
    {"type":"ready"}
    {"type":"original","text":"...","source_lang":"en"}
    {"type":"result","original":"...","translated":"...","source_lang":"en","target_lang":"vi"}
    {"type":"done"}
  stderr -> log messages

Architecture (two-phase):
  Phase 1 — Fast transcription: sliding window (3s chunks, 2s stride).
    Each chunk: RMS check -> transcribe -> deduplicate -> emit "original" immediately.
    Text accumulates in a buffer for the current utterance.

  Phase 2 — Translation on silence: track RMS on incoming audio.
    When silence exceeds endpoint_delay seconds:
      -> translate all accumulated original text as one complete sentence
      -> emit "result" with both original + translated
      -> reset accumulator

  This gives ~2-3s latency for original text, translation on natural pauses.

Usage:
  python3 local_pipeline.py --source-lang en --target-lang vi --endpoint-delay 1.0
"""

import sys
import os
import json
import re
import time
import argparse
import threading

import numpy as np

# Suppress tokenizers parallelism warning
os.environ["TOKENIZERS_PARALLELISM"] = "false"

# ---------------------------------------------------------------------------
# Audio constants
# ---------------------------------------------------------------------------
SAMPLE_RATE = 16000
CHANNELS = 1
SAMPLE_WIDTH = 2  # s16le
CHUNK_SECONDS = 1.5
STRIDE_SECONDS = 1.0
CHUNK_BYTES = int(SAMPLE_RATE * CHUNK_SECONDS * SAMPLE_WIDTH)  # 64000
STRIDE_BYTES = int(SAMPLE_RATE * STRIDE_SECONDS * SAMPLE_WIDTH)  # 48000
SILENCE_THRESHOLD = 30  # RMS below this is treated as silence
# Size of audio tail to check for silence (200ms)
SILENCE_CHECK_BYTES = int(SAMPLE_RATE * 0.2 * SAMPLE_WIDTH)  # 6400
INT16_MAX = 32768.0


# ---------------------------------------------------------------------------
# Opus-MT model registry
# ---------------------------------------------------------------------------
OPUS_MODELS = {
    # English → target
    ("en", "vi"): "Helsinki-NLP/opus-mt-en-vi",
    ("en", "es"): "Helsinki-NLP/opus-mt-en-es",
    ("en", "fr"): "Helsinki-NLP/opus-mt-en-fr",
    ("en", "de"): "Helsinki-NLP/opus-mt-en-de",
    ("en", "zh"): "Helsinki-NLP/opus-mt-en-zh",
    ("en", "ja"): "Helsinki-NLP/opus-mt-en-ja",
    ("en", "ko"): "Helsinki-NLP/opus-mt-en-ko",
    ("en", "pt"): "Helsinki-NLP/opus-mt-en-pt",
    ("en", "ru"): "Helsinki-NLP/opus-mt-en-ru",
    ("en", "ar"): "Helsinki-NLP/opus-mt-en-ar",
    ("en", "hi"): "Helsinki-NLP/opus-mt-en-hi",
    # Target → English
    ("vi", "en"): "Helsinki-NLP/opus-mt-vi-en",
    ("es", "en"): "Helsinki-NLP/opus-mt-es-en",
    ("fr", "en"): "Helsinki-NLP/opus-mt-fr-en",
    ("de", "en"): "Helsinki-NLP/opus-mt-de-en",
    ("zh", "en"): "Helsinki-NLP/opus-mt-zh-en",
    ("ja", "en"): "Helsinki-NLP/opus-mt-ja-en",
    ("ko", "en"): "Helsinki-NLP/opus-mt-ko-en",
    ("pt", "en"): "Helsinki-NLP/opus-mt-pt-en",
    ("ru", "en"): "Helsinki-NLP/opus-mt-ru-en",
    ("ar", "en"): "Helsinki-NLP/opus-mt-ar-en",
    ("hi", "en"): "Helsinki-NLP/opus-mt-hi-en",
}

# CJK language codes -- use character-level dedup for these
CJK_LANGS = {"zh", "ja", "ko"}


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def log(msg):
    """Log to stderr so it does not interfere with the stdout protocol."""
    print(f"[pipeline] {msg}", file=sys.stderr, flush=True)


def emit(data):
    """Write a JSON object to stdout as a single line and flush."""
    print(json.dumps(data, ensure_ascii=False), flush=True)


# ---------------------------------------------------------------------------
# Pipeline
# ---------------------------------------------------------------------------

class LocalPipeline:
    def __init__(
        self,
        source_lang: str = "en",
        target_lang: str = "vi",
        asr_model_repo: str = "mlx-community/whisper-large-v3-turbo",
        two_way: bool = False,
        chunk_seconds: int = CHUNK_SECONDS,
        stride_seconds: int = STRIDE_SECONDS,
        endpoint_delay: float = 1.0,
    ):
        self.source_lang = source_lang
        self.target_lang = target_lang
        self.two_way = two_way
        self.asr_model_repo = asr_model_repo
        self.endpoint_delay = endpoint_delay

        # Derived byte sizes (must be int for bytearray slicing)
        self.chunk_bytes = int(SAMPLE_RATE * chunk_seconds * SAMPLE_WIDTH)
        self.stride_bytes = int(SAMPLE_RATE * stride_seconds * SAMPLE_WIDTH)

        # Audio buffer shared between stdin-reader and main threads
        self.audio_buffer = bytearray()
        self.lock = threading.Lock()
        self.running = True

        # Dedup state
        self.prev_transcript = ""
        self.prev_translation = ""

        # Utterance accumulator: original text pieces waiting for translation
        self.utterance_parts: list[str] = []
        self.utterance_source_lang: str = source_lang
        self.utterance_target_lang: str = target_lang
        self.last_loud_time: float = 0.0  # when we last saw RMS > threshold
        self.first_part_time: float = 0.0  # when the first part of current utterance arrived
        self.max_utterance_secs: float = 4.0  # force-translate after this many seconds of speech

        # Model references (populated during loading)
        self.whisper_repo: str | None = None
        self.opus_tokenizer = None
        self.opus_model = None
        self._reverse_tokenizer = None
        self._reverse_model = None

        # Pivot translation state (source → en → target when no direct model)
        self._forward_pivot = False  # forward direction needs pivot?
        self._pivot_tokenizer = None  # en → target model
        self._pivot_model = None
        self._reverse_pivot = False  # reverse direction needs pivot?
        self._reverse_pivot_tokenizer = None  # en → source model
        self._reverse_pivot_model = None

        self._load_models()

    # ------------------------------------------------------------------
    # Model loading
    # ------------------------------------------------------------------

    def _load_models(self):
        """Load MLX Whisper and Opus-MT models, then warm them up."""

        # --- Whisper (MLX) ---
        log(f"Loading Whisper model ({self.asr_model_repo})...")
        emit({"type": "status", "message": "Loading Whisper model..."})
        t0 = time.time()
        import mlx_whisper  # heavy import, done lazily

        # Warm-up: transcribe 0.1 s of silence so the model is fully loaded
        dummy_audio = np.zeros(int(SAMPLE_RATE * 0.1), dtype=np.float32)
        mlx_whisper.transcribe(
            dummy_audio,
            path_or_hf_repo=self.asr_model_repo,
            language=self.source_lang,
        )
        self.whisper_repo = self.asr_model_repo
        log(f"Whisper loaded in {time.time() - t0:.1f}s")

        # --- Opus-MT ---
        from transformers import MarianMTModel, MarianTokenizer

        lang_pair = (self.source_lang, self.target_lang)
        direct_model = OPUS_MODELS.get(lang_pair)

        if direct_model:
            # Direct pair exists — load it
            log(f"Loading Opus-MT ({direct_model})...")
            emit({"type": "status", "message": "Loading Opus-MT..."})
            t0 = time.time()
            self.opus_tokenizer = MarianTokenizer.from_pretrained(direct_model)
            self.opus_model = MarianMTModel.from_pretrained(direct_model)
            log(f"Opus-MT loaded in {time.time() - t0:.1f}s")
        else:
            # No direct pair — try pivot through English
            fwd = (self.source_lang, "en")
            bwd = ("en", self.target_lang)
            if fwd not in OPUS_MODELS or bwd not in OPUS_MODELS:
                raise ValueError(
                    f"No Opus-MT model or pivot for {lang_pair}. "
                    f"Need {fwd} and {bwd}."
                )
            self._forward_pivot = True
            fwd_name = OPUS_MODELS[fwd]
            bwd_name = OPUS_MODELS[bwd]
            log(f"Loading Opus-MT pivot: {fwd_name} → {bwd_name}...")
            emit({"type": "status", "message": "Loading Opus-MT (pivot via English)..."})
            t0 = time.time()
            # Step 1 model: source → en
            self.opus_tokenizer = MarianTokenizer.from_pretrained(fwd_name)
            self.opus_model = MarianMTModel.from_pretrained(fwd_name)
            # Step 2 model: en → target
            self._pivot_tokenizer = MarianTokenizer.from_pretrained(bwd_name)
            self._pivot_model = MarianMTModel.from_pretrained(bwd_name)
            log(f"Opus-MT pivot loaded in {time.time() - t0:.1f}s")

        # Warm-up translation
        log("Warming up translator...")
        emit({"type": "status", "message": "Warming up translator..."})
        self._translate("Hello world")

        # --- Reverse translator (preloaded for two-way mode) ---
        if self.two_way:
            from transformers import AutoTokenizer, AutoModelForSeq2SeqLM

            rev_pair = (self.target_lang, self.source_lang)
            rev_direct = OPUS_MODELS.get(rev_pair)

            if rev_direct:
                log(f"Loading reverse translator ({rev_direct})...")
                emit({"type": "status", "message": "Loading reverse translator..."})
                try:
                    self._reverse_tokenizer = AutoTokenizer.from_pretrained(rev_direct)
                    self._reverse_model = AutoModelForSeq2SeqLM.from_pretrained(rev_direct)
                    log("Reverse translator loaded")
                except Exception as e:
                    log(f"Failed to load reverse translator: {e}")
            else:
                # Reverse pivot through English
                rev_fwd = (self.target_lang, "en")
                rev_bwd = ("en", self.source_lang)
                if rev_fwd in OPUS_MODELS and rev_bwd in OPUS_MODELS:
                    self._reverse_pivot = True
                    rev_fwd_name = OPUS_MODELS[rev_fwd]
                    rev_bwd_name = OPUS_MODELS[rev_bwd]
                    log(f"Loading reverse pivot: {rev_fwd_name} → {rev_bwd_name}...")
                    emit({"type": "status", "message": "Loading reverse translator (pivot via English)..."})
                    try:
                        self._reverse_tokenizer = AutoTokenizer.from_pretrained(rev_fwd_name)
                        self._reverse_model = AutoModelForSeq2SeqLM.from_pretrained(rev_fwd_name)
                        self._reverse_pivot_tokenizer = AutoTokenizer.from_pretrained(rev_bwd_name)
                        self._reverse_pivot_model = AutoModelForSeq2SeqLM.from_pretrained(rev_bwd_name)
                        log("Reverse pivot translator loaded")
                    except Exception as e:
                        log(f"Failed to load reverse pivot: {e}")
                else:
                    log(f"No reverse model or pivot for {rev_pair}")

        log(f"Pipeline ready! (endpoint_delay={self.endpoint_delay}s)")
        emit({"type": "ready"})

    # ------------------------------------------------------------------
    # Audio utilities
    # ------------------------------------------------------------------

    @staticmethod
    def _compute_rms(pcm_bytes: bytes) -> float:
        """Compute the root-mean-square of the samples in *pcm_bytes*."""
        if len(pcm_bytes) % 2 != 0:
            pcm_bytes = pcm_bytes[:-1]
        if len(pcm_bytes) == 0:
            return 0.0
        samples = np.frombuffer(pcm_bytes, dtype=np.int16).astype(np.float32)
        return float(np.sqrt(np.mean(samples ** 2)))

    # ------------------------------------------------------------------
    # Transcription
    # ------------------------------------------------------------------

    @staticmethod
    def _is_garbage(text: str) -> bool:
        """Detect hallucinated ASR output (repeated words, filler loops)."""
        words = text.split()
        if len(words) < 2:
            return False
        # Check if same word repeats 4+ times consecutively
        norm = [re.sub(r'[^\w]', '', w).lower() for w in words]
        norm = [w for w in norm if w]  # filter empty
        if len(norm) >= 4:
            for i in range(len(norm) - 3):
                if norm[i] == norm[i+1] == norm[i+2] == norm[i+3]:
                    return True
        # Check if >70% of words are the same
        if len(norm) >= 4:
            from collections import Counter
            counts = Counter(norm)
            most_common_count = counts.most_common(1)[0][1]
            if most_common_count / len(norm) > 0.7:
                return True
        return False

    def _transcribe(self, pcm_bytes: bytes) -> tuple[str, str]:
        """Transcribe PCM s16le bytes directly using MLX Whisper."""
        import mlx_whisper

        audio_np = np.frombuffer(pcm_bytes, dtype=np.int16).astype(np.float32) / INT16_MAX

        # Use initial_prompt to improve accuracy (helps Whisper with context)
        prompt = f"The following is a speech in {self.source_lang}."
        if self.prev_transcript:
            # Feed last few words as context for better continuity
            tail = " ".join(self.prev_transcript.split()[-8:])
            prompt = tail

        result = mlx_whisper.transcribe(
            audio_np,
            path_or_hf_repo=self.whisper_repo,
            language=self.source_lang if not self.two_way else None,
            initial_prompt=prompt,
        )
        text = result.get("text", "").strip()
        detected_lang = result.get("language", self.source_lang)

        # Suppress garbage/hallucinated output
        if text and self._is_garbage(text):
            log(f"Suppressed garbage: {text[:60]}...")
            return "", detected_lang

        return text, detected_lang

    # ------------------------------------------------------------------
    # Translation
    # ------------------------------------------------------------------

    def _translate(self, text: str, target_lang: str | None = None) -> str:
        """Translate *text* using the loaded Opus-MT model(s).

        Supports direct translation or pivot through English when no
        direct model exists (e.g., zh→vi becomes zh→en→vi).
        """
        if not text or self.opus_model is None:
            return ""
        model_key = target_lang or self.target_lang

        # Determine direction and whether pivot is needed
        is_reverse = self.two_way and model_key == self.source_lang
        if is_reverse:
            tokenizer, model = self._reverse_tokenizer, self._reverse_model
            needs_pivot = self._reverse_pivot
            pivot_tokenizer, pivot_model = self._reverse_pivot_tokenizer, self._reverse_pivot_model
        else:
            tokenizer, model = self._get_translator(model_key)
            needs_pivot = self._forward_pivot
            pivot_tokenizer, pivot_model = self._pivot_tokenizer, self._pivot_model

        if tokenizer is None or model is None:
            return ""

        # Step 1: translate to intermediate (or direct if no pivot)
        inputs = tokenizer(text, return_tensors="pt", padding=True)
        outputs = model.generate(**inputs, max_new_tokens=200)
        result = tokenizer.decode(outputs[0], skip_special_tokens=True)

        # Step 2: pivot through English if needed
        if needs_pivot and pivot_tokenizer and pivot_model and result:
            inputs2 = pivot_tokenizer(result, return_tensors="pt", padding=True)
            outputs2 = pivot_model.generate(**inputs2, max_new_tokens=200)
            result = pivot_tokenizer.decode(outputs2[0], skip_special_tokens=True)

        return result

    def _get_translator(self, target_lang: str):
        """Get the translator for the given target language."""
        if not self.two_way:
            return self.opus_tokenizer, self.opus_model
        if target_lang == self.target_lang:
            return self.opus_tokenizer, self.opus_model
        return self._reverse_tokenizer, self._reverse_model

    # ------------------------------------------------------------------
    # Deduplication
    # ------------------------------------------------------------------

    def _is_cjk(self) -> bool:
        return self.source_lang in CJK_LANGS

    def _dedup(self, new_text: str, prev_text: str) -> str:
        if not prev_text or not new_text:
            return new_text
        if self._is_cjk():
            return self._dedup_characters(new_text, prev_text)
        else:
            return self._dedup_words(new_text, prev_text)

    @staticmethod
    def _norm_word(w: str) -> str:
        """Strip punctuation for comparison."""
        return re.sub(r'[^\w]', '', w).lower()

    @staticmethod
    def _dedup_characters(new_text: str, prev_text: str) -> str:
        min_overlap = 3
        max_check = min(len(prev_text), len(new_text), 100)
        best = 0
        for length in range(min_overlap, max_check + 1):
            if prev_text[-length:] == new_text[:length]:
                best = length
        if best >= min_overlap:
            remaining = new_text[best:].strip()
            return remaining if remaining else new_text
        return new_text

    @staticmethod
    def _dedup_words(new_text: str, prev_text: str) -> str:
        """Remove overlapping words between the tail of prev and the head of new.

        Uses exact prefix matching: try progressively longer overlaps,
        from max possible down to 1 word. Returns empty string when
        the entire new text is contained in the overlap.
        """
        words_new = new_text.split()
        words_prev = prev_text.split()
        if not words_prev or not words_new:
            return new_text

        # Normalize for comparison
        norm_prev = [LocalPipeline._norm_word(w) for w in words_prev]
        norm_new = [LocalPipeline._norm_word(w) for w in words_new]

        # Build index→normalized pairs (skip empty after normalization)
        prev_entries = [(i, nw) for i, nw in enumerate(norm_prev) if nw]
        new_entries = [(i, nw) for i, nw in enumerate(norm_new) if nw]

        if not prev_entries or not new_entries:
            return new_text

        # Try from longest possible overlap down to 1 word
        max_check = min(len(prev_entries), len(new_entries), 20)
        for overlap_len in range(max_check, 0, -1):
            prev_tail = [nw for _, nw in prev_entries[-overlap_len:]]
            new_head = [nw for _, nw in new_entries[:overlap_len]]

            if prev_tail == new_head:
                # Found exact overlap — skip these words in the new text
                skip_to = new_entries[overlap_len - 1][0] + 1
                remaining = words_new[skip_to:]
                result = " ".join(remaining).strip()
                # Return deduped text, or empty if entire new text was overlap
                return result

        return new_text

    # ------------------------------------------------------------------
    # Phase 1: Transcribe chunk → emit original text immediately
    # ------------------------------------------------------------------

    def _transcribe_chunk(self, pcm_bytes: bytes) -> None:
        """
        Transcribe one chunk and emit "original" immediately.
        Accumulate new text for later translation on silence.
        """
        rms = self._compute_rms(pcm_bytes)
        if rms < SILENCE_THRESHOLD:
            return

        t_asr_start = time.time()
        transcript, detected_lang = self._transcribe(pcm_bytes)
        t_asr = time.time() - t_asr_start

        if not transcript or transcript == self.prev_transcript:
            return

        new_text = self._dedup(transcript, self.prev_transcript)

        # Skip empty or very short fragments (likely noise)
        if not new_text or len(new_text) < 3:
            self.prev_transcript = transcript
            return

        # Skip 1-word fragments that are just trailing repeats
        words = new_text.split()
        if len(words) <= 1:
            norm_prev = self._norm_word(self.prev_transcript.split()[-1]) if self.prev_transcript.split() else ""
            norm_new = self._norm_word(words[0]) if words else ""
            if norm_new and norm_new == norm_prev:
                self.prev_transcript = transcript
                return

        log(f"ASR ({t_asr:.2f}s): {new_text} (lang={detected_lang})")

        # Determine target language
        if self.two_way:
            target_lang = self.source_lang if detected_lang == self.target_lang else self.target_lang
        else:
            target_lang = self.target_lang

        # Emit original text immediately
        emit({
            "type": "original",
            "text": new_text,
            "source_lang": detected_lang,
            "target_lang": target_lang,
        })

        # Accumulate for translation on silence
        self.utterance_parts.append(new_text)
        self.utterance_source_lang = detected_lang
        self.utterance_target_lang = target_lang
        if len(self.utterance_parts) == 1:
            self.first_part_time = time.time()

        self.prev_transcript = transcript

    # ------------------------------------------------------------------
    # Phase 2: Translate accumulated text on silence
    # ------------------------------------------------------------------

    def _translate_accumulated(self) -> None:
        """Translate all accumulated original text as one complete sentence."""
        if not self.utterance_parts:
            return

        # Dedup between consecutive parts to remove residual overlap
        cleaned_parts: list[str] = []
        for part in self.utterance_parts:
            if cleaned_parts:
                deduped = self._dedup(part, cleaned_parts[-1])
                cleaned_parts.append(deduped if deduped else part)
            else:
                cleaned_parts.append(part)
        full_text = " ".join(p for p in cleaned_parts if p).strip()
        self.utterance_parts = []
        self.first_part_time = 0.0

        source_lang = self.utterance_source_lang
        target_lang = self.utterance_target_lang

        t_start = time.time()
        translated = self._translate(full_text, target_lang)
        t_translate = time.time() - t_start

        if translated and self.prev_translation:
            translated = self._dedup(translated, self.prev_translation)

        log(f"Translate ({t_translate:.2f}s): {translated}")

        emit({
            "type": "result",
            "original": full_text,
            "translated": translated,
            "source_lang": source_lang,
            "target_lang": target_lang,
            "timing": {
                "asr": 0,
                "translate": round(t_translate, 2),
                "total": round(time.time() - t_start, 2),
            },
        })

        if translated:
            self.prev_translation = translated

    # ------------------------------------------------------------------
    # Stdin reader (runs in a background daemon thread)
    # ------------------------------------------------------------------

    def _stdin_reader(self) -> None:
        """Continuously read PCM bytes from stdin into audio_buffer."""
        try:
            while self.running:
                data = sys.stdin.buffer.read(4096)
                if not data:
                    break
                with self.lock:
                    self.audio_buffer.extend(data)
        except Exception as exc:
            log(f"stdin reader error: {exc}")
        finally:
            self.running = False

    # ------------------------------------------------------------------
    # Main loop: transcribe on timer, translate on silence
    # ------------------------------------------------------------------

    def run(self) -> None:
        """
        Main loop with two phases:
          - Transcription: process chunks on a sliding window (every ~0.5s check)
          - Translation: when silence exceeds endpoint_delay, translate accumulated text
        """
        reader = threading.Thread(target=self._stdin_reader, daemon=True)
        reader.start()

        processed_pos = 0
        self.last_loud_time = time.time()

        while self.running:
            time.sleep(0.1)

            with self.lock:
                buf_len = len(self.audio_buffer)

            # --- Check silence on the latest incoming audio ---
            # Grab the tail of the buffer to check if user is speaking
            if buf_len > SILENCE_CHECK_BYTES:
                with self.lock:
                    tail = bytes(self.audio_buffer[-SILENCE_CHECK_BYTES:])
                rms = self._compute_rms(tail)
                if rms >= SILENCE_THRESHOLD:
                    self.last_loud_time = time.time()

            # --- Phase 2: Check if silence exceeded endpoint_delay ---
            silence_duration = time.time() - self.last_loud_time
            if silence_duration >= self.endpoint_delay and self.utterance_parts:
                log(f"Silence {silence_duration:.1f}s >= {self.endpoint_delay}s, translating {len(self.utterance_parts)} parts")
                self._translate_accumulated()
                self.last_loud_time = time.time()

            # --- Phase 2b: Force-translate if utterance is too long (continuous speech) ---
            if self.utterance_parts and self.first_part_time > 0:
                utterance_age = time.time() - self.first_part_time
                if utterance_age >= self.max_utterance_secs:
                    log(f"Max utterance {utterance_age:.1f}s >= {self.max_utterance_secs}s, force-translating {len(self.utterance_parts)} parts")
                    self._translate_accumulated()
                    self.last_loud_time = time.time()

            # --- Phase 1: Transcribe chunk if we have enough data ---
            if buf_len - processed_pos >= self.chunk_bytes:
                with self.lock:
                    chunk = bytes(
                        self.audio_buffer[processed_pos : processed_pos + self.chunk_bytes]
                    )
                self._transcribe_chunk(chunk)
                processed_pos += self.stride_bytes

                # Trim consumed bytes to prevent unbounded memory growth
                if processed_pos > self.chunk_bytes:
                    with self.lock:
                        del self.audio_buffer[:processed_pos]
                    processed_pos = 0

        # Drain: translate any remaining accumulated text
        self._translate_accumulated()

        emit({"type": "done"})
        log("Pipeline stopped.")


# ---------------------------------------------------------------------------
# CLI entry point
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(
        description="Auralis local translation pipeline (MLX Whisper + Opus-MT)"
    )
    parser.add_argument(
        "--source-lang",
        default="en",
        help="Source language code (en, vi, es, fr, de, zh, ja)",
    )
    parser.add_argument(
        "--target-lang",
        default="vi",
        help="Target language code (vi, en, es, fr, de, zh, ja)",
    )
    parser.add_argument(
        "--asr-model",
        default="mlx-community/whisper-large-v3-turbo",
        help="Hugging Face repo for MLX Whisper model",
    )
    parser.add_argument(
        "--two-way",
        action="store_true",
        default=False,
        help="Enable two-way translation with language auto-detection",
    )
    parser.add_argument(
        "--chunk-seconds",
        type=float,
        default=2,
        help="Audio chunk size in seconds (default: 2)",
    )
    parser.add_argument(
        "--stride-seconds",
        type=float,
        default=1.5,
        help="Stride between chunks in seconds (default: 1.5). Must be <= chunk-seconds.",
    )
    parser.add_argument(
        "--endpoint-delay",
        type=float,
        default=1.0,
        help="Seconds of silence before translating accumulated text (default: 1.0)",
    )
    args = parser.parse_args()

    pipeline = LocalPipeline(
        source_lang=args.source_lang,
        target_lang=args.target_lang,
        asr_model_repo=args.asr_model,
        two_way=args.two_way,
        chunk_seconds=args.chunk_seconds,
        stride_seconds=args.stride_seconds,
        endpoint_delay=args.endpoint_delay,
    )
    pipeline.run()


if __name__ == "__main__":
    main()
