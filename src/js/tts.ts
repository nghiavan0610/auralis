/**
 * Cross-platform TTS engine using Web Speech API.
 *
 * Provides speech synthesis using the browser's built-in voices.
 * Works on macOS, Windows, and Linux via Tauri's WebView.
 */

export interface TTSVoice {
  name: string;
  lang: string;
  local: boolean;
}

class TTSEngine {
  private synth: SpeechSynthesis | null = null;
  private currentUtterance: SpeechSynthesisUtterance | null = null;
  private _rate: number = 1.0;
  private _voice: string = ''; // empty = auto
  private voices: SpeechSynthesisVoice[] = [];
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
    if (!this.synth || !text.trim()) return;

    await this.voicesLoaded;
    this.stop();

    const utterance = new SpeechSynthesisUtterance(text);

    // Set rate
    utterance.rate = this._rate;

    // Pick voice: user preference > best match for language > default
    const voice = this.pickVoice(lang);
    if (voice) {
      utterance.voice = voice;
    } else {
      // Fallback: set just the lang attribute
      utterance.lang = lang;
    }

    this.currentUtterance = utterance;
    this.synth.speak(utterance);
  }

  /** Stop any currently playing speech. */
  stop(): void {
    if (this.synth) {
      this.synth.cancel();
    }
    this.currentUtterance = null;
  }

  /** Get available voices, optionally filtered by language code prefix. */
  async getVoices(lang?: string): Promise<TTSVoice[]> {
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

  /** Pick the best voice for a given language. */
  private pickVoice(lang: string): SpeechSynthesisVoice | null {
    const prefix = lang.toLowerCase();

    // 1. User-selected voice that matches the language
    if (this._voice) {
      const match = this.voices.find(
        (v) => v.name === this._voice && v.lang.toLowerCase().startsWith(prefix)
      );
      if (match) return match;
    }

    // 2. Local voice for the language
    const local = this.voices.find(
      (v) => v.localService && v.lang.toLowerCase().startsWith(prefix)
    );
    if (local) return local;

    // 3. Any voice for the language
    const any = this.voices.find((v) =>
      v.lang.toLowerCase().startsWith(prefix)
    );
    return any ?? null;
  }
}

// Singleton instance
export const tts = new TTSEngine();
