/**
 * Edge TTS provider.
 *
 * Calls the Rust backend which connects to Microsoft's Edge TTS WebSocket.
 * Returns MP3 audio that plays via HTMLAudioElement.
 */

import { invoke } from '@tauri-apps/api/core';
import type { TTSProviderAdapter, TTSVoice } from './types';

interface EdgeVoice {
  name: string;
  lang: string;
  gender: string;
}

export class EdgeTTSProvider implements TTSProviderAdapter {
  private currentAudio: HTMLAudioElement | null = null;

  async speak(text: string, lang: string, voice: string, rate: number): Promise<void> {
    this.cleanupAudio();

    try {
      const base64Audio = await invoke<string>('edge_tts_synthesize', {
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
      console.warn('[Auralis] Edge TTS failed:', err);
      throw err;
    }
  }

  stop(): void {
    this.cleanupAudio();
  }

  async getVoices(lang?: string): Promise<TTSVoice[]> {
    try {
      const voices = await invoke<EdgeVoice[]>('edge_tts_list_voices', {
        lang: lang ?? null,
      });
      return voices.map((v) => ({
        name: v.name,
        lang: v.lang,
        local: true,
        gender: v.gender,
      }));
    } catch (err) {
      console.warn('[Auralis] Failed to list Edge voices:', err);
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
