/**
 * Subscription & API Keys Store
 *
 * Centralized state for subscription status and API keys.
 */

import { invoke } from '@tauri-apps/api/core';

export interface SubscriptionState {
  tier: 'free' | 'pro';
  apiKey: string;
  googleApiKey: string;
  elevenLabsApiKey: string;
  summariesThisMonth: number;
  lastReset: string;
}

const DEFAULT_STATE: SubscriptionState = {
  tier: 'free',
  apiKey: '',
  googleApiKey: '',
  elevenLabsApiKey: '',
  summariesThisMonth: 0,
  lastReset: '',
};

let currentState: SubscriptionState = { ...DEFAULT_STATE };
let listeners: Set<(state: SubscriptionState) => void> = new Set();

/**
 * Subscription store - reactive state for subscription and API keys
 */
export const subscriptionStore = {
  /**
   * Get current subscription state (reactive)
   */
  get(): SubscriptionState {
    return { ...currentState };
  },

  /**
   * Update a single subscription field
   */
  update<K extends keyof SubscriptionState>(
    key: K,
    value: SubscriptionState[K]
  ): void {
    currentState[key] = value;
    notifyListeners();

    // Persist API key changes
    if (key === 'apiKey' || key === 'googleApiKey' || key === 'elevenLabsApiKey') {
      persistApiKeys();
    }
  },

  /**
   * Update multiple fields at once
   */
  updateMany(settings: Partial<SubscriptionState>): void {
    currentState = { ...currentState, ...settings };
    notifyListeners();

    // Persist API key changes
    if (settings.apiKey !== undefined || settings.googleApiKey !== undefined || settings.elevenLabsApiKey !== undefined) {
      persistApiKeys();
    }
  },

  /**
   * Subscribe to state changes
   */
  subscribe(listener: (state: SubscriptionState) => void): () => void {
    listeners.add(listener);
    return () => listeners.delete(listener);
  },

  /**
   * Check if user is on Pro tier
   */
  isPro(): boolean {
    return currentState.tier === 'pro';
  },

  /**
   * Get remaining summaries for current month
   */
  getRemainingSummaries(): number {
    const limit = currentState.tier === 'free' ? 5 : 500;
    return Math.max(0, limit - currentState.summariesThisMonth);
  },
};

/**
 * Notify all listeners of state changes
 */
function notifyListeners(): void {
  listeners.forEach(listener => listener({ ...currentState }));
}

/**
 * Persist API keys to Tauri backend
 */
async function persistApiKeys(): Promise<void> {
  try {
    await invoke('save_settings', {
      settings: {
        soniox_api_key: currentState.apiKey,
        google_api_key: currentState.googleApiKey,
        elevenlabs_api_key: currentState.elevenLabsApiKey,
      },
    });
  } catch (err) {
    console.error('[SubscriptionStore] Failed to save API keys:', err);
  }
}

/**
 * Load subscription status from backend
 */
export async function loadSubscriptionStatus(): Promise<void> {
  try {
    const status = await invoke<{
      tier: string;
      summaries_this_month: number;
      last_summary_reset: string;
    }>('get_subscription_status');

    currentState.tier = status.tier as 'free' | 'pro';
    currentState.summariesThisMonth = status.summaries_this_month;
    currentState.lastReset = status.last_summary_reset;

    notifyListeners();
  } catch (err) {
    console.error('[SubscriptionStore] Failed to load subscription status:', err);
  }
}

/**
 * Load API keys from backend
 */
export async function loadApiKeys(): Promise<void> {
  try {
    const keys = await invoke<{
      soniox_api_key?: string;
      google_api_key?: string;
      elevenlabs_api_key?: string;
    }>('load_settings');

    currentState.apiKey = keys.soniox_api_key ?? '';
    currentState.googleApiKey = keys.google_api_key ?? '';
    currentState.elevenLabsApiKey = keys.elevenlabs_api_key ?? '';

    notifyListeners();
  } catch (err) {
    console.error('[SubscriptionStore] Failed to load API keys:', err);
  }
}
