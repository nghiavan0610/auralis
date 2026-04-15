#!/usr/bin/env python3
"""
Confidence estimation for MLX Whisper transcriptions.
Provides production-ready confidence scoring for offline mode.

This module implements ensemble confidence estimation using multiple signals:
- Acoustic confidence (audio signal quality)
- Language confidence (detection certainty)
- Linguistic confidence (text quality indicators)
"""

import numpy as np
from typing import List, Tuple, Optional, Dict
import re


class ConfidenceEstimator:
    """
    Ensemble confidence estimator for Whisper transcriptions.

    Combines multiple signals to produce a reliable confidence score:
    - Acoustic: Audio signal quality metrics
    - Language: Detection confidence
    - Linguistic: Text quality and coherence
    """

    def __init__(
        self,
        acoustic_weight: float = 0.4,
        language_weight: float = 0.3,
        linguistic_weight: float = 0.3
    ):
        """
        Initialize confidence estimator.

        Args:
            acoustic_weight: Weight for acoustic confidence (0-1)
            language_weight: Weight for language confidence (0-1)
            linguistic_weight: Weight for linguistic confidence (0-1)

        Raises:
            ValueError: If weights don't sum to approximately 1.0
        """
        total_weight = acoustic_weight + language_weight + linguistic_weight
        if not (0.99 <= total_weight <= 1.01):
            raise ValueError(f"Weights must sum to 1.0, got {total_weight}")

        self.acoustic_weight = acoustic_weight
        self.language_weight = language_weight
        self.linguistic_weight = linguistic_weight

        # Supported languages with high confidence
        self.supported_languages = {
            "en", "vi", "ja", "ko", "zh", "fr", "de", "es",
            "th", "pt", "ru", "ar", "hi", "it", "nl"
        }

    def estimate_confidence(
        self,
        audio: np.ndarray,
        transcript: str,
        detected_language: str,
        segments: Optional[List[Dict]] = None
    ) -> float:
        """
        Estimate confidence using ensemble of multiple signals.

        Args:
            audio: Audio samples (float32, normalized to [-1, 1])
            transcript: Transcribed text
            detected_language: Detected language code
            segments: Optional list of transcription segments

        Returns:
            Confidence score between 0.0 and 1.0
        """
        # Acoustic confidence (audio quality)
        acoustic_conf = self._estimate_acoustic_confidence(audio)

        # Language confidence (detection certainty)
        lang_conf = self._estimate_language_confidence(detected_language)

        # Linguistic confidence (text quality)
        linguistic_conf = self._estimate_linguistic_confidence(
            transcript, segments or []
        )

        # Weighted combination
        confidence = (
            acoustic_conf * self.acoustic_weight +
            lang_conf * self.language_weight +
            linguistic_conf * self.linguistic_weight
        )

        # Clamp to valid range
        return max(0.0, min(1.0, confidence))

    def _estimate_acoustic_confidence(self, audio: np.ndarray) -> float:
        """
        Estimate confidence from audio signal quality.

        Factors:
        - RMS energy level
        - Dynamic range
        - Zero-crossing rate (speech vs noise)
        """
        if len(audio) == 0:
            return 0.0

        # RMS energy
        rms = float(np.sqrt(np.mean(audio ** 2)))

        # Dynamic range
        dynamic_range = float(np.max(audio) - np.min(audio))

        # Zero-crossing rate (helps distinguish speech from noise)
        zero_crossings = np.sum(np.abs(np.diff(np.sign(audio)))) / len(audio)

        # Normalize RMS energy (typical speech: 0.01-0.1)
        energy_score = min(1.0, rms / 0.05)

        # Normalize dynamic range (good speech has variation)
        range_score = min(1.0, dynamic_range / 0.5)

        # Zero-crossing rate (speech typically 0.1-0.5)
        zcr_score = 1.0 - min(1.0, abs(zero_crossings - 0.3) / 0.3)

        # Combine factors
        acoustic_conf = (energy_score * 0.5 +
                        range_score * 0.3 +
                        zcr_score * 0.2)

        return acoustic_conf

    def _estimate_language_confidence(self, detected_lang: str) -> float:
        """
        Estimate language detection confidence.

        In a full implementation, this would use language probabilities
        from Whisper's detect_language() method. For now, we use
        heuristic confidence based on language support.
        """
        if not detected_lang:
            return 0.5

        # High confidence for supported languages
        if detected_lang in self.supported_languages:
            return 0.9

        # Medium confidence for other languages
        return 0.7

    def _estimate_linguistic_confidence(
        self,
        transcript: str,
        segments: List[Dict]
    ) -> float:
        """
        Estimate confidence from text quality indicators.

        Factors:
        - Text length appropriateness
        - Repetition detection (hallucination indicator)
        - Special character ratio (noise indicator)
        - Segment coverage (if available)
        - Word-to-time ratio consistency
        """
        if not transcript:
            return 0.0

        confidence_factors = []

        # Factor 1: Length appropriateness
        words = transcript.split()
        if words:
            # Ideal length: 5-15 words for typical utterance
            length_score = 1.0 - min(1.0, abs(len(words) - 10) / 20)
            confidence_factors.append(length_score)

        # Factor 2: Repetition detection
        repetition_penalty = self._detect_repetition(words)
        confidence_factors.append(1.0 - repetition_penalty)

        # Factor 3: Special character ratio
        special_char_ratio = sum(
            1 for c in transcript
            if not c.isalnum() and not c.isspace()
        ) / max(1, len(transcript))
        special_char_score = 1.0 - min(1.0, special_char_ratio * 10)
        confidence_factors.append(special_char_score)

        # Factor 4: Segment coverage (if segments available)
        if segments and len(segments) > 1:
            coverage_score = self._estimate_segment_coverage(segments)
            confidence_factors.append(coverage_score)

            # Factor 5: Word-to-time ratio consistency
            ratio_score = self._estimate_word_time_ratio(segments)
            confidence_factors.append(ratio_score)

        # Aggregate factors
        if confidence_factors:
            return float(np.mean(confidence_factors))

        return 0.6  # Default moderate confidence

    def _detect_repetition(self, words: List[str]) -> float:
        """
        Detect excessive word repetition (hallucination indicator).

        Returns:
            Penalty score between 0.0 (no repetition) and 1.0 (severe)
        """
        if len(words) < 4:
            return 0.0

        # Check for 4+ consecutive repeated words
        for i in range(len(words) - 3):
            if words[i] == words[i+1] == words[i+2] == words[i+3]:
                return 1.0  # Severe repetition

        # Check for high overall repetition rate
        word_counts = {}
        for word in words:
            word_counts[word] = word_counts.get(word, 0) + 1

        if word_counts:
            max_count = max(word_counts.values())
            repetition_rate = max_count / len(words)

            # Penalty if >30% of words are the same
            if repetition_rate > 0.3:
                return min(1.0, (repetition_rate - 0.3) / 0.7)

        return 0.0

    def _estimate_segment_coverage(self, segments: List[Dict]) -> float:
        """
        Estimate how well segments cover the audio without gaps.

        Returns:
            Coverage score between 0.0 (poor) and 1.0 (excellent)
        """
        if len(segments) < 2:
            return 0.8  # Neutral for single segment

        # Sort segments by start time
        sorted_segments = sorted(segments, key=lambda s: s.get("start", 0))

        # Calculate total gap time
        total_gap = 0.0
        for i in range(len(sorted_segments) - 1):
            current_end = sorted_segments[i].get("end", 0)
            next_start = sorted_segments[i + 1].get("start", 0)
            gap = max(0.0, next_start - current_end)
            total_gap += gap

        # Total duration
        total_duration = sorted_segments[-1].get("end", 0) - sorted_segments[0].get("start", 0)

        if total_duration <= 0:
            return 0.5

        # Coverage score (less gaps = better)
        gap_ratio = total_gap / total_duration
        coverage = 1.0 - min(1.0, gap_ratio * 2)  # Allow small gaps

        return coverage

    def _estimate_word_time_ratio(self, segments: List[Dict]) -> float:
        """
        Estimate consistency of word-to-time ratio across segments.

        Typical speech: 2-3 words per second.

        Returns:
            Ratio score between 0.0 (inconsistent) and 1.0 (consistent)
        """
        if not segments:
            return 0.8  # Neutral if no segments

        ratios = []
        for seg in segments:
            text = seg.get("text", "")
            start = seg.get("start", 0)
            end = seg.get("end", 0)

            duration = max(0.1, end - start)  # Avoid division by zero
            word_count = len(text.split())

            ratio = word_count / duration
            ratios.append(ratio)

        if not ratios:
            return 0.8

        avg_ratio = np.mean(ratios)

        # Typical speech: 2-3 words per second
        # Score based on how close to ideal
        ideal_ratio = 2.5
        deviation = abs(avg_ratio - ideal_ratio)
        score = 1.0 - min(1.0, deviation / 2.0)

        return float(score)


class WhisperConfidenceExtractor:
    """
    Extract confidence scores from MLX Whisper transcriptions.

    This class wraps the standard MLX Whisper transcription and
    adds confidence estimation using the ensemble approach.
    """

    def __init__(
        self,
        model_path: str = "mlx-community/whisper-large-v3-turbo",
        estimator: Optional[ConfidenceEstimator] = None
    ):
        """
        Initialize confidence extractor.

        Args:
            model_path: Path or HF repo for MLX Whisper model
            estimator: Optional custom confidence estimator
        """
        self.model_path = model_path
        self.estimator = estimator or ConfidenceEstimator()

        # Lazy load model
        self._model_loaded = False

    def _ensure_model_loaded(self):
        """Lazy load Whisper model."""
        if not self._model_loaded:
            import mlx_whisper

            # Warm up the model
            dummy_audio = np.zeros(16000, dtype=np.float32)
            mlx_whisper.transcribe(
                dummy_audio,
                path_or_hf_repo=self.model_path,
                language="en",
            )

            self._model_loaded = True

    def transcribe_with_confidence(
        self,
        audio: np.ndarray,
        language: Optional[str] = None
    ) -> Tuple[str, str, float]:
        """
        Transcribe audio and return text, language, and confidence.

        Args:
            audio: Audio samples (float32, 16kHz, normalized to [-1, 1])
            language: Optional language hint (None for auto-detect)

        Returns:
            (transcript, detected_language, confidence_score)
        """
        self._ensure_model_loaded()

        import mlx_whisper

        # Standard transcription
        result = mlx_whisper.transcribe(
            audio,
            path_or_hf_repo=self.model_path,
            language=language,
        )

        text = result.get("text", "").strip()
        detected_lang = result.get("language", language or "en")
        segments = result.get("segments", [])

        # Estimate confidence using ensemble
        confidence = self.estimator.estimate_confidence(
            audio, text, detected_lang, segments
        )

        return text, detected_lang, confidence


# Convenience function for quick usage
def transcribe_with_confidence(
    audio: np.ndarray,
    model_path: str = "mlx-community/whisper-large-v3-turbo",
    language: Optional[str] = None
) -> Tuple[str, str, float]:
    """
    Convenience function to transcribe audio with confidence.

    Args:
        audio: Audio samples (float32, 16kHz, normalized to [-1, 1])
        model_path: Path or HF repo for MLX Whisper model
        language: Optional language hint

    Returns:
        (transcript, detected_language, confidence_score)
    """
    extractor = WhisperConfidenceExtractor(model_path)
    return extractor.transcribe_with_confidence(audio, language)


if __name__ == "__main__":
    # Test with synthetic audio
    print("Testing Whisper confidence estimation...")

    # Create synthetic audio (white noise for testing)
    sample_rate = 16000
    duration = 5.0  # seconds
    audio = np.random.randn(int(sample_rate * duration)).astype(np.float32) * 0.01

    # Test confidence extraction
    text, lang, confidence = transcribe_with_confidence(audio)

    print(f"Transcript: {text}")
    print(f"Language: {lang}")
    print(f"Confidence: {confidence:.3f}")

    # Test ensemble estimator directly
    print("\nTesting ensemble estimator...")
    estimator = ConfidenceEstimator()

    # Test with various scenarios
    test_cases = [
        ("Hello world", "en", 0.9),
        ("", "en", 0.1),
        ("a " * 100, "en", 0.3),  # Repetition
        ("Test test test test", "en", 0.4),  # Repetition
        ("This is a normal sentence with reasonable length.", "en", 0.8),
    ]

    for test_text, test_lang, expected_range in test_cases:
        test_audio = np.random.randn(16000).astype(np.float32) * 0.01
        conf = estimator.estimate_confidence(
            test_audio, test_text, test_lang, []
        )
        print(f"Text: '{test_text[:50]}...' | Confidence: {conf:.3f} | Expected: ~{expected_range:.1f}")

    print("\n✅ Confidence estimation test complete!")
