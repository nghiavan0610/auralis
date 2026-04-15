/** Shared types for Auralis */

export type OperatingMode = 'cloud' | 'offline';
export type TranslationType = 'one_way' | 'two_way';
export type AudioSource = 'microphone' | 'system' | 'both';

/** Language auto-detection state for one-way and two-way translation */
export interface DetectionState {
  status: 'idle' | 'detecting' | 'detected' | 'uncertain' | 'error';
  detectedLanguage?: string;
  /** Active speaker in two-way mode (1 = source language, 2 = target language) */
  activeSpeaker?: 1 | 2;
  /** Detection confidence level */
  confidence?: 'high' | 'medium' | 'low';
}

/** A paired original + translation segment */
export interface Segment {
  id: number;
  original: string;
  translated: string;
  /** Language detected from the original speech */
  detectedLang: string;
  /** Language the translation was rendered into */
  targetLang: string;
  status: 'pending' | 'provisional' | 'translated';
  timestamp: number;
  /** Detection confidence level (for two-way mode) */
  confidence?: 'high' | 'medium' | 'low';
}
