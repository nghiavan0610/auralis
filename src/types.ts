/** Shared types for Auralis */

export type OperatingMode = 'cloud' | 'offline';
export type TranslationType = 'one_way' | 'two_way';
export type AudioSource = 'microphone' | 'system' | 'both';

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
}
