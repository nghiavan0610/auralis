/**
 * Edge TTS provider.
 *
 * Calls the Rust backend which connects to Microsoft's Edge TTS WebSocket.
 * Returns MP3 audio that plays via HTMLAudioElement.
 */

import { invoke } from '@tauri-apps/api/core';
import type { TTSVoice } from './types';
import { BaseAudioTTSProvider } from './BaseAudioTTSProvider';

interface EdgeVoice {
  name: string;
  lang: string;
  gender: string;
}

export class EdgeTTSProvider extends BaseAudioTTSProvider {
  protected async synthesize(text: string, lang: string, voice: string, rate: number): Promise<string> {
    return await invoke<string>('edge_tts_synthesize', {
      text,
      voice,
      rate,
      lang,
    });
  }

  protected async fetchVoices(lang?: string): Promise<TTSVoice[]> {
    const voices = await invoke<EdgeVoice[]>('edge_tts_list_voices', {
      lang: lang ?? null,
    });
    return voices.map((v) => ({
      name: v.name,
      lang: v.lang,
      local: true,
      gender: v.gender,
    }));
  }
}
