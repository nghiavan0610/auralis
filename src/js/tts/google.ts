/**
 * Google Cloud TTS provider.
 *
 * Calls the Rust backend which connects to Google's Text-to-Speech REST API.
 * Returns MP3 audio that plays via HTMLAudioElement.
 */

import { invoke } from '@tauri-apps/api/core';
import type { TTSProviderAdapter, TTSVoice } from './types';

interface GoogleVoice {
  name: string;
  lang: string;
  gender: string;
  natural: boolean;
}

export class GoogleTTSProvider implements TTSProviderAdapter {
  private currentAudio: HTMLAudioElement | null = null;

  async speak(text: string, lang: string, voice: string, rate: number): Promise<void> {
    this.cleanupAudio();

    try {
      const base64Audio = await invoke<string>('google_tts_synthesize', {
        text,
        voice,
        rate,
        lang,
      });

      const audio = new Audio(`data:audio/mp3;base64,${base64Audio}`);
      this.currentAudio = audio;

      try {
        await audio.play();
      } catch (playErr: unknown) {
        if (playErr instanceof DOMException && playErr.name === 'AbortError') return;
        throw playErr;
      }
    } catch (err) {
      console.warn('[Auralis] Google TTS failed:', err);
      throw err;
    }
  }

  stop(): void {
    this.cleanupAudio();
  }

  async getVoices(lang?: string): Promise<TTSVoice[]> {
    try {
      const voices = await invoke<GoogleVoice[]>('google_tts_list_voices', {
        lang: lang ?? null,
      });
      return voices.map((v) => ({
        name: v.name,
        lang: v.lang,
        local: false,
        gender: v.gender,
      }));
    } catch (err) {
      console.warn('[Auralis] Failed to list Google voices:', err);
      return [];
    }
  }

  private cleanupAudio(): void {
    if (this.currentAudio) {
      this.currentAudio.pause();
      this.currentAudio.removeAttribute('src');
      this.currentAudio.load();
      this.currentAudio = null;
    }
  }
}
