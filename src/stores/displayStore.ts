/**
 * Display Settings Store
 *
 * Centralized state for all display-related settings.
 */

import { invoke } from '@tauri-apps/api/core';

export interface DisplaySettings {
  opacity: number;
  fontSize: number;
  maxLines: number;
}

const DEFAULT_SETTINGS: DisplaySettings = {
  opacity: 0.88,
  fontSize: 14,
  maxLines: 100,
};

let currentSettings: DisplaySettings = { ...DEFAULT_SETTINGS };
let listeners: Set<(settings: DisplaySettings) => void> = new Set();

/**
 * Display store - reactive settings for display
 */
export const displayStore = {
  /**
   * Get current display settings (reactive)
   */
  get(): DisplaySettings {
    return { ...currentSettings };
  },

  /**
   * Update a single display setting
   */
  update<K extends keyof DisplaySettings>(
    key: K,
    value: DisplaySettings[K]
  ): void {
    currentSettings[key] = value;
    notifyListeners();
    persistSettings();
  },

  /**
   * Update multiple settings at once
   */
  updateMany(settings: Partial<DisplaySettings>): void {
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
  subscribe(listener: (settings: DisplaySettings) => void): () => void {
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
        opacity: currentSettings.opacity,
        font_size: currentSettings.fontSize,
        max_lines: currentSettings.maxLines,
      },
    });
  } catch (err) {
    console.error('[DisplayStore] Failed to save settings:', err);
  }
}

/**
 * Load settings from Tauri backend
 */
export async function loadDisplaySettings(): Promise<void> {
  try {
    const settings = await invoke<{
      opacity?: number;
      font_size?: number;
      max_lines?: number;
    }>('load_settings');

    currentSettings = {
      opacity: settings.opacity ?? DEFAULT_SETTINGS.opacity,
      fontSize: settings.font_size ?? DEFAULT_SETTINGS.fontSize,
      maxLines: settings.max_lines ?? DEFAULT_SETTINGS.maxLines,
    };

    notifyListeners();
  } catch (err) {
    console.error('[DisplayStore] Failed to load settings:', err);
  }
}
