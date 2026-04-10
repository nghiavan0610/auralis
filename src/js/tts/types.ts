/**
 * TTS provider types and shared interfaces.
 */

export type TTSProviderName = 'webspeech' | 'edge' | 'google';

export interface TTSVoice {
  name: string;
  lang: string;
  local: boolean;
  gender?: string;
}

/**
 * Interface that each TTS provider adapter must implement.
 */
export interface TTSProviderAdapter {
  /** Speak text aloud. Interrupts any current speech from this provider. */
  speak(text: string, lang: string, voice: string, rate: number): Promise<void>;

  /** Stop any currently playing speech. */
  stop(): void;

  /** Get available voices, optionally filtered by language. */
  getVoices(lang?: string): Promise<TTSVoice[]>;
}
