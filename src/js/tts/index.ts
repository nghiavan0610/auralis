/**
 * Cross-platform TTS engine with multi-provider support.
 *
 * Uses the Strategy pattern: each provider implements TTSProviderAdapter.
 * The engine dispatches to the active provider and handles fallback
 * to Web Speech if a cloud provider fails.
 *
 * Providers are lazily loaded - only instantiated when first used.
 * Inactive cloud providers are unloaded after 30 seconds to free resources.
 */

import type { TTSProviderName, TTSVoice, TTSProviderAdapter } from './types';
import { WebSpeechProvider } from './webspeech';
import { EdgeTTSProvider } from './edge';
import { GoogleTTSProvider } from './google';
import { ElevenLabsTTSProvider } from './elevenlabs';

const PROVIDER_UNLOAD_DELAY_MS = 30000; // 30 seconds

class TTSEngine {
  private _provider: TTSProviderName = 'webspeech';
  private _rate: number = 1.0;
  private _voice: string = '';

  // Lazy-loaded provider instances
  private providers: Partial<Record<TTSProviderName, TTSProviderAdapter>> = {};

  // Unload timers for each provider
  private unloadTimers: Partial<Record<TTSProviderName, ReturnType<typeof setTimeout>>> = {};

  constructor() {
    // Always keep WebSpeech loaded (it's lightweight)
    this.providers['webspeech'] = new WebSpeechProvider();
  }

  private getProvider(name: TTSProviderName): TTSProviderAdapter {
    // Create provider on demand if not exists
    if (!this.providers[name]) {
      switch (name) {
        case 'webspeech':
          this.providers[name] = new WebSpeechProvider();
          break;
        case 'edge':
          this.providers[name] = new EdgeTTSProvider();
          break;
        case 'google':
          this.providers[name] = new GoogleTTSProvider();
          break;
        case 'elevenlabs':
          this.providers[name] = new ElevenLabsTTSProvider();
          break;
      }
    }

    // Clear any existing unload timer
    if (this.unloadTimers[name]) {
      clearTimeout(this.unloadTimers[name]);
      delete this.unloadTimers[name];
    }

    return this.providers[name]!;
  }

  private scheduleUnload(providerName: TTSProviderName): void {
    // Don't unload WebSpeech - it's lightweight and always needed
    if (providerName === 'webspeech') {
      return;
    }

    // Clear existing timer if any
    if (this.unloadTimers[providerName]) {
      clearTimeout(this.unloadTimers[providerName]);
    }

    // Schedule unload after delay
    this.unloadTimers[providerName] = setTimeout(() => {
      // Only unload if it's not the current provider
      if (this._provider !== providerName && this.providers[providerName]) {
        // Cleanup the provider
        const provider = this.providers[providerName];
        if (provider && 'cleanup' in provider) {
          try {
            (provider as any).cleanup();
          } catch (e) {
            console.warn(`[TTS] Error cleaning up ${providerName}:`, e);
          }
        }

        delete this.providers[providerName];
      }

      delete this.unloadTimers[providerName];
    }, PROVIDER_UNLOAD_DELAY_MS);
  }

  private get active(): TTSProviderAdapter {
    return this.getProvider(this._provider);
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
        await this.providers['webspeech']!.speak(text, lang, this._voice, this._rate);
      }
    }
  }

  /** Stop any currently playing speech across all providers. */
  stop(): void {
    for (const [name, provider] of Object.entries(this.providers)) {
      try {
        provider?.stop();
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
    const oldProvider = this._provider;
    this._provider = provider;

    // Schedule unload of old provider if it's a cloud provider
    if (oldProvider !== 'webspeech' && oldProvider !== provider) {
      this.scheduleUnload(oldProvider);
    }
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

  /** Cleanup all providers (call on app shutdown) */
  cleanup(): void {
    // Clear all unload timers
    for (const timer of Object.values(this.unloadTimers)) {
      clearTimeout(timer);
    }
    this.unloadTimers = {};

    // Cleanup all providers
    for (const [name, provider] of Object.entries(this.providers)) {
      if (provider && 'cleanup' in provider) {
        try {
          (provider as any).cleanup();
        } catch (e) {
          console.warn(`[TTS] Error cleaning up ${name}:`, e);
        }
      }
    }
    this.providers = { webspeech: this.providers['webspeech']! };
  }
}

// Singleton instance
export const tts = new TTSEngine();

// Re-export types for convenience
export type { TTSProviderName, TTSVoice } from './types';
