#!/usr/bin/env python3
"""
Local translation pipeline sidecar for Auralis.
Receives PCM audio via stdin, transcribes with MLX Whisper, translates with Opus-MT.
Outputs JSON results via stdout.

Protocol:
  stdin  -> raw PCM s16le 16kHz mono bytes (continuous stream)
  stdout -> JSON lines:
    {"type":"status","message":"Loading Whisper model..."}
    {"type":"status","message":"Loading Opus-MT..."}
    {"type":"ready"}
    {"type":"result","original":"...","translated":"...","timing":{"asr":1.23,"translate":0.45,"total":1.68}}
    {"type":"done"}
  stderr -> log messages

Usage:
  python3 local_pipeline.py --source-lang en --target-lang vi [--asr-model mlx-community/whisper-large-v3-turbo]
"""

import sys
import os
import json
import time
import wave
import tempfile
import argparse
import threading

# Suppress tokenizers parallelism warning
os.environ["TOKENIZERS_PARALLELISM"] = "false"

# ---------------------------------------------------------------------------
# Audio constants
# ---------------------------------------------------------------------------
SAMPLE_RATE = 16000
CHANNELS = 1
SAMPLE_WIDTH = 2  # s16le
CHUNK_SECONDS = 3
STRIDE_SECONDS = 2
CHUNK_BYTES = SAMPLE_RATE * CHUNK_SECONDS * SAMPLE_WIDTH  # 96000
STRIDE_BYTES = SAMPLE_RATE * STRIDE_SECONDS * SAMPLE_WIDTH  # 64000
SILENCE_THRESHOLD = 100  # RMS below this is treated as silence


# ---------------------------------------------------------------------------
# Opus-MT model registry
# ---------------------------------------------------------------------------
OPUS_MODELS = {
    ("en", "vi"): "Helsinki-NLP/opus-mt-en-vi",
    ("en", "es"): "Helsinki-NLP/opus-mt-en-es",
    ("en", "fr"): "Helsinki-NLP/opus-mt-en-fr",
    ("en", "de"): "Helsinki-NLP/opus-mt-en-de",
    ("en", "zh"): "Helsinki-NLP/opus-mt-en-zh",
    ("en", "ja"): "Helsinki-NLP/opus-mt-en-ja",
    ("vi", "en"): "Helsinki-NLP/opus-mt-vi-en",
    ("es", "en"): "Helsinki-NLP/opus-mt-es-en",
    ("fr", "en"): "Helsinki-NLP/opus-mt-fr-en",
    ("de", "en"): "Helsinki-NLP/opus-mt-de-en",
    ("zh", "en"): "Helsinki-NLP/opus-mt-zh-en",
    ("ja", "en"): "Helsinki-NLP/opus-mt-ja-en",
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
        chunk_seconds: int = CHUNK_SECONDS,
        stride_seconds: int = STRIDE_SECONDS,
    ):
        self.source_lang = source_lang
        self.target_lang = target_lang
        self.asr_model_repo = asr_model_repo
        self.chunk_seconds = chunk_seconds
        self.stride_seconds = stride_seconds

        # Derived byte sizes
        self.chunk_bytes = SAMPLE_RATE * self.chunk_seconds * SAMPLE_WIDTH
        self.stride_bytes = SAMPLE_RATE * self.stride_seconds * SAMPLE_WIDTH

        # Audio buffer shared between stdin-reader and main threads
        self.audio_buffer = bytearray()
        self.lock = threading.Lock()
        self.running = True

        # Previous transcription for dedup
        self.prev_transcript = ""
        self.prev_translation = ""

        # Model references (populated during loading)
        self.whisper_repo: str | None = None
        self.opus_tokenizer = None
        self.opus_model = None

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
        import numpy as np

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
        lang_pair = (self.source_lang, self.target_lang)
        model_name = OPUS_MODELS.get(lang_pair)
        if model_name is None:
            raise ValueError(
                f"No Opus-MT model for language pair {lang_pair}. "
                f"Supported pairs: {list(OPUS_MODELS.keys())}"
            )

        log(f"Loading Opus-MT ({model_name})...")
        emit({"type": "status", "message": "Loading Opus-MT..."})
        t0 = time.time()
        from transformers import MarianMTModel, MarianTokenizer

        self.opus_tokenizer = MarianTokenizer.from_pretrained(model_name)
        self.opus_model = MarianMTModel.from_pretrained(model_name)
        log(f"Opus-MT loaded in {time.time() - t0:.1f}s")

        # Warm-up translation
        log("Warming up translator...")
        emit({"type": "status", "message": "Warming up translator..."})
        self._translate("Hello world")

        log("Pipeline ready!")
        emit({"type": "ready"})

    # ------------------------------------------------------------------
    # Audio utilities
    # ------------------------------------------------------------------

    @staticmethod
    def _save_pcm_as_wav(pcm_bytes: bytes) -> str:
        """Write raw PCM s16le bytes to a temporary WAV file and return its path."""
        tmp = tempfile.NamedTemporaryFile(suffix=".wav", delete=False)
        with wave.open(tmp.name, "w") as wf:
            wf.setnchannels(CHANNELS)
            wf.setsampwidth(SAMPLE_WIDTH)
            wf.setframerate(SAMPLE_RATE)
            wf.writeframes(pcm_bytes)
        return tmp.name

    @staticmethod
    def _compute_rms(pcm_bytes: bytes) -> float:
        """Compute the root-mean-square of the samples in *pcm_bytes*."""
        import numpy as np
        samples = np.frombuffer(pcm_bytes, dtype=np.int16).astype(np.float32)
        return float(np.sqrt(np.mean(samples ** 2)))

    # ------------------------------------------------------------------
    # Transcription
    # ------------------------------------------------------------------

    def _transcribe(self, wav_path: str) -> str:
        """Return the transcribed text for the WAV at *wav_path*."""
        import mlx_whisper
        import numpy as np

        # Load WAV as float32 numpy (avoids ffmpeg dependency)
        with wave.open(wav_path, "r") as wf:
            raw = wf.readframes(wf.getnframes())
            audio_np = np.frombuffer(raw, dtype=np.int16).astype(np.float32) / 32768.0

        result = mlx_whisper.transcribe(
            audio_np,
            path_or_hf_repo=self.whisper_repo,
            language=self.source_lang,
        )
        return result.get("text", "").strip()

    # ------------------------------------------------------------------
    # Translation
    # ------------------------------------------------------------------

    def _translate(self, text: str) -> str:
        """Translate *text* using the loaded Opus-MT model."""
        if not text or self.opus_model is None:
            return ""
        inputs = self.opus_tokenizer(text, return_tensors="pt", padding=True)
        outputs = self.opus_model.generate(**inputs, max_new_tokens=200)
        return self.opus_tokenizer.decode(outputs[0], skip_special_tokens=True)

    # ------------------------------------------------------------------
    # Deduplication
    # ------------------------------------------------------------------

    def _is_cjk(self) -> bool:
        """Return True if the source language uses character-based writing."""
        return self.source_lang in CJK_LANGS

    def _dedup(self, new_text: str, prev_text: str) -> str:
        """
        Remove overlap between *prev_text* suffix and *new_text* prefix.
        For CJK languages the overlap is measured in characters;
        for others it is measured in words.
        Minimum overlap to trigger removal: 3 units (characters or words).
        """
        if not prev_text or not new_text:
            return new_text

        if self._is_cjk():
            return self._dedup_characters(new_text, prev_text)
        else:
            return self._dedup_words(new_text, prev_text)

    @staticmethod
    def _dedup_characters(new_text: str, prev_text: str) -> str:
        """Character-level dedup for CJK languages."""
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
        """Word-level dedup for non-CJK languages."""
        words_new = new_text.split()
        words_prev = prev_text.split()
        min_overlap = 3
        if len(words_prev) < min_overlap or len(words_new) < min_overlap:
            return new_text
        max_check = min(len(words_prev), len(words_new))
        best = 0
        for length in range(min_overlap, max_check + 1):
            suffix = " ".join(words_prev[-length:])
            prefix = " ".join(words_new[:length])
            if suffix.lower() == prefix.lower():
                best = length
        if best >= min_overlap:
            remaining = " ".join(words_new[best:]).strip()
            return remaining if remaining else new_text
        return new_text

    # ------------------------------------------------------------------
    # Chunk processing
    # ------------------------------------------------------------------

    def _process_chunk(self, pcm_bytes: bytes) -> None:
        """Process a single audio chunk: silence check -> transcribe -> dedup -> translate -> emit."""
        t_start = time.time()

        # Silence detection
        rms = self._compute_rms(pcm_bytes)
        if rms < SILENCE_THRESHOLD:
            return

        # Save to temp WAV for Whisper
        wav_path = self._save_pcm_as_wav(pcm_bytes)

        try:
            # Step 1: transcribe
            t_asr_start = time.time()
            transcript = self._transcribe(wav_path)
            t_asr = time.time() - t_asr_start

            if not transcript or transcript == self.prev_transcript:
                return

            # Step 2: deduplicate transcript against previous
            new_text = self._dedup(transcript, self.prev_transcript)
            if not new_text or len(new_text) < 3:
                self.prev_transcript = transcript
                return

            log(f"Transcript: {transcript}")
            log(f"New text:   {new_text}")

            # Step 3: translate
            t_trans_start = time.time()
            translated = self._translate(new_text)
            t_translate = time.time() - t_trans_start

            # Step 4: deduplicate translation
            if translated and self.prev_translation:
                translated = self._dedup(translated, self.prev_translation)

            total = time.time() - t_start
            log(f"ASR={t_asr:.2f}s  translate={t_translate:.2f}s  total={total:.2f}s")

            # Step 5: emit result
            emit({
                "type": "result",
                "original": new_text,
                "translated": translated,
                "timing": {
                    "asr": round(t_asr, 2),
                    "translate": round(t_translate, 2),
                    "total": round(total, 2),
                },
            })

            # Store for next iteration's dedup
            self.prev_transcript = transcript
            if translated:
                self.prev_translation = translated

        finally:
            # Clean up temp WAV
            try:
                os.unlink(wav_path)
            except OSError:
                pass

    # ------------------------------------------------------------------
    # Stdin reader (runs in a background daemon thread)
    # ------------------------------------------------------------------

    def _stdin_reader(self) -> None:
        """Continuously read PCM bytes from stdin and append to the shared buffer."""
        try:
            while self.running:
                data = sys.stdin.buffer.read(4096)
                if not data:
                    break  # EOF
                with self.lock:
                    self.audio_buffer.extend(data)
        except Exception as exc:
            log(f"stdin reader error: {exc}")
        finally:
            self.running = False

    # ------------------------------------------------------------------
    # Main loop
    # ------------------------------------------------------------------

    def run(self) -> None:
        """
        Main loop: a background thread reads stdin into audio_buffer;
        this thread wakes every 500 ms, extracts CHUNK_BYTES-sized windows
        with STRIDE_BYTES stride, and processes each one.
        """
        # Start the stdin reader daemon
        reader = threading.Thread(target=self._stdin_reader, daemon=True)
        reader.start()

        processed_pos = 0  # byte offset into audio_buffer already consumed

        while self.running:
            time.sleep(0.5)  # check every 500 ms

            with self.lock:
                available = len(self.audio_buffer) - processed_pos

            if available >= self.chunk_bytes:
                with self.lock:
                    chunk = bytes(
                        self.audio_buffer[processed_pos : processed_pos + self.chunk_bytes]
                    )
                self._process_chunk(chunk)
                processed_pos += self.stride_bytes

        # Drain remaining audio (at least 1 second worth)
        with self.lock:
            remaining = len(self.audio_buffer) - processed_pos
            if remaining > SAMPLE_RATE * SAMPLE_WIDTH:
                chunk = bytes(self.audio_buffer[processed_pos:])
                self._process_chunk(chunk)

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
    args = parser.parse_args()

    pipeline = LocalPipeline(
        source_lang=args.source_lang,
        target_lang=args.target_lang,
        asr_model_repo=args.asr_model,
    )
    pipeline.run()


if __name__ == "__main__":
    main()
