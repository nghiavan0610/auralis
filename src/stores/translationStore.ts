/**
 * Translation Settings Store
 *
 * Centralized state for all translation-related settings.
 * Eliminates prop drilling and state duplication.
 */

import { invoke } from '@tauri-apps/api/core';
import type { AudioSource, OperatingMode, TranslationType } from '../js/constants';
import type { ConfidenceFilterLevel } from '../js/confidenceFilter';

export interface TranslationSettings {
  mode: OperatingMode;
  sourceLanguage: string;
  targetLanguage: string;
  translationType: TranslationType;
  audioSource: AudioSource;
  endpointDelay: number;
  confidenceFilterLevel: ConfidenceFilterLevel;
}

const DEFAULT_SETTINGS: TranslationSettings = {
  mode: 'cloud' as OperatingMode,
  sourceLanguage: 'en',
  targetLanguage: 'vi',
  translationType: 'one_way' as TranslationType,
  audioSource: 'microphone' as AudioSource,
  endpointDelay: 1.0,
  confidenceFilterLevel: 'low',
};

let currentSettings: TranslationSettings = { ...DEFAULT_SETTINGS };
let listeners: Set<(settings: TranslationSettings) => void> = new Set();

/**
 * Translation store - reactive settings for translation
 */
export const translationStore = {
  /**
   * Get current translation settings (reactive)
   */
  get(): TranslationSettings {
    return { ...currentSettings };
  },

  /**
   * Update a single translation setting
   */
  update<K extends keyof TranslationSettings>(
    key: K,
    value: TranslationSettings[K]
  ): void {
    currentSettings[key] = value;
    notifyListeners();
    persistSettings();
  },

  /**
   * Update multiple settings at once
   */
  updateMany(settings: Partial<TranslationSettings>): void {
    currentSettings = { ...currentSettings, ...settings };
    notifyListeners();
    persistSettings();
  },

  /**
   * Reset to defaults
   */
  reset(): void {
    currentSettings = { ...DEFAULT_SETTINGS };
    notifyListeners();
    persistSettings();
  },

  /**
   * Subscribe to settings changes
   */
  subscribe(listener: (settings: TranslationSettings) => void): () => void {
    listeners.add(listener);
    return () => listeners.delete(listener);
  },
};

/**
 * Notify all listeners of settings changes
 */
function notifyListeners(): void {
  listeners.forEach(listener => listener({ ...currentSettings }));
}

/**
 * Persist settings to Tauri backend
 */
async function persistSettings(): Promise<void> {
  try {
    await invoke('save_settings', {
      settings: {
        mode: currentSettings.mode,
        source_language: currentSettings.sourceLanguage,
        target_language: currentSettings.targetLanguage,
        translation_type: currentSettings.translationType,
        audio_source: currentSettings.audioSource,
        endpoint_delay: currentSettings.endpointDelay,
        confidence_filter_level: currentSettings.confidenceFilterLevel,
      },
    });
  } catch (err) {
    console.error('[TranslationStore] Failed to save settings:', err);
  }
}

/**
 * Load settings from Tauri backend
 */
export async function loadTranslationSettings(): Promise<void> {
  try {
    const settings = await invoke<{
      mode?: string;
      source_language?: string;
      target_language?: string;
      translation_type?: string;
      audio_source?: string;
      endpoint_delay?: number;
      confidence_filter_level?: string;
    }>('load_settings');

    currentSettings = {
      mode: (settings.mode ?? DEFAULT_SETTINGS.mode) as OperatingMode,
      sourceLanguage: settings.source_language ?? DEFAULT_SETTINGS.sourceLanguage,
      targetLanguage: settings.target_language ?? DEFAULT_SETTINGS.targetLanguage,
      translationType: (settings.translation_type ?? DEFAULT_SETTINGS.translationType) as TranslationType,
      audioSource: (settings.audio_source ?? DEFAULT_SETTINGS.audioSource) as AudioSource,
      endpointDelay: settings.endpoint_delay ?? DEFAULT_SETTINGS.endpointDelay,
      confidenceFilterLevel: (settings.confidence_filter_level ?? DEFAULT_SETTINGS.confidenceFilterLevel) as ConfidenceFilterLevel,
    };

    notifyListeners();
  } catch (err) {
    console.error('[TranslationStore] Failed to load settings:', err);
  }
}
