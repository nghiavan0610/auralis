/**
 * Base class for audio-based TTS providers.
 *
 * Handles common audio playback logic for TTS providers that return
 * base64-encoded audio (MP3, etc.) that plays via HTMLAudioElement.
 *
 * Extending classes only need to implement:
 * - `synthesize()`: Call backend to get base64 audio
 * - `fetchVoices()`: Get available voices from backend
 */

import { invoke } from '@tauri-apps/api/core';
import type { TTSProviderAdapter, TTSVoice } from './types';

export abstract class BaseAudioTTSProvider implements TTSProviderAdapter {
  protected currentAudio: HTMLAudioElement | null = null;

  /**
   * Abstract method - subclasses must implement the actual TTS synthesis
   */
  protected abstract synthesize(text: string, lang: string, voice: string, rate: number): Promise<string>;

  /**
   * Abstract method - subclasses must implement voice fetching
   */
  protected abstract fetchVoices(lang?: string): Promise<TTSVoice[]>;

  /**
   * Common speak implementation for all audio-based providers
   */
  async speak(text: string, lang: string, voice: string, rate: number): Promise<void> {
    this.cleanupAudio();

    try {
      const base64Audio = await this.synthesize(text, lang, voice, rate);
      const audio = new Audio(`data:audio/mp3;base64,${base64Audio}`);
      this.currentAudio = audio;

      try {
        await audio.play();
      } catch (playErr: unknown) {
        if (playErr instanceof DOMException && playErr.name === 'AbortError') return;
        throw playErr;
      }
    } catch (err) {
      console.warn('[Auralis] TTS failed:', err);
      throw err;
    }
  }

  /**
   * Common stop implementation
   */
  stop(): void {
    this.cleanupAudio();
  }

  /**
   * Common getVoices implementation
   */
  async getVoices(lang?: string): Promise<TTSVoice[]> {
    try {
      return await this.fetchVoices(lang);
    } catch (err) {
      console.warn('[Auralis] Failed to list TTS voices:', err);
      return [];
    }
  }

  /**
   * Common audio cleanup - shared by all audio-based providers
   */
  protected cleanupAudio(): void {
    if (this.currentAudio) {
      this.currentAudio.pause();
      this.currentAudio.removeAttribute('src');
      this.currentAudio.load();
      this.currentAudio = null;
    }
  }
}
