/**
 * TTS Settings Store
 *
 * Centralized state for all TTS-related settings.
 */

import { invoke } from '@tauri-apps/api/core';
import type { TtsProvider } from '../js/constants';

export interface TTSSettings {
  enabled: boolean;
  provider: TtsProvider;
  rate: number;
}

const DEFAULT_SETTINGS: TTSSettings = {
  enabled: false,
  provider: 'webspeech' as TtsProvider,
  rate: 1.0,
};

let currentSettings: TTSSettings = { ...DEFAULT_SETTINGS };
let listeners: Set<(settings: TTSSettings) => void> = new Set();

/**
 * TTS store - reactive settings for TTS
 */
export const ttsStore = {
  /**
   * Get current TTS settings (reactive)
   */
  get(): TTSSettings {
    return { ...currentSettings };
  },

  /**
   * Update a single TTS setting
   */
  update<K extends keyof TTSSettings>(
    key: K,
    value: TTSSettings[K]
  ): void {
    currentSettings[key] = value;
    notifyListeners();
    persistSettings();
  },

  /**
   * Update multiple settings at once
   */
  updateMany(settings: Partial<TTSSettings>): void {
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
  subscribe(listener: (settings: TTSSettings) => void): () => void {
    listeners.add(listener);
    return () => listeners.delete(listener);
  },

  /**
   * Toggle TTS enabled state
   */
  toggle(): void {
    currentSettings.enabled = !currentSettings.enabled;
    notifyListeners();
    persistSettings();
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
        tts_enabled: currentSettings.enabled,
        tts_provider: currentSettings.provider,
        tts_rate: currentSettings.rate,
      },
    });
  } catch (err) {
    console.error('[TTSStore] Failed to save settings:', err);
  }
}

/**
 * Load settings from Tauri backend
 */
export async function loadTTSSettings(): Promise<void> {
  try {
    const settings = await invoke<{
      tts_enabled?: boolean;
      tts_provider?: string;
      tts_rate?: number;
    }>('load_settings');

    currentSettings = {
      enabled: settings.tts_enabled ?? DEFAULT_SETTINGS.enabled,
      provider: (settings.tts_provider ?? DEFAULT_SETTINGS.provider) as TtsProvider,
      rate: settings.tts_rate ?? DEFAULT_SETTINGS.rate,
    };

    notifyListeners();
  } catch (err) {
    console.error('[TTSStore] Failed to load settings:', err);
  }
}
