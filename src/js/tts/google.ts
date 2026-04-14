/**
 * Google Cloud TTS provider.
 *
 * Calls the Rust backend which connects to Google's Text-to-Speech REST API.
 * Returns MP3 audio that plays via HTMLAudioElement.
 */

import { invoke } from '@tauri-apps/api/core';
import type { TTSVoice } from './types';
import { BaseAudioTTSProvider } from './BaseAudioTTSProvider';

interface GoogleVoice {
  name: string;
  lang: string;
  gender: string;
  natural: boolean;
}

export class GoogleTTSProvider extends BaseAudioTTSProvider {
  protected async synthesize(text: string, lang: string, voice: string, rate: number): Promise<string> {
    return await invoke<string>('google_tts_synthesize', {
      text,
      voice,
      rate,
      lang,
    });
  }

  protected async fetchVoices(lang?: string): Promise<TTSVoice[]> {
    const voices = await invoke<GoogleVoice[]>('google_tts_list_voices', {
      lang: lang ?? null,
    });
    return voices.map((v) => ({
      name: v.name,
      lang: v.lang,
      local: false,
      gender: v.gender,
    }));
  }
}
