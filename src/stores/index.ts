/**
 * Centralized State Management
 *
 * Single source of truth for all application state.
 * Eliminates prop drilling and state duplication.
 */

export { translationStore, loadTranslationSettings } from './translationStore';
export { displayStore, loadDisplaySettings } from './displayStore';
export { ttsStore, loadTTSSettings } from './ttsStore';
export { subscriptionStore, loadSubscriptionStatus, loadApiKeys } from './subscriptionStore';

export type { TranslationSettings } from './translationStore';
export type { DisplaySettings } from './displayStore';
export type { TTSSettings } from './ttsStore';
export type { SubscriptionState } from './subscriptionStore';
