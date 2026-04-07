"""
MADLAD Translator using ctranslate2

This module provides Python-based translation using the MADLAD model
through ctranslate2 for efficient neural machine translation.
"""

import os
import warnings
from typing import Optional, Tuple, List
from pathlib import Path

# Optional imports - only required when actually using the translator
try:
    import ctranslate2
    import sentencepiece as spm
    TRANSLATOR_AVAILABLE = True
except ImportError:
    TRANSLATOR_AVAILABLE = False
    warnings.warn("ctranslate2 or sentencepiece not available. Translation functionality will be limited.")


class MadladTranslator:
    """
    MADLAD neural machine translation engine.

    This class provides translation capabilities using the MADLAD model
    with ctranslate2 for efficient inference.
    """

    def __init__(
        self,
        model_path: str,
        device: str = "cpu",
        compute_type: str = "default",
        **kwargs
    ):
        """
        Initialize the MADLAD translator.

        Args:
            model_path: Path to the MADLAD model directory
            device: Device to use for inference ("cpu", "cuda", "auto")
            compute_type: Compute type ("default", "int8", "int8_float16", "int16", "float16")
            **kwargs: Additional arguments passed to ctranslate2.Translator
        """
        if not TRANSLATOR_AVAILABLE:
            raise RuntimeError(
                "ctranslate2 and sentencepiece are required for translation. "
                "Install them with: pip install ctranslate2 sentencepiece"
            )

        self.model_path = Path(model_path)
        self.device = device
        self.compute_type = compute_type
        self.translator = None
        self.sp_model = None
        self._initialized = False

    def initialize(self) -> None:
        """
        Initialize the translator and load the model.

        Raises:
            RuntimeError: If the model path doesn't exist or initialization fails
        """
        if not self.model_path.exists():
            raise RuntimeError(f"Model path not found: {self.model_path}")

        try:
            # Initialize ctranslate2 translator
            self.translator = ctranslate2.Translator(
                str(self.model_path),
                device=self.device,
                compute_type=self.compute_type,
            )

            # Load sentencepiece model for tokenization
            sp_model_path = self.model_path / "sentencepiece.model"
            if sp_model_path.exists():
                self.sp_model = spm.SentencePieceProcessor(str(sp_model_path))
            else:
                warnings.warn(f"SentencePiece model not found at {sp_model_path}")

            self._initialized = True

        except Exception as e:
            raise RuntimeError(f"Failed to initialize MADLAD translator: {e}")

    def is_available(self) -> bool:
        """Check if the translator is available and initialized."""
        return self._initialized and self.translator is not None

    def _tokenize(self, text: str) -> List[str]:
        """
        Tokenize text using sentencepiece if available.

        Args:
            text: Input text to tokenize

        Returns:
            List of tokens
        """
        if self.sp_model:
            return self.sp_model.encode(text, out_type=str)
        return text.split()

    def _detokenize(self, tokens: List[str]) -> str:
        """
        Detokenize tokens using sentencepiece if available.

        Args:
            tokens: List of tokens to detokenize

        Returns:
            Detokenized text
        """
        if self.sp_model:
            return self.sp_model.decode(tokens)
        return " ".join(tokens)

    def translate(
        self,
        text: str,
        source_lang: str,
        target_lang: str,
        beam_size: int = 1,
        max_input_length: int = 512,
        **kwargs
    ) -> Tuple[str, float]:
        """
        Translate text from source language to target language.

        Args:
            text: Text to translate
            source_lang: Source language code (e.g., "en", "es", "zh")
            target_lang: Target language code
            beam_size: Beam size for beam search
            max_input_length: Maximum input length in tokens
            **kwargs: Additional translation parameters

        Returns:
            Tuple of (translated_text, confidence_score)

        Raises:
            RuntimeError: If translator is not initialized
            ValueError: If input text is empty
        """
        if not self.is_available():
            raise RuntimeError("Translator is not initialized. Call initialize() first.")

        if not text or not text.strip():
            raise ValueError("Input text cannot be empty")

        # Prepare source text with language token
        # MADLAD uses language tokens like "en:", "es:", etc.
        source_prefix = f"{source_lang}:"
        target_prefix = f"{target_lang}:"

        # Tokenize input
        tokens = self._tokenize(text)
        source_tokens = [source_prefix] + tokens

        # Perform translation
        try:
            results = self.translator.translate_batch(
                [source_tokens],
                target_prefix=[[target_prefix]],
                beam_size=beam_size,
                max_input_length=max_input_length,
                **kwargs
            )

            if not results or not results[0].hypotheses:
                return "", 0.0

            # Get the best hypothesis
            translated_tokens = results[0].hypotheses[0]
            translated_text = self._detokenize(translated_tokens)

            # Calculate confidence score (simplified)
            # In a real implementation, you might use model probabilities
            confidence = results[0].scores[0] if results[0].scores else 0.5
            # Normalize confidence to 0-1 range
            confidence = max(0.0, min(1.0, confidence))

            return translated_text, confidence

        except Exception as e:
            raise RuntimeError(f"Translation failed: {e}")

    def translate_batch(
        self,
        texts: List[str],
        source_lang: str,
        target_lang: str,
        **kwargs
    ) -> List[Tuple[str, float]]:
        """
        Translate multiple texts in batch.

        Args:
            texts: List of texts to translate
            source_lang: Source language code
            target_lang: Target language code
            **kwargs: Additional translation parameters

        Returns:
            List of (translated_text, confidence_score) tuples
        """
        if not texts:
            return []

        if not self.is_available():
            raise RuntimeError("Translator is not initialized. Call initialize() first.")

        # Prepare source texts with language tokens
        source_prefix = f"{source_lang}:"
        target_prefix = f"{target_lang}:"

        # Tokenize all texts
        batch_source_tokens = []
        for text in texts:
            if not text or not text.strip():
                batch_source_tokens.append([source_prefix])
            else:
                tokens = self._tokenize(text)
                batch_source_tokens.append([source_prefix] + tokens)

        # Perform batch translation
        try:
            results = self.translator.translate_batch(
                batch_source_tokens,
                target_prefix=[[target_prefix]] * len(texts),
                **kwargs
            )

            translations = []
            for i, result in enumerate(results):
                if result.hypotheses:
                    translated_tokens = result.hypotheses[0]
                    translated_text = self._detokenize(translated_tokens)
                    confidence = result.scores[0] if result.scores else 0.5
                    confidence = max(0.0, min(1.0, confidence))
                    translations.append((translated_text, confidence))
                else:
                    translations.append(("", 0.0))

            return translations

        except Exception as e:
            raise RuntimeError(f"Batch translation failed: {e}")

    def supported_languages(self) -> List[str]:
        """
        Get list of supported language codes.

        Returns:
            List of supported language codes
        """
        # MADLAD supports a wide range of languages
        return [
            "en", "es", "fr", "de", "it", "pt", "ru", "zh",
            "ja", "ko", "ar", "hi", "tr", "vi", "th", "nl",
            "pl", "sv", "cs", "el", "he", "id", "ms", "ro",
            "uk", "da", "fi", "no", "bg", "hr", "sk", "sl"
        ]

    def is_pair_supported(self, source_lang: str, target_lang: str) -> bool:
        """
        Check if a language pair is supported.

        Args:
            source_lang: Source language code
            target_lang: Target language code

        Returns:
            True if the pair is supported
        """
        supported = self.supported_languages()
        return source_lang in supported and target_lang in supported


class MockTranslator:
    """
    Mock translator for testing purposes.

    This class provides a simple mock implementation that returns
    predictable translations without requiring actual model files.
    """

    def __init__(self, **kwargs):
        """Initialize the mock translator."""
        self._initialized = True

    def initialize(self) -> None:
        """Initialize the mock translator (no-op)."""
        self._initialized = True

    def is_available(self) -> bool:
        """Check if the translator is available."""
        return True

    def translate(
        self,
        text: str,
        source_lang: str,
        target_lang: str,
        **kwargs
    ) -> Tuple[str, float]:
        """
        Mock translation that returns a predictable result.

        Args:
            text: Text to translate
            source_lang: Source language code
            target_lang: Target language code
            **kwargs: Additional parameters (ignored)

        Returns:
            Tuple of (mock_translated_text, confidence_score)
        """
        if not text or not text.strip():
            return "", 0.0

        # Simple mock translation
        mock_translation = f"[{target_lang}] {text}"
        confidence = 0.85

        return mock_translation, confidence

    def translate_batch(
        self,
        texts: List[str],
        source_lang: str,
        target_lang: str,
        **kwargs
    ) -> List[Tuple[str, float]]:
        """
        Mock batch translation.

        Args:
            texts: List of texts to translate
            source_lang: Source language code
            target_lang: Target language code
            **kwargs: Additional parameters (ignored)

        Returns:
            List of (mock_translated_text, confidence_score) tuples
        """
        return [
            self.translate(text, source_lang, target_lang, **kwargs)
            for text in texts
        ]

    def supported_languages(self) -> List[str]:
        """Get list of supported language codes."""
        return [
            "en", "es", "fr", "de", "it", "pt", "ru", "zh",
            "ja", "ko", "ar", "hi", "tr", "vi", "th", "nl"
        ]

    def is_pair_supported(self, source_lang: str, target_lang: str) -> bool:
        """Check if a language pair is supported."""
        supported = self.supported_languages()
        return source_lang in supported and target_lang in supported


def create_translator(model_path: str, mock_mode: bool = False, **kwargs) -> "MadladTranslator | MockTranslator":
    """
    Factory function to create a translator.

    Args:
        model_path: Path to the MADLAD model directory
        mock_mode: If True, create a mock translator
        **kwargs: Additional arguments for the translator

    Returns:
        MadladTranslator or MockTranslator instance
    """
    if mock_mode:
        return MockTranslator(**kwargs)

    if not TRANSLATOR_AVAILABLE:
        warnings.warn("Translation libraries not available, using mock mode")
        return MockTranslator(**kwargs)

    return MadladTranslator(model_path, **kwargs)


# Export classes for PyO3 integration
__all__ = [
    "MadladTranslator",
    "MockTranslator",
    "create_translator",
    "TRANSLATOR_AVAILABLE",
]
