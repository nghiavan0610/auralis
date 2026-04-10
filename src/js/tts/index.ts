/**
 * Cross-platform TTS engine with multi-provider support.
 *
 * Uses the Strategy pattern: each provider implements TTSProviderAdapter.
 * The engine dispatches to the active provider and handles fallback
 * to Web Speech if a cloud provider fails.
 */

import type { TTSProviderName, TTSVoice, TTSProviderAdapter } from './types';
import { WebSpeechProvider } from './webspeech';
import { EdgeTTSProvider } from './edge';
import { GoogleTTSProvider } from './google';

class TTSEngine {
  private _provider: TTSProviderName = 'webspeech';
  private _rate: number = 1.0;
  private _voice: string = '';

  // Cached singleton instances — each provider holds its own audio state
  // (currentAudio, currentUtterance) so we must reuse the same instance.
  private providers: Record<TTSProviderName, TTSProviderAdapter>;

  constructor() {
    this.providers = {
      webspeech: new WebSpeechProvider(),
      edge: new EdgeTTSProvider(),
      google: new GoogleTTSProvider(),
    };
  }

  private get active(): TTSProviderAdapter {
    return this.providers[this._provider];
  }

  /** Speak text aloud. Interrupts any current speech. Falls back to Web Speech on error. */
  async speak(text: string, lang: string): Promise<void> {
    if (!text.trim()) return;

    this.stop();

    try {
      await this.active.speak(text, lang, this._voice, this._rate);
    } catch {
      if (this._provider !== 'webspeech') {
        console.warn('[Auralis] TTS provider failed, falling back to Web Speech');
        await this.providers['webspeech'].speak(text, lang, this._voice, this._rate);
      }
    }
  }

  /** Stop any currently playing speech across all providers. */
  stop(): void {
    for (const provider of Object.values(this.providers)) {
      try {
        provider.stop();
      } catch {
        // Best-effort
      }
    }
  }

  /** Get available voices for the current provider, optionally filtered by language. */
  async getVoices(lang?: string): Promise<TTSVoice[]> {
    return this.active.getVoices(lang);
  }

  /** Set TTS provider. */
  setProvider(provider: TTSProviderName): void {
    this._provider = provider;
  }

  /** Get current provider. */
  get provider(): TTSProviderName {
    return this._provider;
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
}

// Singleton instance
export const tts = new TTSEngine();

// Re-export types for convenience
export type { TTSProviderName, TTSVoice } from './types';
