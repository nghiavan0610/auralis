/**
 * ElevenLabs TTS provider.
 *
 * Calls the Rust backend which connects to ElevenLabs REST API.
 * Returns MP3 audio that plays via HTMLAudioElement.
 */

import { invoke } from '@tauri-apps/api/core';
import type { TTSVoice } from './types';
import { BaseAudioTTSProvider } from './BaseAudioTTSProvider';

interface ElevenLabsVoice {
  voice_id: string;
  name: string;
  lang: string;
  gender: string;
}

export class ElevenLabsTTSProvider extends BaseAudioTTSProvider {
  protected async synthesize(text: string, lang: string, voice: string, rate: number): Promise<string> {
    return await invoke<string>('elevenlabs_tts_synthesize', {
      text,
      voice,
      rate,
      lang,
    });
  }

  protected async fetchVoices(_lang?: string): Promise<TTSVoice[]> {
    const voices = await invoke<ElevenLabsVoice[]>('elevenlabs_tts_list_voices');
    return voices.map((v) => ({
      name: v.voice_id,
      lang: v.lang,
      local: false,
      gender: v.gender,
    }));
  }
}
