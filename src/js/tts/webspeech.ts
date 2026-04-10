/**
 * Web Speech API TTS provider.
 *
 * Uses the browser's built-in SpeechSynthesis API.
 * Runs entirely in the frontend — no backend calls needed.
 */

import type { TTSProviderAdapter, TTSVoice } from './types';

export class WebSpeechProvider implements TTSProviderAdapter {
  private synth: SpeechSynthesis | null = null;
  private currentUtterance: SpeechSynthesisUtterance | null = null;
  private voices: SpeechSynthesisVoice[] = [];
  private voicesLoaded: Promise<void>;

  constructor() {
    this.synth = window.speechSynthesis ?? null;

    this.voicesLoaded = new Promise((resolve) => {
      if (!this.synth) {
        resolve();
        return;
      }

      const loaded = this.synth.getVoices();
      if (loaded.length > 0) {
        this.voices = loaded;
        resolve();
        return;
      }

      const handler = () => {
        this.voices = this.synth!.getVoices();
        resolve();
      };
      this.synth.addEventListener('voiceschanged', handler, { once: true });
      setTimeout(resolve, 2000);
    });
  }

  async speak(text: string, lang: string, voice: string, rate: number): Promise<void> {
    if (!this.synth) return;
    this.stop();

    const utterance = new SpeechSynthesisUtterance(text);
    utterance.rate = rate;

    const pickedVoice = this.pickVoice(lang, voice);
    if (pickedVoice) {
      utterance.voice = pickedVoice;
    } else {
      utterance.lang = lang;
    }

    this.currentUtterance = utterance;
    this.synth.speak(utterance);
  }

  stop(): void {
    if (this.synth) {
      this.synth.cancel();
    }
    this.currentUtterance = null;
  }

  async getVoices(_lang?: string): Promise<TTSVoice[]> {
    if (this.synth && this.voices.length === 0) {
      this.voices = this.synth.getVoices();
    }
    return this.voices.map((v) => ({
      name: v.name,
      lang: v.lang,
      local: v.localService,
    }));
  }

  private pickVoice(lang: string, preferredVoice: string): SpeechSynthesisVoice | null {
    const prefix = lang.toLowerCase();

    if (preferredVoice) {
      const match = this.voices.find(
        (v) => v.name === preferredVoice && v.lang.toLowerCase().startsWith(prefix)
      );
      if (match) return match;
    }

    const local = this.voices.find(
      (v) => v.localService && v.lang.toLowerCase().startsWith(prefix)
    );
    if (local) return local;

    const any = this.voices.find((v) =>
      v.lang.toLowerCase().startsWith(prefix)
    );
    return any ?? null;
  }
}
