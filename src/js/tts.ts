/**
 * Cross-platform TTS engine with multi-provider support.
 *
 * Phase 1: Web Speech API (offline, built-in)
 * Phase 2: Edge TTS (cloud, free, high quality)
 */

import { invoke } from '@tauri-apps/api/core';

export type TTSProvider = 'webspeech' | 'edge';

export interface TTSVoice {
  name: string;
  lang: string;
  local: boolean;
}

interface EdgeVoice {
  name: string;
  lang: string;
  gender: string;
}

class TTSEngine {
  private synth: SpeechSynthesis | null = null;
  private currentUtterance: SpeechSynthesisUtterance | null = null;
  private currentAudio: HTMLAudioElement | null = null;
  private _provider: TTSProvider = 'webspeech';
  private _rate: number = 1.0;
  private _voice: string = ''; // empty = auto
  private voices: SpeechSynthesisVoice[] = [];
  private edgeVoices: EdgeVoice[] = [];
  private voicesLoaded: Promise<void>;

  constructor() {
    this.synth = window.speechSynthesis ?? null;

    // Voices load asynchronously on some browsers
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

      // Safety timeout — some platforms never fire voiceschanged
      setTimeout(resolve, 2000);
    });
  }

  /** Speak text aloud. Interrupts any current speech. */
  async speak(text: string, lang: string): Promise<void> {
    if (!text.trim()) return;

    this.stop();

    if (this._provider === 'edge') {
      await this.speakEdge(text, lang);
    } else {
      this.speakWebSpeech(text, lang);
    }
  }

  /** Stop any currently playing speech. */
  stop(): void {
    // Stop Web Speech
    if (this.synth) {
      this.synth.cancel();
    }
    this.currentUtterance = null;

    // Stop Edge TTS audio
    if (this.currentAudio) {
      this.currentAudio.pause();
      this.currentAudio.src = '';
      this.currentAudio = null;
    }
  }

  /** Get available voices for the current provider, optionally filtered by language. */
  async getVoices(lang?: string): Promise<TTSVoice[]> {
    if (this._provider === 'edge') {
      return this.getEdgeVoices(lang);
    }
    return this.getWebSpeechVoices(lang);
  }

  /** Set TTS provider. */
  setProvider(provider: TTSProvider): void {
    this._provider = provider;
  }

  /** Get current provider. */
  get provider(): TTSProvider {
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

  // --- Web Speech API ---

  private speakWebSpeech(text: string, lang: string): void {
    if (!this.synth) return;

    const utterance = new SpeechSynthesisUtterance(text);
    utterance.rate = this._rate;

    const voice = this.pickWebSpeechVoice(lang);
    if (voice) {
      utterance.voice = voice;
    } else {
      utterance.lang = lang;
    }

    this.currentUtterance = utterance;
    this.synth.speak(utterance);
  }

  private async getWebSpeechVoices(lang?: string): Promise<TTSVoice[]> {
    await this.voicesLoaded;

    let filtered = this.voices;
    if (lang) {
      const prefix = lang.toLowerCase();
      filtered = this.voices.filter((v) =>
        v.lang.toLowerCase().startsWith(prefix)
      );
    }

    return filtered.map((v) => ({
      name: v.name,
      lang: v.lang,
      local: v.localService,
    }));
  }

  private pickWebSpeechVoice(lang: string): SpeechSynthesisVoice | null {
    const prefix = lang.toLowerCase();

    if (this._voice) {
      const match = this.voices.find(
        (v) => v.name === this._voice && v.lang.toLowerCase().startsWith(prefix)
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

  // --- Edge TTS ---

  private async speakEdge(text: string, lang: string): Promise<void> {
    try {
      const base64Audio = await invoke<string>('edge_tts_synthesize', {
        text,
        voice: this._voice,
        rate: this._rate,
        lang,
      });

      const audio = new Audio(`data:audio/mp3;base64,${base64Audio}`);
      this.currentAudio = audio;
      await audio.play();
    } catch (err) {
      console.warn('[Auralis] Edge TTS failed, falling back to Web Speech:', err);
      this.speakWebSpeech(text, lang);
    }
  }

  private async getEdgeVoices(lang?: string): Promise<TTSVoice[]> {
    try {
      const voices = await invoke<EdgeVoice[]>('edge_tts_list_voices', {
        lang: lang ?? null,
      });
      return voices.map((v) => ({
        name: v.name,
        lang: v.lang,
        local: true, // Edge voices are served locally after synthesis
      }));
    } catch (err) {
      console.warn('[Auralis] Failed to list Edge voices:', err);
      return [];
    }
  }
}

// Singleton instance
export const tts = new TTSEngine();
