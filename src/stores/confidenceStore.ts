/**
 * Confidence Store - Centralized state for confidence management
 *
 * Manages confidence thresholds, filtering, and per-language overrides
 * for transcript quality control.
 */

import { invoke } from '@tauri-apps/api/core';

export interface ConfidenceSettings {
  filterLevel: 'none' | 'low' | 'medium' | 'high';
  highThreshold: number;  // Default: 0.85
  mediumThreshold: number; // Default: 0.70
  lowThreshold: number;   // Default: 0.50
  showConfidenceScores: boolean;
  perLanguageOverrides: Record<string, Partial<ConfidenceSettings>>;
}

export interface ConfidenceStats {
  totalSegments: number;
  highConfidence: number;
  mediumConfidence: number;
  lowConfidence: number;
  averageConfidence: number;
  distribution: Record<string, number>;
}

const DEFAULT_SETTINGS: ConfidenceSettings = {
  filterLevel: 'none',
  highThreshold: 0.85,
  mediumThreshold: 0.70,
  lowThreshold: 0.50,
  showConfidenceScores: false,
  perLanguageOverrides: {}
};

let currentSettings: ConfidenceSettings = { ...DEFAULT_SETTINGS };
let listeners: Set<(settings: ConfidenceSettings) => void> = new Set();

/**
 * Confidence store - reactive settings for confidence management
 */
export const confidenceStore = {
  /**
   * Get current confidence settings (reactive)
   */
  get(): ConfidenceSettings {
    return { ...currentSettings };
  },

  /**
   * Update a single confidence setting
   */
  update<K extends keyof ConfidenceSettings>(
    key: K,
    value: ConfidenceSettings[K]
  ): void {
    currentSettings[key] = value;
    notifyListeners();
    persistSettings();
  },

  /**
   * Update multiple settings at once
   */
  updateMany(settings: Partial<ConfidenceSettings>): void {
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
   * Get effective settings for a specific language
   */
  getForLanguage(language: string): ConfidenceSettings {
    const overrides = currentSettings.perLanguageOverrides[language];
    return {
      ...currentSettings,
      ...(overrides || {})
    };
  },

  /**
   * Set language-specific override
   */
  setLanguageOverride(language: string, settings: Partial<ConfidenceSettings>): void {
    currentSettings.perLanguageOverrides[language] = {
      ...currentSettings.perLanguageOverrides[language],
      ...settings
    };
    notifyListeners();
    persistSettings();
  },

  /**
   * Clear language-specific override
   */
  clearLanguageOverride(language: string): void {
    delete currentSettings.perLanguageOverrides[language];
    notifyListeners();
    persistSettings();
  },

  /**
   * Determine confidence level for a given score
   */
  getConfidenceLevel(score: number, language?: string): 'high' | 'medium' | 'low' {
    const settings = language ? this.getForLanguage(language) : currentSettings;

    if (score >= settings.highThreshold) return 'high';
    if (score >= settings.mediumThreshold) return 'medium';
    return 'low';
  },

  /**
   * Check if a transcript segment should be filtered based on confidence
   */
  shouldFilter(score: number, language?: string): boolean {
    const settings = language ? this.getForLanguage(language) : currentSettings;

    switch (settings.filterLevel) {
      case 'high':
        return score < settings.highThreshold;
      case 'medium':
        return score < settings.mediumThreshold;
      case 'low':
        return score < settings.lowThreshold;
      case 'none':
        return false;
    }
  },

  /**
   * Subscribe to settings changes
   */
  subscribe(listener: (settings: ConfidenceSettings) => void): () => void {
    listeners.add(listener);
    return () => listeners.delete(listener);
  }
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
    await invoke('save_confidence_settings', {
      settings: {
        filter_level: currentSettings.filterLevel,
        high_threshold: currentSettings.highThreshold,
        medium_threshold: currentSettings.mediumThreshold,
        low_threshold: currentSettings.lowThreshold,
        show_confidence_scores: currentSettings.showConfidenceScores,
        per_language_overrides: currentSettings.perLanguageOverrides
      }
    });
  } catch (err) {
    console.error('[ConfidenceStore] Failed to save settings:', err);
  }
}

/**
 * Load settings from Tauri backend
 */
export async function loadConfidenceSettings(): Promise<void> {
  try {
    const settings = await invoke<{
      filter_level?: string;
      high_threshold?: number;
      medium_threshold?: number;
      low_threshold?: number;
      show_confidence_scores?: boolean;
      per_language_overrides?: Record<string, any>;
    }>('load_confidence_settings');

    currentSettings = {
      filterLevel: (settings.filter_level ?? DEFAULT_SETTINGS.filterLevel) as ConfidenceSettings['filterLevel'],
      highThreshold: settings.high_threshold ?? DEFAULT_SETTINGS.highThreshold,
      mediumThreshold: settings.medium_threshold ?? DEFAULT_SETTINGS.mediumThreshold,
      lowThreshold: settings.low_threshold ?? DEFAULT_SETTINGS.lowThreshold,
      showConfidenceScores: settings.show_confidence_scores ?? DEFAULT_SETTINGS.showConfidenceScores,
      perLanguageOverrides: settings.per_language_overrides ?? DEFAULT_SETTINGS.perLanguageOverrides
    };

    notifyListeners();
  } catch (err) {
    console.error('[ConfidenceStore] Failed to load settings:', err);
  }
}

/**
 * Calculate confidence statistics from transcript segments
 */
export function calculateConfidenceStats(
  segments: Array<{ confidence?: number; language?: string }>
): ConfidenceStats {
  const stats: ConfidenceStats = {
    totalSegments: segments.length,
    highConfidence: 0,
    mediumConfidence: 0,
    lowConfidence: 0,
    averageConfidence: 0,
    distribution: {}
  };

  let totalConfidence = 0;
  let segmentsWithConfidence = 0;

  for (const segment of segments) {
    if (segment.confidence === undefined) continue;

    segmentsWithConfidence++;
    totalConfidence += segment.confidence;

    const level = confidenceStore.getConfidenceLevel(segment.confidence, segment.language);
    switch (level) {
      case 'high':
        stats.highConfidence++;
        break;
      case 'medium':
        stats.mediumConfidence++;
        break;
      case 'low':
        stats.lowConfidence++;
        break;
    }

    // Track distribution by language
    const lang = segment.language || 'unknown';
    stats.distribution[lang] = (stats.distribution[lang] || 0) + 1;
  }

  if (segmentsWithConfidence > 0) {
    stats.averageConfidence = totalConfidence / segmentsWithConfidence;
  }

  return stats;
}