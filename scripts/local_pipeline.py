#!/usr/bin/env python3
"""
Local translation pipeline sidecar for Auralis.
Receives PCM audio via stdin, transcribes with MLX Whisper, translates with Gemma-3 LLM.
Outputs JSON results via stdout.

Protocol:
  stdin  -> raw PCM s16le 16kHz mono bytes (continuous stream, no flush markers)
  stdout -> JSON lines:
    {"type":"status","message":"Loading Whisper model..."}
    {"type":"ready"}
    {"type":"result","original":"...","translated":"...","source_lang":"en","target_lang":"vi"}
    {"type":"done"}
  stderr -> log messages

Architecture (single-phase):
  Sliding window (5s chunks, 3.5s stride).
  Each chunk: RMS check -> transcribe -> deduplicate -> translate -> emit "result".

  This gives clean, complete transcriptions (Whisper works best with longer audio)
  and emits both original + translated text together for each chunk.

Usage:
  python3 local_pipeline.py --source-lang en --target-lang vi
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
CHUNK_SECONDS = 7
STRIDE_SECONDS = 5
CHUNK_BYTES = int(SAMPLE_RATE * CHUNK_SECONDS * SAMPLE_WIDTH)
STRIDE_BYTES = int(SAMPLE_RATE * STRIDE_SECONDS * SAMPLE_WIDTH)
SILENCE_THRESHOLD = 100  # RMS below this is treated as silence
INT16_MAX = 32768.0

# CJK language codes -- use character-level dedup for these
CJK_LANGS = {"zh", "ja", "ko"}

# Language display names for translation prompts
LANG_NAMES = {
    "vi": "Vietnamese", "en": "English", "ja": "Japanese",
    "ko": "Korean", "zh": "Chinese", "fr": "French",
    "de": "German", "es": "Spanish", "th": "Thai",
    "pt": "Portuguese", "ru": "Russian", "ar": "Arabic",
    "hi": "Hindi", "it": "Italian", "nl": "Dutch",
}

# VAD constants
SILENCE_THRESHOLD = 50  # Lowered from 100 for better sensitivity
MIN_SPEECH_DURATION = 0.3  # Minimum 300ms of speech
MAX_SILENCE_DURATION = 1.5  # Cut off after 1.5s of silence


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
# Hybrid VAD (Voice Activity Detection)
# ---------------------------------------------------------------------------

class HybridVAD:
    """
    Hybrid Voice Activity Detection combining multiple features:
    - RMS energy (loudness)
    - Zero-crossing rate (fricative detection)
    - Spectral centroid (voiced/unvoiced distinction)
    """

    def __init__(self):
        self.rms_threshold = SILENCE_THRESHOLD
        self.energy_history = []
        self.history_size = 100

    def _compute_rms(self, pcm_bytes: bytes) -> float:
        """Compute root-mean-square energy."""
        if len(pcm_bytes) % 2 != 0:
            pcm_bytes = pcm_bytes[:-1]
        if len(pcm_bytes) == 0:
            return 0.0
        samples = np.frombuffer(pcm_bytes, dtype=np.int16).astype(np.float32)
        return float(np.sqrt(np.mean(samples ** 2)))

    def _compute_zero_crossing_rate(self, pcm_bytes: bytes) -> float:
        """Compute zero-crossing rate (high for fricatives like 's', 'f')."""
        if len(pcm_bytes) % 2 != 0:
            pcm_bytes = pcm_bytes[:-1]
        if len(pcm_bytes) < 4:
            return 0.0
        samples = np.frombuffer(pcm_bytes, dtype=np.int16)
        # Count sign changes
        crossings = np.sum(np.abs(np.diff(np.signbit(samples.astype(np.int16)))))
        return float(crossings) / len(samples)

    def _compute_spectral_centroid(self, pcm_bytes: bytes) -> float:
        """Compute spectral centroid (higher for voiced speech)."""
        if len(pcm_bytes) % 2 != 0:
            pcm_bytes = pcm_bytes[:-1]
        if len(pcm_bytes) < 4:
            return 0.0
        samples = np.frombuffer(pcm_bytes, dtype=np.int16).astype(np.float32)

        # Compute magnitude spectrum using FFT
        spectrum = np.abs(np.fft.rfft(samples))
        # Normalize by total energy
        total_energy = np.sum(spectrum) + 1e-10  # Avoid division by zero
        # Weight frequencies by magnitude
        freqs = np.fft.rfftfreq(len(samples), d=1.0/SAMPLE_RATE)
        centroid = np.sum(freqs * spectrum) / total_energy
        return float(centroid)

    def is_speech(self, pcm_bytes: bytes) -> bool:
        """
        Determine if audio chunk contains speech using hybrid features.

        Returns:
            True if speech is detected, False otherwise
        """
        rms = self._compute_rms(pcm_bytes)
        zcr = self._compute_zero_crossing_rate(pcm_bytes)
        spectral_centroid = self._compute_spectral_centroid(pcm_bytes)

        # Adaptive threshold based on recent history
        if len(self.energy_history) > 10:
            self.rms_threshold = np.percentile(self.energy_history, 30)

        self.energy_history.append(rms)
        if len(self.energy_history) > self.history_size:
            self.energy_history.pop(0)

        # Combined decision using weighted features
        # RMS energy is most important (50%)
        energy_score = min(1.0, rms / (self.rms_threshold * 2)) if rms > self.rms_threshold else 0

        # Zero-crossing rate helps detect fricatives (30%)
        zcr_score = min(1.0, zcr / 0.15) if zcr > 0.05 else 0

        # Spectral centroid helps distinguish voiced speech (20%)
        centroid_score = min(1.0, spectral_centroid / 1500) if spectral_centroid > 500 else 0

        # Combined score
        combined_score = (energy_score * 0.5 +
                         zcr_score * 0.3 +
                         centroid_score * 0.2)

        return combined_score > 0.4  # Threshold for speech detection


# ---------------------------------------------------------------------------
# Translation Cache (LRU)
# ---------------------------------------------------------------------------

class TranslationCache:
    """Simple LRU cache for translations to avoid redundant LLM calls."""

    def __init__(self, max_size=1000):
        self.cache = {}
        self.max_size = max_size
        self.access_order = []

    def get(self, text, target_lang):
        """Get cached translation if available."""
        key = f"{text}:{target_lang}"
        if key in self.cache:
            # Update access order
            self.access_order.remove(key)
            self.access_order.append(key)
            return self.cache[key]
        return None

    def set(self, text, target_lang, translation):
        """Cache a translation."""
        key = f"{text}:{target_lang}"

        # Remove oldest if at capacity
        if len(self.cache) >= self.max_size and key not in self.cache:
            oldest = self.access_order.pop(0)
            del self.cache[oldest]

        self.cache[key] = translation
        if key not in self.access_order:
            self.access_order.append(key)

    def clear(self):
        """Clear the cache."""
        self.cache.clear()
        self.access_order.clear()

    def stats(self):
        """Return cache statistics."""
        return {
            "size": len(self.cache),
            "max_size": self.max_size,
            "utilization": len(self.cache) / self.max_size * 100
        }


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
    ):
        self.source_lang = source_lang
        self.target_lang = target_lang
        self.source_lang_name = LANG_NAMES.get(source_lang, source_lang)
        self.target_lang_name = LANG_NAMES.get(target_lang, target_lang)
        self.two_way = two_way
        self.asr_model_repo = asr_model_repo

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

        # Rolling context for translation continuity
        self.context_history = []  # list of (original, translated) tuples
        self.max_context = 5

        # Translation cache to avoid redundant LLM calls
        self.translation_cache = TranslationCache(max_size=1000)

        # Hybrid VAD for better speech detection
        self.vad = HybridVAD()

        # Model references (populated during loading)
        self.whisper_repo: str | None = None
        self.llm_model = None
        self.llm_tokenizer = None

        self._load_models()

    # ------------------------------------------------------------------
    # Model loading
    # ------------------------------------------------------------------

    def _load_whisper(self) -> None:
        """Load MLX Whisper model + warm-up. Runs in its own thread."""
        log(f"Loading Whisper model ({self.asr_model_repo})...")
        emit({"type": "status", "message": "Loading Whisper model..."})
        t0 = time.time()
        import mlx_whisper

        dummy_audio = np.zeros(int(SAMPLE_RATE * 0.1), dtype=np.float32)
        mlx_whisper.transcribe(
            dummy_audio,
            path_or_hf_repo=self.asr_model_repo,
            language=self.source_lang,
        )
        self.whisper_repo = self.asr_model_repo
        log(f"Whisper loaded in {time.time() - t0:.1f}s")

    def _load_translator(self) -> None:
        """Load Gemma-3 LLM translator. Runs in its own thread."""
        log("Loading Gemma-3-4B translator...")
        emit({"type": "status", "message": "Loading Gemma-3-4B translator..."})
        t0 = time.time()
        from mlx_lm import load
        self.llm_model, self.llm_tokenizer = load(
            "mlx-community/gemma-3-4b-it-qat-4bit"
        )
        log(f"Gemma-3-4B loaded in {time.time() - t0:.1f}s")
        # Skip warm-up generate() — it can crash when Whisper is loading
        # in parallel due to GPU memory contention. First real translation
        # will be slightly slower but safe.

    def _load_models(self):
        """Load Whisper first, then Gemma-3 LLM sequentially to avoid GPU contention."""
        t0 = time.time()

        # Load Whisper first — it's fast (~12s) and needs GPU alone
        self._load_whisper()

        # Then load Gemma-3 — it's large (2.5GB) and would starve Whisper if parallel
        self._load_translator()

        log(f"All models loaded in {time.time() - t0:.1f}s")
        log("Pipeline ready!")
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
        prompt = "The following is a speech transcription."
        if self.prev_transcript:
            tail = " ".join(self.prev_transcript.split()[-8:])
            prompt = tail

        result = mlx_whisper.transcribe(
            audio_np,
            path_or_hf_repo=self.whisper_repo,
            language=None,  # Enable auto-detection for all modes
            initial_prompt=prompt,
        )
        text = result.get("text", "").strip()
        detected_lang = result.get("language", self.source_lang)

        return text, detected_lang

    # ------------------------------------------------------------------
    # Translation (Gemma-3 LLM with rolling context)
    # ------------------------------------------------------------------

    def _translate(self, text: str, target_lang: str | None = None) -> str:
        """Translate *text* using Gemma-3 LLM with rolling context.

        Produces natural, context-aware translations using the
        Gemma-3-4B-IT model quantized to 4-bit via mlx-lm.
        """
        if not text or self.llm_model is None:
            return ""

        target = target_lang or self.target_lang

        # Check cache first to avoid redundant LLM calls
        cached = self.translation_cache.get(text, target)
        if cached is not None:
            return cached

        target_name = LANG_NAMES.get(target, target)
        source_name = LANG_NAMES.get(self.source_lang, self.source_lang)

        from mlx_lm import generate

        # Build rolling context from recent translations
        context_block = ""
        if self.context_history:
            recent = self.context_history[-self.max_context:]
            ctx_originals = " / ".join(orig for orig, _ in recent)
            context_block = (
                f"[Topic context: {ctx_originals}]\n\n"
            )

        prompt = (
            "<start_of_turn>user\n"
            f"Translate this ONE {source_name} sentence to {target_name}.\n"
            f"Output ONLY the {target_name} translation of the LAST line. Do NOT repeat previous content.\n"
            "\n"
            f"Rules: {target_name} only. ONE sentence output only. Natural, fluent translation.\n"
            "\n"
            f"{context_block}"
            f"Translate: {text}\n"
            "<end_of_turn>\n"
            "<start_of_turn>model\n"
        )

        # Dynamic token allocation based on input length (min 200, max 512)
        input_tokens = len(text.split())
        max_tokens = min(512, max(200, input_tokens * 2))

        result = generate(
            self.llm_model,
            self.llm_tokenizer,
            prompt=prompt,
            max_tokens=max_tokens,
        )

        # Post-process: clean up LLM output
        result = self._clean_translation(result)

        # Dedup: remove overlap with previous translation
        if result and self.context_history:
            last_trans = self.context_history[-1][1]
            result = self._remove_overlap(result, last_trans)

        # Add to context history
        if result:
            self.context_history.append((text, result))
            if len(self.context_history) > self.max_context * 2:
                self.context_history = self.context_history[-self.max_context:]

            # Cache the translation for future use
            self.translation_cache.set(text, target, result)

        return result

    @staticmethod
    def _clean_translation(text: str) -> str:
        """Remove special tokens and truncate at hallucination."""
        # Remove Gemma special tokens
        text = text.split('<end_of_turn>')[0]
        text = re.sub(r'<[^>]+>', '', text)
        # Take only the first meaningful line
        lines = [l.strip() for l in text.split('\n') if l.strip()]
        text = lines[0] if lines else ''
        # Remove any prefix artifacts
        text = re.sub(r'^(VI:\s*|EN:\s*|→\s*|Translate:\s*|Translation:\s*)', '', text)
        # Clean up whitespace
        text = re.sub(r'\s+', ' ', text).strip()
        return text

    @staticmethod
    def _remove_overlap(new_text: str, prev_text: str) -> str:
        """Remove text from new_text that overlaps with prev_text."""
        if not prev_text or not new_text:
            return new_text
        words_new = new_text.split()
        words_prev = prev_text.split()
        if len(words_prev) < 3 or len(words_new) < 3:
            return new_text
        max_overlap = min(len(words_new), len(words_prev))
        overlap_len = 0
        for i in range(3, max_overlap + 1):
            suffix = ' '.join(words_prev[-i:])
            prefix = ' '.join(words_new[:i])
            if suffix.lower() == prefix.lower():
                overlap_len = i
        if overlap_len >= 3:
            return ' '.join(words_new[overlap_len:]).strip()
        return new_text

    # ------------------------------------------------------------------
    # Deduplication (ASR transcript overlap removal)
    # ------------------------------------------------------------------

    def _is_cjk(self) -> bool:
        return self.source_lang in CJK_LANGS

    def _dedup(self, new_text: str, prev_text: str) -> str:
        """Remove overlap between previous chunk tail and new chunk head.

        Uses character-level matching for all languages (same as my-translator).
        """
        if not prev_text or not new_text:
            return new_text
        return self._dedup_characters(new_text, prev_text)

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
        """Remove overlapping words between the tail of prev and the head of new."""
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
                return result

        return new_text

    # ------------------------------------------------------------------
    # Process one chunk: transcribe → dedup → translate → emit result
    # ------------------------------------------------------------------

    def _process_chunk(self, pcm_bytes: bytes) -> None:
        """Transcribe one chunk, translate new text, and emit a single result."""
        # Use hybrid VAD for better speech detection
        if not self.vad.is_speech(pcm_bytes):
            return

        t_start = time.time()

        # Step 1: Transcribe
        t1 = time.time()
        transcript, detected_lang = self._transcribe(pcm_bytes)
        t_asr = time.time() - t1

        if not transcript or transcript == self.prev_transcript:
            return

        # Skip hallucinated ASR output (repeated words, filler loops)
        if self._is_garbage(transcript):
            log(f"Garbage detected, skipping: {transcript[:60]}")
            self.prev_transcript = transcript
            return

        # Step 2: Deduplicate against previous chunk
        new_text = self._dedup(transcript, self.prev_transcript)

        # Skip empty or very short fragments (likely noise)
        if not new_text or len(new_text) < 3:
            self.prev_transcript = transcript
            return

        log(f"Transcript: {transcript}")
        log(f"New text:   {new_text}")

        # Determine target language
        if self.two_way:
            target_lang = self.source_lang if detected_lang == self.target_lang else self.target_lang
        else:
            target_lang = self.target_lang

        # Emit original text immediately so the UI shows it right away
        emit({
            "type": "original",
            "text": new_text,
            "source_lang": detected_lang,
            "target_lang": target_lang,
        })

        # Step 3: Translate
        t2 = time.time()
        translated = self._translate(new_text, target_lang)
        t_translate = time.time() - t2

        total = time.time() - t_start
        log(f"ASR={t_asr:.2f}s Translate={t_translate:.2f}s total={total:.2f}s")

        # Step 4: Emit combined result with translation (replaces pending segment)
        emit({
            "type": "result",
            "original": new_text,
            "translated": translated,
            "source_lang": detected_lang,
            "target_lang": target_lang,
            "timing": {
                "asr": round(t_asr, 2),
                "translate": round(t_translate, 2),
                "total": round(total, 2),
            },
        })

        if translated:
            self.prev_translation = translated
        self.prev_transcript = transcript

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
    # Main loop: process chunks with sliding window
    # ------------------------------------------------------------------

    def run(self) -> None:
        """
        Main loop: read audio, process chunks with sliding window.
        Each chunk is transcribed → deduped → translated → emitted as one result.
        """
        reader = threading.Thread(target=self._stdin_reader, daemon=True)
        reader.start()

        processed_pos = 0

        while self.running:
            time.sleep(0.5)

            with self.lock:
                buf_len = len(self.audio_buffer)

            # Process chunk when we have enough data
            if buf_len - processed_pos >= self.chunk_bytes:
                with self.lock:
                    chunk = bytes(
                        self.audio_buffer[processed_pos : processed_pos + self.chunk_bytes]
                    )
                self._process_chunk(chunk)
                processed_pos += self.stride_bytes

                # Trim consumed bytes to prevent unbounded memory growth
                if processed_pos > self.chunk_bytes:
                    with self.lock:
                        del self.audio_buffer[:processed_pos]
                    processed_pos = 0

        emit({"type": "done"})
        log("Pipeline stopped.")


# ---------------------------------------------------------------------------
# CLI entry point
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(
        description="Auralis local translation pipeline (MLX Whisper + Gemma-3 LLM)"
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
        default=7,
        help="Audio chunk size in seconds (default: 7)",
    )
    parser.add_argument(
        "--stride-seconds",
        type=float,
        default=5,
        help="Stride between chunks in seconds (default: 5). Must be <= chunk-seconds.",
    )
    args = parser.parse_args()

    pipeline = LocalPipeline(
        source_lang=args.source_lang,
        target_lang=args.target_lang,
        asr_model_repo=args.asr_model,
        two_way=args.two_way,
        chunk_seconds=args.chunk_seconds,
        stride_seconds=args.stride_seconds,
    )
    pipeline.run()


if __name__ == "__main__":
    main()
