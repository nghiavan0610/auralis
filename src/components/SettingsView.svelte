<script lang="ts">
  import ModelDownloader from './ModelDownloader.svelte';
  import TabNavigation from './TabNavigation.svelte';
  import Tooltip from './Tooltip.svelte';
  import Button from './Button.svelte';
  import type { OperatingMode, TranslationType, AudioSource } from '../types';
  import { getLangLabel, SUPPORTED_LANGUAGES } from '../js/lang';
  import { tts } from '../js/tts';
  import { check } from '@tauri-apps/plugin-updater';
  import { relaunch } from '@tauri-apps/plugin-process';
  import {
    initRevenueCat,
    getSubscriptionStatus,
    purchasePro,
    restorePurchases,
    getManageSubscriptionUrl,
    forceSync,
    type SubscriptionStatus,
    type PurchaseError
  } from '../js/revenuecat';
  import { getTabIcon } from '../js/tabIcons';

  interface PlatformInfo {
    os: string;
    system_audio_available: boolean;
    offline_mode_available: boolean;
  }

  let {
    mode = 'cloud',
    sonioxApiKey = '',
    googleApiKey = '',
    sourceLanguage = 'en',
    targetLanguage = 'vi',
    translationType = 'one_way',
    audioSource = 'microphone',
    isTranslating = false,
    opacity = 0.88,
    fontSize = 14,
    maxLines = 100,
    endpointDelay = 1.0,
    ttsEnabled = false,
    ttsVoice = '',
    ttsRate = 1.0,
    ttsProvider = 'webspeech' as 'webspeech' | 'edge' | 'google' | 'elevenlabs',
    elevenlabsApiKey = '',
    platformInfo = null as PlatformInfo | null,
    offlineSetupProgress = $bindable(0),
    offlineSetupMessage = $bindable(''),
    offlineSetupStep = $bindable(''),
    offlineReady = $bindable(false),
    initialTab = 'translation',
    onSave,
    onBack,
  }: {
    mode?: OperatingMode;
    sonioxApiKey?: string;
    googleApiKey?: string;
    elevenlabsApiKey?: string;
    sourceLanguage?: string;
    targetLanguage?: string;
    translationType?: TranslationType;
    audioSource?: AudioSource;
    isTranslating?: boolean;
    opacity?: number;
    fontSize?: number;
    maxLines?: number;
    endpointDelay?: number;
    ttsEnabled?: boolean;
    ttsVoice?: string;
    ttsRate?: number;
    ttsProvider?: 'webspeech' | 'edge' | 'google' | 'elevenlabs';
    platformInfo?: PlatformInfo | null;
    offlineSetupProgress?: number;
    offlineSetupMessage?: string;
    offlineSetupStep?: string;
    offlineReady?: boolean;
    initialTab?: 'translation' | 'display' | 'tts' | 'subscription' | 'about';
    onSave: (settings: {
      mode: OperatingMode;
      soniox_api_key: string;
      source_language: string;
      target_language: string;
      translation_type: TranslationType;
      audio_source: AudioSource;
      opacity: number;
      font_size: number;
      max_lines: number;
      endpoint_delay: number;
      tts_enabled: boolean;
      tts_voice: string;
      tts_rate: number;
      tts_provider: 'webspeech' | 'edge' | 'google' | 'elevenlabs';
      google_api_key: string;
      elevenlabs_api_key: string;
    }) => void;
    onBack: () => void;
  } = $props();

  // Derived platform availability (default to true if info not yet loaded)
  let systemAudioAvailable = $derived(platformInfo?.system_audio_available ?? true);
  let offlineModeAvailable = $derived(platformInfo?.offline_mode_available ?? true);

  // Local copies for editing
  let localMode: OperatingMode = $state('cloud');
  let localApiKey = $state('');
  let localSource = $state('en');
  let localTarget = $state('vi');
  let localTranslationType: TranslationType = $state(translationType);
  let localAudioSource: AudioSource = $state('microphone');
  let localOpacity = $state(0.88);
  let localFontSize = $state(14);
  let localMaxLines = $state(100);
  let localEndpointDelay = $state(1.0);
  let localEndpointTenths = $state(10);
  let localTtsEnabled = $state(false);
  let previewingVoice = $state(false);
  let expandedFaqIndex = $state<number | null>(null);
  let localTtsVoice = $state('');
  let localTtsRate = $state(1.0);
  let localTtsRateTenths = $state(10);
  let localTtsProvider: 'webspeech' | 'edge' | 'google' | 'elevenlabs' = $state('webspeech');
  let localGoogleApiKey = $state('');
  let localElevenlabsApiKey = $state('');
  let localClaudeApiKey = $state('');
  let localOpenaiApiKey = $state('');

  let voiceDropdownOpen = $state(false);
  let modeDropdownOpen = $state(false);
  let providerDropdownOpen = $state(false);
  let availableVoices: Array<{ name: string; lang: string; local: boolean; gender?: string }> = $state([]);
  let activeTab = $state<'translation' | 'display' | 'tts' | 'subscription' | 'about'>(initialTab);

  // Update activeTab when initialTab changes (e.g., when navigating from different entry points)
  $effect(() => {
    activeTab = initialTab;
  });

  // Update checker state
  let appVersion = $state('...');
  let updateStatus: 'idle' | 'checking' | 'up-to-date' | 'available' | 'downloading' | 'error' = $state('idle');
  let latestVersion = $state('');
  let updateProgress = $state(0);

  // Subscription state
  let subscriptionTier = $state<'free' | 'pro'>('free');
  let remainingSummaries = $state(5);
  let resetDate = $state('');
  let subscriptionStatus = $state<SubscriptionStatus | null>(null);
  let subscriptionLoaded = $state(false);

  // Purchase state
  let purchaseInProgress = $state(false);
  let purchaseSuccess = $state(false);
  let purchaseError = $state<PurchaseError | null>(null);
  let showRestoreSuccess = $state(false);

  // RevenueCat initialization
  let revenueCatInitialized = $state(false);
  let revenueCatAvailable = $state(true); // false if API key not configured

  // Slider works with integer (30–100), opacity is 0.3–1.0
  let localOpacityPercent = $state(88);

  // Sync from props when they change (e.g. settings re-opened)
  $effect(() => {
    localMode = mode;
    localApiKey = sonioxApiKey;
    localSource = sourceLanguage;
    localTarget = targetLanguage;
    localTranslationType = translationType;
    localAudioSource = audioSource;
    localOpacity = opacity;
    localFontSize = fontSize;
    localMaxLines = maxLines;
    localOpacityPercent = Math.round(opacity * 100);
    localEndpointDelay = endpointDelay;
    localEndpointTenths = Math.round(endpointDelay * 10);
    localTtsEnabled = ttsEnabled;
    localTtsVoice = ttsVoice;
    localTtsRate = ttsRate;
    localTtsRateTenths = Math.round(ttsRate * 10);
    localTtsProvider = ttsProvider;
    localGoogleApiKey = googleApiKey;
    localElevenlabsApiKey = elevenlabsApiKey;
  });

  // Load version and check for updates when About tab is opened
  // Use a flag to track if we've already checked, avoiding reactive dependency on updateStatus
  let aboutAutoChecked = $state(false);
  $effect(() => {
    if (activeTab === 'about') {
      loadVersion();
      if (!aboutAutoChecked) {
        aboutAutoChecked = true;
        checkForUpdates();
      }
    } else {
      aboutAutoChecked = false;
    }
  });

  // Load subscription status when Subscription tab is opened
  let subscriptionAutoChecked = $state(false);
  $effect(() => {
    if (activeTab === 'subscription') {
      if (!subscriptionAutoChecked) {
        subscriptionAutoChecked = true;
        loadSubscriptionStatus();
      }
    } else {
      subscriptionAutoChecked = false;
    }
  });

  // Track previous provider to only reset voice on actual change
  let prevProvider: string = $state('');

  // Load voices when TTS tab is opened or provider changes
  $effect(() => {
    const provider = localTtsProvider; // Read to create reactive dependency
    if (activeTab === 'tts') {
      tts.setProvider(provider);
      // Only reset voice when provider actually changes (not on tab open)
      if (prevProvider && prevProvider !== provider) {
        localTtsVoice = '';
      }
      prevProvider = provider;
      tts.getVoices().then((v) => {
        // Sort: target language voices first, then others by lang
        const target = localTarget.toLowerCase();
        v.sort((a, b) => {
          const aMatch = a.lang.toLowerCase() === target ? 0 : 1;
          const bMatch = b.lang.toLowerCase() === target ? 0 : 1;
          if (aMatch !== bMatch) return aMatch - bMatch;
          return a.name.localeCompare(b.name);
        });
        availableVoices = v;
      });
    }
  });

  // Validation state
  let googleApiKeyError = $state(false);
  let elevenlabsApiKeyError = $state(false);
  let sonioxApiKeyError = $state(false);

  function handleSave() {
    // Validate: Cloud mode requires a Soniox API key
    if (localMode === 'cloud' && !localApiKey.trim()) {
      sonioxApiKeyError = true;
      activeTab = 'translation';
      setTimeout(() => {
        document.getElementById('api-key')?.focus();
      }, 50);
      return;
    }
    sonioxApiKeyError = false;

    // Validate: Google TTS requires an API key
    if (localTtsProvider === 'google' && !localGoogleApiKey.trim()) {
      googleApiKeyError = true;
      activeTab = 'tts';
      setTimeout(() => {
        document.getElementById('google-api-key')?.focus();
      }, 50);
      return;
    }
    googleApiKeyError = false;

    // Validate: ElevenLabs TTS requires an API key
    if (localTtsProvider === 'elevenlabs' && !localElevenlabsApiKey.trim()) {
      elevenlabsApiKeyError = true;
      activeTab = 'tts';
      setTimeout(() => {
        document.getElementById('elevenlabs-api-key')?.focus();
      }, 50);
      return;
    }
    elevenlabsApiKeyError = false;

    onSave({
      mode: localMode,
      soniox_api_key: localApiKey,
      source_language: localSource,
      target_language: localTarget,
      translation_type: localTranslationType,
      audio_source: localAudioSource,
      opacity: localOpacity,
      font_size: localFontSize,
      max_lines: localMaxLines,
      endpoint_delay: localEndpointDelay,
      tts_enabled: localTtsEnabled,
      tts_voice: localTtsVoice,
      tts_rate: localTtsRate,
      tts_provider: localTtsProvider,
      google_api_key: localGoogleApiKey,
      elevenlabs_api_key: localElevenlabsApiKey,
    });
  }

  async function loadVersion() {
    try {
      const { getVersion } = await import('@tauri-apps/api/app');
      appVersion = await getVersion();
    } catch {
      appVersion = '0.1.0';
    }
  }

  async function loadSubscriptionStatus() {
    subscriptionLoaded = false;

    try {
      // Initialize RevenueCat on first load
      if (!revenueCatInitialized) {
        try {
          const initialized = await initRevenueCat();
          revenueCatInitialized = initialized;
          revenueCatAvailable = initialized;
        } catch (err) {
          console.error('Failed to initialize RevenueCat:', err);
          revenueCatAvailable = false;
          // Still try to get backend status
        }
      }

      // Get subscription status (combines RevenueCat + backend)
      const status = await getSubscriptionStatus();
      subscriptionStatus = status;
      subscriptionTier = status.tier;
      remainingSummaries = status.remaining_summaries;
      resetDate = status.reset_date;
      subscriptionLoaded = true;
    } catch (err) {
      console.error('Failed to load subscription status:', err);
      subscriptionLoaded = true;
      // Keep default values on error
    }
  }

  async function handleUpgrade() {
    purchaseInProgress = true;
    purchaseSuccess = false;
    purchaseError = null;
    showRestoreSuccess = false;

    try {
      console.log('[SettingsView] Starting purchase flow...');
      const status = await purchasePro();
      console.log('[SettingsView] Purchase successful:', status);
      // Force sync to ensure latest status from RevenueCat
      const syncedStatus = await forceSync();
      // Update UI with new status
      subscriptionStatus = syncedStatus;
      subscriptionTier = syncedStatus.tier;
      remainingSummaries = syncedStatus.remaining_summaries;
      resetDate = syncedStatus.reset_date;
      purchaseSuccess = true;

      // Auto-hide success message after 3 seconds
      setTimeout(() => {
        purchaseSuccess = false;
      }, 3000);
    } catch (err) {
      console.error('[SettingsView] Purchase failed:', err);
      console.error('[SettingsView] Error details:', JSON.stringify(err));

      // Parse the error
      if (err && typeof err === 'object' && 'type' in err && 'userMessage' in err) {
        purchaseError = err as PurchaseError;
      } else {
        purchaseError = {
          type: 'unknown',
          message: err instanceof Error ? err.message : 'Unknown error',
          userMessage: 'Purchase failed. Please try again.',
          recoverable: true,
        };
      }
    } finally {
      purchaseInProgress = false;
    }
  }

  async function handleRestore() {
    purchaseInProgress = true;
    purchaseError = null;
    showRestoreSuccess = false;

    try {
      const status = await restorePurchases();
      // Force sync to ensure latest status from RevenueCat
      const syncedStatus = await forceSync();
      // Update UI with restored status
      subscriptionStatus = syncedStatus;
      subscriptionTier = syncedStatus.tier;
      remainingSummaries = syncedStatus.remaining_summaries;
      resetDate = syncedStatus.reset_date;
      showRestoreSuccess = true;

      // Auto-hide success message after 3 seconds
      setTimeout(() => {
        showRestoreSuccess = false;
      }, 3000);
    } catch (err) {
      console.error('Restore failed:', err);

      if (err && typeof err === 'object' && 'type' in err && 'userMessage' in err) {
        purchaseError = err as PurchaseError;
      } else {
        purchaseError = {
          type: 'unknown',
          message: err instanceof Error ? err.message : 'Unknown error',
          userMessage: 'Failed to restore purchases. Please try again.',
          recoverable: true,
        };
      }
    } finally {
      purchaseInProgress = false;
    }
  }

  async function handleManageSubscription() {
    const url = getManageSubscriptionUrl();
    // Try Tauri's shell open first, fallback to window.open
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('plugin:shell|open', { path: url });
    } catch {
      // Fallback to window.open
      window.open(url, '_blank');
    }
  }

  function clearError() {
    purchaseError = null;
  }

  async function checkForUpdates() {
    updateStatus = 'checking';
    try {
      const update = await check();
      if (update?.available) {
        latestVersion = update.version;
        updateStatus = 'available';
      } else {
        updateStatus = 'up-to-date';
      }
    } catch {
      updateStatus = 'up-to-date';
    }
  }

  async function downloadAndInstallUpdate() {
    updateStatus = 'downloading';
    updateProgress = 0;
    try {
      const update = await check();
      if (!update?.available) {
        updateStatus = 'up-to-date';
        return;
      }
      let contentLength = 0;
      let downloaded = 0;
      await update.downloadAndInstall((event) => {
        switch (event.event) {
          case 'Started':
            contentLength = event.data.contentLength ?? 0;
            break;
          case 'Progress':
            downloaded += event.data.chunkLength;
            if (contentLength > 0) {
              updateProgress = Math.round((downloaded / contentLength) * 100);
            }
            break;
          case 'Finished':
            updateProgress = 100;
            break;
        }
      });
      await relaunch();
    } catch {
      updateStatus = 'error';
    }
  }

  // --- Voice display helpers ---

  const LANG_FLAGS: Record<string, string> = {
    en: '🇬🇧', vi: '🇻🇳', es: '🇪🇸', fr: '🇫🇷', de: '🇩🇪',
    zh: '🇨🇳', ja: '🇯🇵', ko: '🇰🇷', pt: '🇧🇷', ru: '🇷🇺',
    ar: '🇸🇦', hi: '🇮🇳',
    // Additional Edge TTS languages
    it: '🇮🇹', nl: '🇳🇱', pl: '🇵🇱', sv: '🇸🇪', da: '🇩🇰',
    fi: '🇫🇮', nb: '🇳🇴', tr: '🇹🇷', th: '🇹🇭', id: '🇮🇩',
    ms: '🇲🇾', tl: '🇵🇭', uk: '🇺🇦', cs: '🇨🇿', sk: '🇸🇰',
    hu: '🇭🇺', ro: '🇷🇴', bg: '🇧🇬', hr: '🇭🇷', sl: '🇸🇮',
    et: '🇪🇪', lv: '🇱🇻', lt: '🇱🇹', el: '🇬🇷', he: '🇮🇱',
    ca: '🇪🇸', eu: '🇪🇸', gl: '🇪🇸', mt: '🇲🇹', ga: '🇮🇪',
    cy: '🇬🇧', fil: '🇵🇭', te: '🇮🇳', ta: '🇮🇳', mr: '🇮🇳',
    gu: '🇮🇳', kn: '🇮🇳', ml: '🇮🇳', bn: '🇧🇩', ur: '🇵🇰',
    sw: '🇰🇪', am: '🇪🇹', jv: '🇮🇩', su: '🇮🇩', ne: '🇳🇵',
    km: '🇰🇭', lo: '🇱🇦', my: '🇲🇲', ka: '🇬🇪', az: '🇦🇿',
    uz: '🇺🇿', kk: '🇰🇿', mn: '🇲🇳', ps: '🇦🇫',
  };

  interface VoiceDisplay {
    value: string;
    name: string;
    gender: string;
    flag: string;
  }

  function getFlag(lang: string, name: string): string {
    try {
      const code = (lang ?? '').toLowerCase();
      if (LANG_FLAGS[code]) return LANG_FLAGS[code];
      const prefix = (name ?? '').split('-')[0]?.toLowerCase() ?? '';
      return LANG_FLAGS[prefix] ?? '🌐';
    } catch {
      return '🌐';
    }
  }

  function parseVoiceDisplay(voice: { name: string; lang: string; local: boolean; gender?: string }): VoiceDisplay {
    try {
      const flag = getFlag(voice.lang, voice.name);
      const gender = voice.gender ?? '—';
      let displayName = voice.name ?? '';
      const neuralMatch = displayName.match(/([A-Z][a-zA-Z]+)(?:Multilingual)?Neural$/);
      if (neuralMatch) {
        displayName = neuralMatch[1];
      }
      return { value: voice.name, name: displayName, gender, flag };
    } catch {
      return { value: voice.name ?? '', name: voice.name ?? '', gender: '—', flag: '🌐' };
    }
  }

  // Dropdown position for fixed positioning
  let dropdownPos = $state({ top: 0, left: 0, width: 0 });

  function positionDropdown(containerClass: string) {
    const trigger = document.querySelector(`${containerClass} .voice-trigger`) as HTMLElement;
    if (trigger) {
      const rect = trigger.getBoundingClientRect();
      const dropdownHeight = 280; // max-height
      const windowHeight = window.innerHeight;
      const windowWidth = window.innerWidth;

      let top = rect.bottom + 4;
      let left = rect.left;
      let width = rect.width;

      // Prevent dropdown from going off the bottom of the screen
      if (top + dropdownHeight > windowHeight - 20) {
        // Position above the trigger instead
        top = rect.top - dropdownHeight - 4;
      }

      // Prevent dropdown from going off the right side of the screen
      if (left + width > windowWidth - 20) {
        left = windowWidth - width - 20;
      }

      // Prevent dropdown from going off the left side of the screen
      if (left < 20) {
        left = 20;
      }

      dropdownPos = { top, left, width };
    }
  }

  function closeAllDropdowns() {
    modeDropdownOpen = false;
    providerDropdownOpen = false;
    voiceDropdownOpen = false;
  }

  function toggleModeDropdown() {
    const opening = !modeDropdownOpen;
    closeAllDropdowns();
    if (opening) {
      modeDropdownOpen = true;
      positionDropdown('.mode-dropdown-container');
    }
  }

  function toggleProviderDropdown() {
    const opening = !providerDropdownOpen;
    closeAllDropdowns();
    if (opening) {
      providerDropdownOpen = true;
      positionDropdown('.provider-dropdown-container');
    }
  }

  function toggleVoiceDropdown() {
    const opening = !voiceDropdownOpen;
    closeAllDropdowns();
    if (opening) {
      voiceDropdownOpen = true;
      positionDropdown('.voice-dropdown-container');
    }
  }

  function selectVoice(name: string) {
    localTtsVoice = name;
    voiceDropdownOpen = false;
  }

  async function previewVoice(voiceName: string) {
    if (previewingVoice) return;
    previewingVoice = true;

    try {
      const sampleText = "Hello, this is a preview of how this voice sounds.";
      await tts.speak(sampleText, localTarget);
    } catch (error) {
      console.error('[SettingsView] Voice preview error:', error);
    } finally {
      previewingVoice = false;
    }
  }

  interface TranslationPreset {
    id: string;
    name: string;
    desc: string;
    icon: string;
    config: {
      translationType: 'one_way' | 'two_way';
      endpointDelay: number;
      audioSource: 'microphone' | 'system' | 'both';
    };
  }

  const presets: TranslationPreset[] = [
    {
      id: 'lecture',
      name: 'Lecture',
      desc: 'One-way translation with longer delay for complete thoughts',
      icon: '🎤',
      config: { translationType: 'one_way', endpointDelay: 2.0, audioSource: 'microphone' }
    },
    {
      id: 'conversation',
      name: 'Conversation',
      desc: 'Two-way translation with fast response for natural dialogue',
      icon: '💬',
      config: { translationType: 'two_way', endpointDelay: 1.0, audioSource: 'microphone' }
    },
    {
      id: 'meeting',
      name: 'Meeting',
      desc: 'Two-way translation balanced for group discussions',
      icon: '👥',
      config: { translationType: 'two_way', endpointDelay: 1.5, audioSource: 'microphone' }
    },
    {
      id: 'interview',
      name: 'Interview',
      desc: 'Two-way translation optimized for Q&A sessions',
      icon: '🎙️',
      config: { translationType: 'two_way', endpointDelay: 1.2, audioSource: 'microphone' }
    }
  ];

  function applyPreset(preset: TranslationPreset) {
    localTranslationType = preset.config.translationType;
    localEndpointDelay = preset.config.endpointDelay;
    localEndpointTenths = Math.round(preset.config.endpointDelay * 10);
    localAudioSource = preset.config.audioSource;
  }

  function toggleFaq(index: number) {
    expandedFaqIndex = expandedFaqIndex === index ? null : index;
  }

  function handleGlobalClick(e: MouseEvent) {
    const target = e.target as HTMLElement;
    if (!target.closest('.voice-dropdown-container') && !target.closest('.mode-dropdown-container') && !target.closest('.provider-dropdown-container')) {
      closeAllDropdowns();
    }
  }

  function handleScroll() {
    closeAllDropdowns();
  }

  let tabContentEl: HTMLElement | undefined = $state();
</script>

<svelte:window onclick={handleGlobalClick} />

<div class="settings-view">
  <!-- Header (in drag region) -->
  <div class="settings-header" data-tauri-drag-region>
    <button class="btn-icon" onclick={onBack} title="Back">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <line x1="19" y1="12" x2="5" y2="12"/>
        <polyline points="12 19 5 12 12 5"/>
      </svg>
    </button>
    <span class="settings-title" data-tauri-drag-region>Settings</span>
    <div style="flex:1" data-tauri-drag-region></div>
    <Button onclick={handleSave} disabled={isTranslating}>Save</Button>
  </div>

  <!-- Tab Navigation -->
  <TabNavigation
    activeTab={activeTab}
    tabs={[
      { id: 'translation', label: 'Translation', icon: getTabIcon('translate') },
      { id: 'display', label: 'Display', icon: getTabIcon('display') },
      { id: 'tts', label: 'TTS', icon: getTabIcon('volume') },
      { id: 'subscription', label: 'Subscription', icon: getTabIcon('crown') },
      { id: 'about', label: 'About', icon: getTabIcon('info') }
    ]}
    onchange={(tabId) => activeTab = tabId}
  />

  <!-- Tab content -->
  <div class="tab-content" bind:this={tabContentEl} onscroll={handleScroll}>
    {#if activeTab === 'translation'}
      <div class="settings-section">
        <!-- 1. Mode selector -->
        <div class="section-label">Translation Mode</div>
        <p class="section-desc">Choose how speech is recognized and translated.</p>
        <div class="voice-dropdown-container mode-dropdown-container" class:open={modeDropdownOpen}>
          <button
            class="voice-trigger"
            onclick={toggleModeDropdown}
            disabled={isTranslating}
          >
            <span class="voice-trigger-text">
              {localMode === 'cloud' ? 'Cloud (Soniox)' : 'Offline (MLX)'}
            </span>
            <svg class="voice-trigger-arrow" class:open={modeDropdownOpen} width="10" height="6" viewBox="0 0 10 6" fill="none">
              <path d="M1 1L5 5L9 1" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </button>

          {#if modeDropdownOpen && !isTranslating}
            <div class="voice-dropdown" style="top:{dropdownPos.top}px;left:{dropdownPos.left}px;width:{dropdownPos.width}px">
              <div
                class="voice-option"
                class:active={localMode === 'cloud'}
                role="button"
                tabindex="0"
                onclick={() => { localMode = 'cloud'; modeDropdownOpen = false; }}
                onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); localMode = 'cloud'; modeDropdownOpen = false; }}}
              >
                <span class="voice-option-check">{localMode === 'cloud' ? '✓' : ''}</span>
                <div class="voice-option-content">
                  <span class="voice-option-name">Cloud (Soniox)</span>
                  <span class="voice-option-desc">~150ms latency, requires internet</span>
                </div>
              </div>
              <div
                class="voice-option"
                class:active={localMode === 'offline'}
                class:disabled={!offlineModeAvailable}
                role="button"
                tabindex="0"
                onclick={() => { if (offlineModeAvailable) { localMode = 'offline'; modeDropdownOpen = false; } }}
                onkeydown={(e) => { if ((e.key === 'Enter' || e.key === ' ') && offlineModeAvailable) { e.preventDefault(); localMode = 'offline'; modeDropdownOpen = false; } }}
              >
                <span class="voice-option-check">{localMode === 'offline' ? '✓' : ''}</span>
                <div class="voice-option-content">
                  <span class="voice-option-name">Offline (MLX)</span>
                  <span class="voice-option-desc">{offlineModeAvailable ? '~3s latency, works offline' : 'macOS only'}</span>
                </div>
              </div>
            </div>
          {/if}
        </div>

        <!-- 2. Mode-specific setup (right after mode choice) -->
        {#if localMode === 'cloud'}
          <div class="field">
            <div class="label-row">
              <label for="api-key">Soniox API Key</label>
              <span class="badge-required">Required</span>
            </div>
            <p class="field-desc">Required for cloud mode. Get a free key from soniox.com.</p>
            <input
              id="api-key"
              type="password"
              class:input-error={sonioxApiKeyError}
              placeholder="Enter your Soniox API key"
              bind:value={localApiKey}
              oninput={() => sonioxApiKeyError = false}
              disabled={isTranslating}
            />
            {#if sonioxApiKeyError}
              <p class="field-error">API key is required for cloud mode.</p>
            {/if}
            <a href="https://soniox.com/api-keys" target="_blank" rel="noopener noreferrer" class="field-link">
              Get API key
            </a>
          </div>
        {:else}
          <div class="field">
            <ModelDownloader
              bind:progress={offlineSetupProgress}
              bind:progressMessage={offlineSetupMessage}
              bind:progressStep={offlineSetupStep}
              bind:offlineReady={offlineReady}
            />
          </div>
        {/if}

        <!-- 3. Languages -->
        <div class="section-label">Languages</div>
        <p class="section-desc">Set the source language (what you speak) and target language (what to translate into).</p>
        <div class="language-row">
          <div class="field">
            <label for="src-lang">Source</label>
            <select id="src-lang" bind:value={localSource} disabled={isTranslating}>
              {#each SUPPORTED_LANGUAGES as lang}
                <option value={lang.code}>{lang.flag} {lang.name}</option>
              {/each}
            </select>
          </div>

          <div class="lang-arrow">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M5 12h14M12 5l7 7-7 7"/>
            </svg>
          </div>

          <div class="field">
            <label for="tgt-lang">Target</label>
            <select id="tgt-lang" bind:value={localTarget} disabled={isTranslating}>
              {#each SUPPORTED_LANGUAGES as lang}
                <option value={lang.code}>{lang.flag} {lang.name}</option>
              {/each}
            </select>
          </div>
        </div>

        <!-- 4. Translation direction -->
        <div class="section-label">Direction</div>
        <p class="section-desc">One way translates source to target. Two way auto-detects which language is spoken and translates to the other.</p>
        <div class="direction-cards">
          <label class="mode-card" class:active={localTranslationType === 'one_way'}>
            <input type="radio" name="direction" value="one_way" bind:group={localTranslationType} disabled={isTranslating} />
            <div class="mode-card-content">
              <Tooltip content="Best for: Lectures, presentations, meetings where everyone speaks one language" position="top">
                <span class="mode-name">
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="vertical-align: -2px; margin-right: 4px;">
                    <path d="M5 12h14"/>
                  </svg>
                  One Way
                </span>
              </Tooltip>
              <span class="mode-desc">{getLangLabel(localSource)} → {getLangLabel(localTarget)}</span>
            </div>
          </label>
          <label class="mode-card" class:active={localTranslationType === 'two_way'}>
            <input type="radio" name="direction" value="two_way" bind:group={localTranslationType} disabled={isTranslating} />
            <div class="mode-card-content">
              <Tooltip content="Best for: Conversations, bilingual meetings, interviews. Auto-detects active speaker." position="top">
                <span class="mode-name">
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="vertical-align: -2px; margin-right: 4px;">
                    <path d="M7 16l-4-4 4-4"/>
                    <path d="M17 8l4 4-4 4"/>
                    <line x1="3" y1="12" x2="21" y2="12"/>
                  </svg>
                  Two Way
                </span>
              </Tooltip>
              <span class="mode-desc">{getLangLabel(localSource)} ↔ {getLangLabel(localTarget)}</span>
            </div>
          </label>
        </div>

        <!-- Quick Presets -->
        <div class="section-label">Quick Presets</div>
        <p class="section-desc">One-click configurations for common translation scenarios. Automatically adjusts direction, delay, and audio source.</p>
        <div class="presets-grid">
          {#each presets as preset}
            <button
              class="preset-card"
              class:active={
                localTranslationType === preset.config.translationType &&
                Math.abs(localEndpointDelay - preset.config.endpointDelay) < 0.1 &&
                localAudioSource === preset.config.audioSource
              }
              onclick={() => applyPreset(preset)}
              disabled={isTranslating}
            >
              <span class="preset-icon">{preset.icon}</span>
              <div class="preset-content">
                <span class="preset-name">{preset.name}</span>
                <span class="preset-desc">{preset.desc}</span>
              </div>
              <svg class="preset-arrow" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <polyline points="9 18 15 12 9 6"/>
              </svg>
            </button>
          {/each}
        </div>

        <!-- 5. Audio source -->
        <div class="section-label">Audio Source</div>
        <p class="section-desc">Where to capture audio from. System audio captures sound playing on your computer (requires Screen Recording permission).</p>
        <div class="source-cards">
          <label class="mode-card source-card" class:active={localAudioSource === 'microphone'}>
            <input type="radio" name="audio-source" value="microphone" bind:group={localAudioSource} disabled={isTranslating} />
            <div class="mode-card-content">
              <span class="mode-name">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="vertical-align: -2px; margin-right: 4px;">
                  <path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z"/>
                  <path d="M19 10v2a7 7 0 0 1-14 0v-2"/>
                  <line x1="12" y1="19" x2="12" y2="23"/>
                  <line x1="8" y1="23" x2="16" y2="23"/>
                </svg>
                Microphone
              </span>
              <span class="mode-desc">Capture from your mic</span>
            </div>
          </label>
          <label class="mode-card source-card" class:active={localAudioSource === 'system'} class:disabled={!systemAudioAvailable}>
            <input type="radio" name="audio-source" value="system" bind:group={localAudioSource} disabled={isTranslating || !systemAudioAvailable} />
            <div class="mode-card-content">
              <span class="mode-name">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="vertical-align: -2px; margin-right: 4px;">
                  <path d="M2 10v4m4-6v8m4-10v12m4-8v4m4-6v8m4-4v0"/>
                </svg>
                System Audio
              </span>
              <span class="mode-desc">{systemAudioAvailable ? 'Capture computer audio' : 'macOS only'}</span>
            </div>
          </label>
          <label class="mode-card source-card" class:active={localAudioSource === 'both'} class:disabled={!systemAudioAvailable}>
            <input type="radio" name="audio-source" value="both" bind:group={localAudioSource} disabled={isTranslating || !systemAudioAvailable} />
            <div class="mode-card-content">
              <span class="mode-name">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="vertical-align: -2px; margin-right: 4px;">
                  <path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z"/>
                  <path d="M19 10v2a7 7 0 0 1-14 0v-2"/>
                  <line x1="12" y1="19" x2="12" y2="23"/>
                  <line x1="8" y1="23" x2="16" y2="23"/>
                </svg>
                Both
              </span>
              <span class="mode-desc">{systemAudioAvailable ? 'Mic + system audio' : 'macOS only'}</span>
            </div>
          </label>
        </div>

        <!-- Endpoint Delay card -->
        <div class="slider-card">
          <div class="slider-card-header">
            <Tooltip
              content="Lower values (0.5-1.0s) = Faster but may cut off speakers. Higher values (2.0-3.0s) = More complete but slower. Recommended: 1.5s for most situations."
              position="top"
            >
              <span class="slider-card-label">Endpoint Delay</span>
            </Tooltip>
            <span class="slider-card-value">{localEndpointDelay.toFixed(1)}s</span>
          </div>
          <p class="slider-card-desc">How long to wait after silence before finalizing a transcript segment. Lower for faster output, higher to avoid cutting off pauses.</p>
          <input
            type="range"
            min="5"
            max="30"
            step="1"
            bind:value={localEndpointTenths}
            oninput={() => localEndpointDelay = localEndpointTenths / 10}
            class="slider"
            style="--fill: {((localEndpointTenths - 5) / 25) * 100}%"
          />
        </div>
      </div>

    {:else if activeTab === 'display'}
      <div class="settings-section">
        <div class="section-label">Appearance</div>
        <p class="section-desc">Adjust how the overlay looks and feels.</p>

        <!-- Live Preview -->
        <div class="display-preview-section">
          <div class="preview-header">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--accent)" stroke-width="2">
              <path d="M1 12s4-8 11-8 11 8 11 8-11 11-8z"/>
              <circle cx="12" cy="12" r="3"/>
            </svg>
            <span>Live Preview</span>
          </div>
          <div class="preview-overlay" style="background-color: rgba(15, 15, 20, {localOpacity}); font-size: {localFontSize}px;">
            <div class="preview-transcript" style="-webkit-line-clamp: {localMaxLines}; -webkit-box-orient: vertical; overflow: hidden;">
              <p class="preview-entry">Welcome to Auralis real-time translation.</p>
              <p class="preview-entry">This preview shows how your translated text will appear with the current display settings.</p>
              <p class="preview-entry">Adjust the sliders above to see how opacity, font size, and max lines affect readability in real-time.</p>
              <p class="preview-entry">Longer transcripts like this one will be truncated based on your max lines setting, helping you find the perfect balance between information density and screen space.</p>
              <p class="preview-entry">The preview background opacity matches your overlay setting, so you can see how transparent the subtitle background will be during translation.</p>
            </div>
          </div>
        </div>

        <!-- Opacity card -->
        <div class="slider-card">
          <div class="slider-card-header">
            <span class="slider-card-label">Background Opacity</span>
            <span class="slider-card-value">{Math.round(localOpacity * 100)}%</span>
          </div>
          <input
            type="range"
            min="30"
            max="100"
            step="1"
            bind:value={localOpacityPercent}
            oninput={() => {
              localOpacity = Number(localOpacityPercent) / 100;
              console.log('[SettingsView] Opacity changed:', { percent: localOpacityPercent, opacity: localOpacity });
            }}
            class="slider"
            style="--fill: {((localOpacityPercent - 30) / 70) * 100}%"
          />
        </div>

        <!-- Font Size card -->
        <div class="slider-card">
          <div class="slider-card-header">
            <span class="slider-card-label">Font Size</span>
            <span class="slider-card-value">{localFontSize}px</span>
          </div>
          <input
            type="range"
            min="12"
            max="24"
            step="1"
            bind:value={localFontSize}
            class="slider"
            style="--fill: {((localFontSize - 12) / 12) * 100}%"
          />
        </div>

        <!-- Max Lines card -->
        <div class="slider-card">
          <div class="slider-card-header">
            <span class="slider-card-label">Max Transcript Lines</span>
            <span class="slider-card-value">{localMaxLines}</span>
          </div>
          <input
            type="range"
            min="10"
            max="200"
            step="10"
            bind:value={localMaxLines}
            class="slider"
            style="--fill: {((localMaxLines - 10) / 190) * 100}%"
          />
        </div>
      </div>

    {:else if activeTab === 'tts'}
      <div class="settings-section">
        <!-- Enable toggle -->
        <div class="section-label">Text-to-Speech</div>
        <p class="section-desc">When enabled, translated text is automatically spoken aloud.</p>
        <div class="toggle-card">
          <div class="toggle-card-info">
            <span class="toggle-card-label">Speak translations aloud</span>
          </div>
          <button
            class="toggle"
            class:active={localTtsEnabled}
            onclick={() => localTtsEnabled = !localTtsEnabled}
            disabled={isTranslating}
            aria-label="Toggle text-to-speech"
          >
            <span class="toggle-thumb"></span>
          </button>
        </div>

        <!-- Provider dropdown -->
        <div class="section-label" style="margin-top: var(--space-lg);">Provider</div>
        <p class="section-desc">Choose between offline system voices or cloud-based TTS providers.</p>
        <div class="voice-dropdown-container provider-dropdown-container" class:open={providerDropdownOpen}>
          <button
            class="voice-trigger"
            onclick={toggleProviderDropdown}
            disabled={isTranslating || !localTtsEnabled}
          >
            <span class="voice-trigger-text">
              {#if localTtsProvider === 'webspeech'}
                Web Speech
              {:else if localTtsProvider === 'edge'}
                Edge TTS
              {:else if localTtsProvider === 'google'}
                Google Cloud TTS
              {:else}
                ElevenLabs
              {/if}
            </span>
            <svg class="voice-trigger-arrow" class:open={providerDropdownOpen} width="10" height="6" viewBox="0 0 10 6" fill="none">
              <path d="M1 1L5 5L9 1" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </button>

          {#if providerDropdownOpen && !isTranslating && localTtsEnabled}
            <div class="voice-dropdown" style="top:{dropdownPos.top}px;left:{dropdownPos.left}px;width:{dropdownPos.width}px">
              <div
                class="voice-option"
                class:active={localTtsProvider === 'webspeech'}
                role="button"
                tabindex="0"
                onclick={() => { localTtsProvider = 'webspeech'; providerDropdownOpen = false; }}
                onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); localTtsProvider = 'webspeech'; providerDropdownOpen = false; }}}
              >
                <span class="voice-option-check">{localTtsProvider === 'webspeech' ? '✓' : ''}</span>
                <div class="voice-option-content">
                  <span class="voice-option-name">Web Speech</span>
                  <span class="voice-option-desc">Offline, system voices</span>
                </div>
              </div>
              <div
                class="voice-option"
                class:active={localTtsProvider === 'edge'}
                role="button"
                tabindex="0"
                onclick={() => { localTtsProvider = 'edge'; providerDropdownOpen = false; }}
                onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); localTtsProvider = 'edge'; providerDropdownOpen = false; }}}
              >
                <span class="voice-option-check">{localTtsProvider === 'edge' ? '✓' : ''}</span>
                <div class="voice-option-content">
                  <span class="voice-option-name">Edge TTS</span>
                  <span class="voice-option-desc">Cloud, 40+ languages, free</span>
                </div>
              </div>
              <div
                class="voice-option"
                class:active={localTtsProvider === 'google'}
                role="button"
                tabindex="0"
                onclick={() => { localTtsProvider = 'google'; providerDropdownOpen = false; }}
                onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); localTtsProvider = 'google'; providerDropdownOpen = false; }}}
              >
                <span class="voice-option-check">{localTtsProvider === 'google' ? '✓' : ''}</span>
                <div class="voice-option-content">
                  <span class="voice-option-name">Google Cloud TTS</span>
                  <span class="voice-option-desc">WaveNet voices, requires API key</span>
                </div>
              </div>
              <div
                class="voice-option"
                class:active={localTtsProvider === 'elevenlabs'}
                role="button"
                tabindex="0"
                onclick={() => { localTtsProvider = 'elevenlabs'; providerDropdownOpen = false; }}
                onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); localTtsProvider = 'elevenlabs'; providerDropdownOpen = false; }}}
              >
                <span class="voice-option-check">{localTtsProvider === 'elevenlabs' ? '✓' : ''}</span>
                <div class="voice-option-content">
                  <span class="voice-option-name">ElevenLabs</span>
                  <span class="voice-option-desc">Ultra-realistic AI voices, API key</span>
                </div>
              </div>
            </div>
          {/if}
        </div>

        <!-- Google API Key (shown when Google is selected) -->
        {#if localTtsProvider === 'google'}
          <div class="field">
            <div class="label-row">
              <label for="google-api-key">Google Cloud API Key</label>
              <span class="badge-required">Required</span>
            </div>
            <p class="field-desc">Required for Google Cloud Text-to-Speech.</p>
            <input
              id="google-api-key"
              type="password"
              class:input-error={googleApiKeyError}
              placeholder="Enter your Google Cloud TTS API key"
              bind:value={localGoogleApiKey}
              oninput={() => googleApiKeyError = false}
              disabled={isTranslating}
            />
            {#if googleApiKeyError}
              <p class="field-error">API key is required to use Google Cloud TTS.</p>
            {/if}
            <a href="https://console.cloud.google.com/apis/credentials" target="_blank" rel="noopener noreferrer" class="field-link">
              Get API key
            </a>
          </div>
        {/if}

        <!-- ElevenLabs API Key (shown when ElevenLabs is selected) -->
        {#if localTtsProvider === 'elevenlabs'}
          <div class="field">
            <div class="label-row">
              <label for="elevenlabs-api-key">ElevenLabs API Key</label>
              <span class="badge-required">Required</span>
            </div>
            <p class="field-desc">Required for ElevenLabs Text-to-Speech.</p>
            <input
              id="elevenlabs-api-key"
              type="password"
              class:input-error={elevenlabsApiKeyError}
              placeholder="Enter your ElevenLabs API key"
              bind:value={localElevenlabsApiKey}
              oninput={() => elevenlabsApiKeyError = false}
              disabled={isTranslating}
            />
            {#if elevenlabsApiKeyError}
              <p class="field-error">API key is required to use ElevenLabs TTS.</p>
            {/if}
            <a href="https://elevenlabs.io/app/settings/api-keys" target="_blank" rel="noopener noreferrer" class="field-link">
              Get API key
            </a>
          </div>
        {/if}

        <!-- Voice selector -->
        <div class="section-label" style="margin-top: var(--space-lg);">Voice</div>
        {#if localTtsProvider === 'edge' || localTtsProvider === 'google' || localTtsProvider === 'elevenlabs'}
          <p class="section-desc">{localTtsProvider === 'edge' ? 'Microsoft Neural voices for high-quality speech.' : localTtsProvider === 'google' ? 'Google WaveNet/Neural2 voices for natural speech.' : 'ElevenLabs ultra-realistic AI voices.'}</p>
          <div class="voice-dropdown-container" class:open={voiceDropdownOpen}>
            <!-- Trigger -->
            <button
              class="voice-trigger"
              onclick={toggleVoiceDropdown}
              disabled={isTranslating || !localTtsEnabled}
            >
              {#if localTtsVoice}
                {@const selected = availableVoices.find(v => v.name === localTtsVoice)}
                {#if selected}
                  {@const display = parseVoiceDisplay(selected)}
                  <span class="voice-trigger-text">
                    <span class="voice-trigger-flag">{display.flag}</span>
                    {display.name} — {display.gender}
                  </span>
                {:else}
                  <span class="voice-trigger-text">{localTtsVoice}</span>
                {/if}
              {:else}
                <span class="voice-trigger-text voice-trigger-placeholder">Auto (best for language)</span>
              {/if}
              <svg class="voice-trigger-arrow" class:open={voiceDropdownOpen} width="10" height="6" viewBox="0 0 10 6" fill="none">
                <path d="M1 1L5 5L9 1" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
            </button>

            <!-- Dropdown list -->
            {#if voiceDropdownOpen && !isTranslating && localTtsEnabled}
              <div class="voice-dropdown" style="top:{dropdownPos.top}px;left:{dropdownPos.left}px;width:{dropdownPos.width}px">
                <button
                  class="voice-option"
                  class:active={localTtsVoice === ''}
                  onclick={() => selectVoice('')}
                >
                  <span class="voice-option-check">{localTtsVoice === '' ? '✓' : ''}</span>
                  <div class="voice-option-content">
                    <span class="voice-option-name">Auto</span>
                    <span class="voice-option-desc">Best for language</span>
                  </div>
                </button>
                {#each availableVoices as voice (voice.name)}
                  <div
                    class="voice-option"
                    class:active={localTtsVoice === voice.name}
                    role="button"
                    tabindex="0"
                    onclick={() => selectVoice(voice.name)}
                    onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); selectVoice(voice.name); }}}
                  >
                    <span class="voice-option-check">{localTtsVoice === voice.name ? '✓' : ''}</span>
                    <div class="voice-option-content">
                      <div class="voice-option-main">
                        <span class="voice-option-name">{parseVoiceDisplay(voice).name}</span>
                        <button
                          class="voice-preview-btn"
                          onclick={(e) => { e.stopPropagation(); previewVoice(voice.name); }}
                          disabled={previewingVoice}
                          aria-label="Preview this voice"
                        >
                          {#if previewingVoice}
                            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="previewing-icon">
                              <circle cx="12" cy="12" r="10" stroke-dasharray="32" stroke-dashoffset="32" class="preview-spinner"/>
                            </svg>
                          {:else}
                            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="var(--accent)" stroke-width="2">
                              <polygon points="5 3 19 12 5 21 12"/>
                              <line x1="4.93" y1="16.07" x2="16.07" y2="8.93"/>
                            </svg>
                          {/if}
                        </button>
                      </div>
                      <span class="voice-option-desc">{voice.gender ?? '—'}</span>
                    </div>
                    <span class="voice-option-flag">{getFlag(voice.lang ?? '', voice.name ?? '')}</span>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        {:else}
          <p class="section-desc">Web Speech uses your system's default voice. Switch to another provider for voice selection.</p>
          <div class="voice-dropdown-container">
            <button class="voice-trigger" disabled>
              <span class="voice-trigger-text voice-trigger-placeholder">System default voice</span>
              <svg class="voice-trigger-arrow" width="10" height="6" viewBox="0 0 10 6" fill="none">
                <path d="M1 1L5 5L9 1" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
            </button>
          </div>
        {/if}

        <!-- Speed slider -->
        <div class="slider-card">
          <div class="slider-card-header">
            <span class="slider-card-label">Speed</span>
            <span class="slider-card-value">{localTtsRate.toFixed(1)}x</span>
          </div>
          <p class="slider-card-desc">Playback speed for spoken translations. 1.0x is normal speed.</p>
          <input
            type="range"
            min="5"
            max="20"
            step="1"
            bind:value={localTtsRateTenths}
            oninput={() => localTtsRate = localTtsRateTenths / 10}
            class="slider"
            style="--fill: {((localTtsRateTenths - 5) / 15) * 100}%"
          />
        </div>
      </div>

    {:else if activeTab === 'subscription'}
      <div class="settings-section">
        <div class="section-label">Subscription</div>
        <p class="section-desc">Choose your plan to unlock better AI and more features.</p>

        <!-- Current Plan Status Banner -->
        <div class="plan-status-banner" class:free-plan={subscriptionTier === 'free'} class:pro-plan={subscriptionTier === 'pro'}>
          <div class="plan-status-content">
            <div class="plan-status-icon">
              {#if subscriptionTier === 'pro'}
                <svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z"/>
                </svg>
              {:else}
                <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <circle cx="12" cy="12" r="10"/>
                  <path d="M12 6v6l4 2"/>
                </svg>
              {/if}
            </div>
            <div class="plan-status-text">
              <div class="plan-status-title">
                {subscriptionTier === 'pro' ? 'PRO PLAN' : 'FREE PLAN'}
                {#if subscriptionStatus?.is_trial}
                  <span class="trial-badge">Trial</span>
                {/if}
              </div>
              <div class="plan-status-subtitle">
                {#if subscriptionTier === 'pro'}
                  {#if subscriptionStatus?.will_renew}
                    Renews {new Date(subscriptionStatus.expire_date || '').toLocaleDateString()}
                  {:else}
                    Expires {new Date(subscriptionStatus.expire_date || '').toLocaleDateString()}
                  {/if}
                {:else}
                  {remainingSummaries} of 5 summaries remaining this month
                {/if}
              </div>
            </div>
          </div>
          <div class="plan-status-action">
            {#if subscriptionTier === 'free'}
              <button class="banner-upgrade-btn" onclick={handleUpgrade} disabled={purchaseInProgress}>
                Upgrade Now
              </button>
            {:else}
              <button class="banner-manage-btn" onclick={handleManageSubscription}>
                Manage
              </button>
            {/if}
          </div>
        </div>

        <!-- Status messages -->
        {#if showRestoreSuccess}
          <div class="status-message success" style="margin-bottom: var(--space-md);">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <polyline points="20 6 9 17 4 12"/>
            </svg>
            <span>Purchases restored successfully</span>
          </div>
        {/if}

        {#if purchaseSuccess}
          <div class="status-message success" style="margin-bottom: var(--space-md);">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <polyline points="20 6 9 17 4 12"/>
            </svg>
            <span>Upgrade successful! Welcome to Pro</span>
          </div>
        {/if}

        {#if purchaseError}
          <div class="status-message error" style="margin-bottom: var(--space-md);">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="12" cy="12" r="10"/>
              <line x1="12" y1="8" x2="12" y2="12"/>
              <line x1="12" y1="16" x2="12.01" y2="16"/>
            </svg>
            <span>{purchaseError.userMessage}</span>
            {#if purchaseError.recoverable}
              <button class="status-dismiss" onclick={clearError} title="Dismiss">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <line x1="18" y1="6" x2="6" y2="18"/>
                  <line x1="6" y1="6" x2="18" y2="18"/>
                </svg>
              </button>
            {/if}
          </div>
        {/if}

        <!-- Usage Visualization -->
        <div class="usage-section">
          <div class="section-label" style="margin-bottom: var(--space-sm);">Usage This Month</div>
          <div class="usage-visual-card">
            <div class="usage-progress-container">
              <svg class="usage-progress-ring" width="120" height="120">
                <circle
                  class="usage-progress-bg"
                  cx="60"
                  cy="60"
                  r="50"
                  fill="none"
                  stroke="var(--border)"
                  stroke-width="8"
                />
                <circle
                  class="usage-progress-fill"
                  cx="60"
                  cy="60"
                  r="50"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="8"
                  stroke-linecap="round"
                  style="--progress: {(subscriptionTier === 'free' ? (5 - remainingSummaries) / 5 : 0.15)}"
                />
              </svg>
              <div class="usage-progress-text">
                <div class="usage-progress-value">{subscriptionTier === 'free' ? (5 - remainingSummaries) : '75+'}</div>
                <div class="usage-progress-label">used</div>
              </div>
            </div>
            <div class="usage-stats">
              <div class="usage-stat">
                <span class="usage-stat-label">Plan</span>
                <span class="usage-stat-value" class:free-stat={subscriptionTier === 'free'} class:pro-stat={subscriptionTier === 'pro'}>
                  {subscriptionTier === 'free' ? 'Free' : 'Pro'}
                </span>
              </div>
              <div class="usage-stat">
                <span class="usage-stat-label">Monthly limit</span>
                <span class="usage-stat-value">{subscriptionTier === 'free' ? '5' : '500'} summaries</span>
              </div>
              <div class="usage-stat">
                <span class="usage-stat-label">Remaining</span>
                <span class="usage-stat-value" class:low-remaining={subscriptionTier === 'free' && remainingSummaries <= 1}>
                  {subscriptionTier === 'free' ? remainingSummaries : '425+'}
                </span>
              </div>
            </div>
          </div>
          {#if subscriptionTier === 'free' && remainingSummaries <= 1}
            <div class="usage-warning">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--warning)" stroke-width="2">
                <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/>
                <line x1="12" y1="9" x2="12" y2="13"/>
                <line x1="12" y1="17" x2="12.01" y2="17"/>
              </svg>
              <span>You've used most of your monthly summaries. Upgrade to Pro for up to 500 summaries per month.</span>
            </div>
          {/if}
        </div>

        <!-- Restore purchases with explanation -->
        <div class="restore-section">
          <button
            class="restore-btn"
            onclick={handleRestore}
            disabled={purchaseInProgress}
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="margin-right: var(--space-xs);">
              <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 12"/>
              <path d="M3 3v9h9"/>
            </svg>
            Restore Purchases
          </button>
          <div class="restore-help">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="12" cy="12" r="10"/>
              <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/>
              <line x1="12" y1="17" x2="12.01" y2="17"/>
            </svg>
            <span>If you subscribed on another device, click here to sync your subscription</span>
          </div>
        </div>

        <div style="display: grid; gap: var(--space-md); margin-top: var(--space-md);">
          <!-- Free Tier -->
          <div class="tier-card" class:current-tier={subscriptionTier === 'free'}>
            <div class="tier-header">
              <div class="tier-info">
                <span class="tier-name">Free</span>
                <span class="tier-price">$0/month</span>
              </div>
              <div class="tier-badge" class:free-badge={subscriptionTier === 'free'} class:pro-badge={subscriptionTier !== 'free'}>
                {subscriptionTier === 'free' ? 'Current' : 'Downgrade'}
              </div>
            </div>
            <div class="tier-features">
              <div class="tier-feature">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--success)" stroke-width="2"><polyline points="20 6 9 17 4 12"/></svg>
                <span><strong>5 summaries</strong> per month</span>
              </div>
              <div class="tier-feature">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--success)" stroke-width="2"><polyline points="20 6 9 17 4 12"/></svg>
                <span>Gemma-3 offline model</span>
              </div>
              <div class="tier-feature">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--success)" stroke-width="2"><polyline points="20 6 9 17 4 12"/></svg>
                <span>Key points + overview</span>
              </div>
              <div class="tier-feature">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--text-dim)" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
                <span style="color: var(--text-dim);">No action items or decisions</span>
              </div>
            </div>
          </div>

          <!-- Pro Tier -->
          <div class="tier-card pro-tier" class:current-tier={subscriptionTier === 'pro'}>
            <div class="tier-header">
              <div class="tier-info">
                <span class="tier-name">Pro</span>
                <span class="tier-price">$9/month</span>
              </div>
              <div class="tier-badge" class:free-badge={subscriptionTier !== 'pro'} class:pro-badge={subscriptionTier === 'pro'}>
                {subscriptionTier === 'pro' ? 'Current' : 'Upgrade'}
              </div>
            </div>
            <div class="tier-features">
              <div class="tier-feature">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--success)" stroke-width="2"><polyline points="20 6 9 17 4 12"/></svg>
                <span><strong>Up to 500 summaries</strong> per month</span>
              </div>
              <div class="tier-feature">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--success)" stroke-width="2"><polyline points="20 6 9 17 4 12"/></svg>
                <span><strong>Advanced AI</strong> model</span>
              </div>
              <div class="tier-feature">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--success)" stroke-width="2"><polyline points="20 6 9 17 4 12"/></svg>
                <span>Action items + decisions</span>
              </div>
              <div class="tier-feature">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--success)" stroke-width="2"><polyline points="20 6 9 17 4 12"/></svg>
                <span>No API keys needed</span>
              </div>
            </div>
            {#if subscriptionTier === 'pro'}
              <!-- Manage subscription button for Pro users -->
              <button class="tier-manage-btn" onclick={handleManageSubscription}>
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="margin-right: var(--space-xs);">
                  <path d="M12 20h9"/>
                  <path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"/>
                </svg>
                Manage Subscription
              </button>
            {:else}
              <!-- Upgrade button for Free users -->
              <button
                class="tier-upgrade-btn"
                class:loading={purchaseInProgress}
                onclick={handleUpgrade}
                disabled={purchaseInProgress}
              >
                {#if purchaseInProgress}
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="spin" style="margin-right: var(--space-xs);">
                    <circle cx="12" cy="12" r="10" stroke-opacity="0.25"/>
                    <path d="M12 2a10 10 0 0 1 10 10" stroke-opacity="1"/>
                  </svg>
                  Processing...
                {:else}
                  Upgrade to Pro
                {/if}
              </button>
            {/if}
          </div>
        </div>

        <!-- Help text -->
        {#if subscriptionTier !== 'pro'}
          <div style="margin-top: var(--space-lg); padding: var(--space-md); background: rgba(99, 102, 241, 0.05); border: 1px solid rgba(99, 102, 241, 0.1); border-radius: var(--radius-md); font-size: var(--font-size-sm); color: var(--text-secondary);">
            <strong style="color: var(--text-primary);">How it works:</strong> When you click "Upgrade to Pro", you'll be taken to a secure payment page. After payment, your Pro benefits activate immediately.
          </div>
        {:else}
          <div style="margin-top: var(--space-lg); padding: var(--space-md); background: rgba(99, 102, 241, 0.05); border: 1px solid rgba(99, 102, 241, 0.1); border-radius: var(--radius-md); font-size: var(--font-size-sm); color: var(--text-secondary);">
            <strong style="color: var(--text-primary);">Need help?</strong> Click "Manage Subscription" to update your payment method, cancel, or view your subscription history.
          </div>
        {/if}

        <!-- FAQ Section -->
        <div class="faq-section">
          <div class="section-label" style="margin-bottom: var(--space-md);">Frequently Asked Questions</div>

          <div class="faq-list">
            <div class="faq-item">
              <button class="faq-question" onclick={() => toggleFaq(0)}>
                <span>What's the difference between Free and Pro?</span>
                <svg class="faq-icon" class:expanded={expandedFaqIndex === 0} width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <polyline points="6 9 12 15 18 9"/>
                </svg>
              </button>
              <div class="faq-answer" class:expanded={expandedFaqIndex === 0}>
                <p>Free gives you 5 AI summaries per month using the Gemma-3 offline model. Pro unlocks up to 500 summaries per month with an advanced AI model that extracts action items and decisions from your meetings.</p>
              </div>
            </div>

            <div class="faq-item">
              <button class="faq-question" onclick={() => toggleFaq(1)}>
                <span>How does the free trial work?</span>
                <svg class="faq-icon" class:expanded={expandedFaqIndex === 1} width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <polyline points="6 9 12 15 18 9"/>
                </svg>
              </button>
              <div class="faq-answer" class:expanded={expandedFaqIndex === 1}>
                <p>When you upgrade to Pro, you get full access to all Pro features immediately. Your subscription renews monthly unless you cancel through the "Manage Subscription" option.</p>
              </div>
            </div>

            <div class="faq-item">
              <button class="faq-question" onclick={() => toggleFaq(2)}>
                <span>Can I cancel anytime?</span>
                <svg class="faq-icon" class:expanded={expandedFaqIndex === 2} width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <polyline points="6 9 12 15 18 9"/>
                </svg>
              </button>
              <div class="faq-answer" class:expanded={expandedFaqIndex === 2}>
                <p>Yes! You can cancel your subscription anytime from the "Manage Subscription" option. You'll continue to have access to Pro features until the end of your current billing period.</p>
              </div>
            </div>

            <div class="faq-item">
              <button class="faq-question" onclick={() => toggleFaq(3)}>
                <span>What if I switch devices?</span>
                <svg class="faq-icon" class:expanded={expandedFaqIndex === 3} width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <polyline points="6 9 12 15 18 9"/>
                </svg>
              </button>
              <div class="faq-answer" class:expanded={expandedFaqIndex === 3}>
                <p>Your subscription is tied to your Apple ID or Google Play account. Use the "Restore Purchases" button to sync your subscription to a new device.</p>
              </div>
            </div>

            <div class="faq-item">
              <button class="faq-question" onclick={() => toggleFaq(4)}>
                <span>Is payment secure?</span>
                <svg class="faq-icon" class:expanded={expandedFaqIndex === 4} width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <polyline points="6 9 12 15 18 9"/>
                </svg>
              </button>
              <div class="faq-answer" class:expanded={expandedFaqIndex === 4}>
                <p>Absolutely. All payments are processed through Apple App Store or Google Play, which use industry-standard encryption and security practices. We never see or store your payment information.</p>
              </div>
            </div>
          </div>
        </div>
      </div>

    {:else}
      <div class="settings-section">
        <!-- Hero Section -->
        <div class="about-hero">
          <div class="about-logo-container">
            <div class="about-logo">
              <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="var(--accent)" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                <path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z"/>
                <path d="M19 10v2a7 7 0 0 1-14 0v-2"/>
                <line x1="12" y1="19" x2="12" y2="23"/>
                <line x1="8" y1="23" x2="16" y2="23"/>
              </svg>
            </div>
          </div>
          <h1 class="about-title">Auralis</h1>
          <p class="about-subtitle">Real-time speech translation and meeting summaries</p>
          <div class="about-version">Version {appVersion}</div>
        </div>

        <!-- Features Grid -->
        <div class="about-features">
          <div class="feature-item">
            <span class="feature-emoji">🎙️</span>
            <div class="feature-content">
              <h3 class="feature-title">Real-time Translation</h3>
              <p class="feature-desc">Instant speech-to-speech translation with support for 100+ languages</p>
            </div>
          </div>

          <div class="feature-item">
            <span class="feature-emoji">📝</span>
            <div class="feature-content">
              <h3 class="feature-title">AI Summaries</h3>
              <p class="feature-desc">Automatic meeting summaries powered by advanced AI</p>
            </div>
          </div>

          <div class="feature-item">
            <span class="feature-emoji">💻</span>
            <div class="feature-content">
              <h3 class="feature-title">Offline Mode</h3>
              <p class="feature-desc">Works without internet using local ML models on Apple Silicon</p>
            </div>
          </div>

          <div class="feature-item">
            <span class="feature-emoji">🔒</span>
            <div class="feature-content">
              <h3 class="feature-title">Privacy First</h3>
              <p class="feature-desc">Your audio data stays on your device in offline mode</p>
            </div>
          </div>
        </div>

        <!-- Update Section -->
        <div class="about-section">
          <div class="section-header">
            <h2 class="section-title">Updates</h2>
          </div>
          <div class="update-card">
            <div class="update-info">
              {#if updateStatus === 'idle'}
                <div class="update-status-icon">
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--text-dim)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <circle cx="12" cy="12" r="10"/>
                    <polyline points="12 6 12 12 16 14"/>
                  </svg>
                </div>
                <div class="update-text">
                  <span class="update-label">Last checked: never</span>
                </div>
              {:else if updateStatus === 'checking'}
                <div class="update-spinner"></div>
                <div class="update-text">
                  <span class="update-label">Checking for updates...</span>
                </div>
              {:else if updateStatus === 'up-to-date'}
                <div class="update-status-icon update-success">
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--success)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/>
                    <polyline points="22 4 12 14.01 9 11.01"/>
                  </svg>
                </div>
                <div class="update-text">
                  <span class="update-label" style="color: var(--success);">You're up to date</span>
                  <span class="update-sub">v{appVersion} is the latest version</span>
                </div>
              {:else if updateStatus === 'available'}
                <div class="update-status-icon update-available">
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--accent)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
                    <polyline points="7 10 12 15 17 10"/>
                    <line x1="12" y1="15" x2="12" y2="3"/>
                  </svg>
                </div>
                <div class="update-text">
                  <span class="update-label" style="color: var(--accent);">Update available</span>
                  <span class="update-sub">v{latestVersion} is available</span>
                </div>
                <Button onclick={downloadAndInstallUpdate} size="sm">
                  Update Now
                </Button>
              {:else if updateStatus === 'downloading'}
                <div class="update-spinner"></div>
                <div class="update-text">
                  <span class="update-label" style="color: var(--accent);">Downloading update...</span>
                  <span class="update-sub">{updateProgress}%</span>
                </div>
              {:else}
                <div class="update-status-icon update-error">
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--danger)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <circle cx="12" cy="12" r="10"/>
                    <line x1="15" y1="9" x2="9" y2="15"/>
                    <line x1="9" y1="9" x2="15" y2="15"/>
                  </svg>
                </div>
                <div class="update-text">
                  <span class="update-label" style="color: var(--danger);">Check failed</span>
                  <span class="update-sub">Could not reach update server</span>
                </div>
              {/if}
            </div>
            {#if updateStatus !== 'checking' && updateStatus !== 'downloading'}
              <button class="btn-icon update-refresh" onclick={checkForUpdates} title="Check for updates">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <polyline points="23 4 23 10 17 10"/>
                  <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/>
                </svg>
              </button>
            {/if}
          </div>
        </div>

        <!-- Changelog Section -->
        <div class="about-section">
          <div class="section-header">
            <h2 class="section-title">What's New</h2>
          </div>
          <div class="changelog-list">
            <div class="changelog-item">
              <div class="changelog-version">
                <span class="version-tag">v0.1.0</span>
                <span class="version-date">Initial Release</span>
              </div>
              <ul class="changelog-features">
                <li>Real-time speech translation with 100+ language support</li>
                <li>AI-powered meeting summaries with key points and action items</li>
                <li>Offline mode using local ML models on Apple Silicon</li>
                <li>Two-way translation with automatic language detection</li>
                <li>Text-to-speech with multiple voice options</li>
                <li>Subscription tiers (Free and Pro) with RevenueCat integration</li>
                <li>Auto-update support for seamless upgrades</li>
                <li>Saved transcripts with search and management</li>
              </ul>
            </div>
          </div>
        </div>

        <!-- Keyboard Shortcuts Section -->
        <div class="about-section">
          <div class="section-header">
            <h2 class="section-title">Keyboard Shortcuts</h2>
          </div>
          <div class="shortcuts-grid">
            <div class="shortcut-category">
              <h3 class="shortcut-category-title">Translation Controls</h3>
              <div class="shortcut-list">
                <div class="shortcut-item">
                  <div class="shortcut-keys">
                    <kbd class="key">Space</kbd>
                  </div>
                  <span class="shortcut-desc">Start/Stop translation</span>
                </div>
                <div class="shortcut-item">
                  <div class="shortcut-keys">
                    <kbd class="key">⌘</kbd><span class="key-plus">+</span><kbd class="key">,</kbd>
                  </div>
                  <span class="shortcut-desc">Open Settings</span>
                </div>
                <div class="shortcut-item">
                  <div class="shortcut-keys">
                    <kbd class="key">⌘</kbd><span class="key-plus">+</span><kbd class="key">H</kbd>
                  </div>
                  <span class="shortcut-desc">Show/Hide transcript overlay</span>
                </div>
              </div>
            </div>

            <div class="shortcut-category">
              <h3 class="shortcut-category-title">Transcript Management</h3>
              <div class="shortcut-list">
                <div class="shortcut-item">
                  <div class="shortcut-keys">
                    <kbd class="key">⌘</kbd><span class="key-plus">+</span><kbd class="key">S</kbd>
                  </div>
                  <span class="shortcut-desc">Save current transcript</span>
                </div>
                <div class="shortcut-item">
                  <div class="shortcut-keys">
                    <kbd class="key">⌘</kbd><span class="key-plus">+</span><kbd class="key">D</kbd>
                  </div>
                  <span class="shortcut-desc">Clear transcript</span>
                </div>
                <div class="shortcut-item">
                  <div class="shortcut-keys">
                    <kbd class="key">⌘</kbd><span class="key-plus">+</span><kbd class="key">T</kbd>
                  </div>
                  <span class="shortcut-desc">Open saved transcripts</span>
                </div>
              </div>
            </div>

            <div class="shortcut-category">
              <h3 class="shortcut-category-title">Window Controls</h3>
              <div class="shortcut-list">
                <div class="shortcut-item">
                  <div class="shortcut-keys">
                    <kbd class="key">⌘</kbd><span class="key-plus">+</span><kbd class="key">M</kbd>
                  </div>
                  <span class="shortcut-desc">Minimize window</span>
                </div>
                <div class="shortcut-item">
                  <div class="shortcut-keys">
                    <kbd class="key">⌘</kbd><span class="key-plus">+</span><kbd class="key">W</kbd>
                  </div>
                  <span class="shortcut-desc">Close window</span>
                </div>
                <div class="shortcut-item">
                  <div class="shortcut-keys">
                    <kbd class="key">⌘</kbd><span class="key-plus">+</span><kbd class="key">P</kbd>
                  </div>
                  <span class="shortcut-desc">Pin/Unpin window</span>
                </div>
              </div>
            </div>
          </div>
          <p class="shortcuts-note">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--text-dim)" stroke-width="2" style="vertical-align: -2px; margin-right: 4px;">
              <circle cx="12" cy="12" r="10"/>
              <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/>
              <line x1="12" y1="17" x2="12.01" y2="17"/>
            </svg>
            Shortcuts use <kbd class="key-inline">⌘</kbd> (Command) on macOS or <kbd class="key-inline">Ctrl</kbd> on Windows/Linux
          </p>
        </div>

        <!-- Resources Section -->
        <div class="about-section">
          <div class="section-header">
            <h2 class="section-title">Resources</h2>
          </div>
          <div class="links-grid">
            <a href="https://github.com/nghiavan0610/auralis" target="_blank" rel="noopener noreferrer" class="link-card">
              <div class="link-icon">
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="var(--accent)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M9 19c-5 1.5-5-2.5-7-3m14 6v-3.87a3.37 3.37 0 0 0-.94-2.61c3.14-.35 6.44-1.54 6.44-7A5.44 5.44 0 0 0 20 4.77 5.07 5.07 0 0 0 19.91 1S18.73.65 16 2.48a13.38 13.38 0 0 0-7 0C6.27.65 5.09 1 5.09 1A5.07 5.07 0 0 0 5 4.77a5.44 5.44 0 0 0-1.5 3.78c0 5.42 3.3 6.61 6.44 7A3.37 3.37 0 0 0 9 18.13V22"/>
                </svg>
              </div>
              <div class="link-content">
                <div class="link-title">GitHub Repository</div>
                <div class="link-desc">Source code and contributions</div>
              </div>
              <div class="link-arrow">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <line x1="5" y1="12" x2="19" y2="12"/>
                  <polyline points="12 5 19 12 12 19"/>
                </svg>
              </div>
            </a>

            <a href="https://sentialab.com" target="_blank" rel="noopener noreferrer" class="link-card">
              <div class="link-icon">
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="var(--accent)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <circle cx="12" cy="12" r="10"/>
                  <line x1="2" y1="12" x2="22" y2="12"/>
                  <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/>
                </svg>
              </div>
              <div class="link-content">
                <div class="link-title">Website</div>
                <div class="link-desc">Learn more about Sentia Lab</div>
              </div>
              <div class="link-arrow">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <line x1="5" y1="12" x2="19" y2="12"/>
                  <polyline points="12 5 19 12 12 19"/>
                </svg>
              </div>
            </a>

            <a href="https://github.com/nghiavan0610/auralis/blob/main/PRIVACY.md" target="_blank" rel="noopener noreferrer" class="link-card">
              <div class="link-icon">
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="var(--accent)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
                </svg>
              </div>
              <div class="link-content">
                <div class="link-title">Privacy Policy</div>
                <div class="link-desc">How we handle your data</div>
              </div>
              <div class="link-arrow">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <line x1="5" y1="12" x2="19" y2="12"/>
                  <polyline points="12 5 19 12 12 19"/>
                </svg>
              </div>
            </a>
          </div>
        </div>

        <!-- Legal & Community Section -->
        <div class="about-section">
          <div class="section-header">
            <h2 class="section-title">Legal & Community</h2>
          </div>
          <div class="info-grid">
            <div class="info-item">
              <div class="info-label">License</div>
              <div class="info-value">MIT License</div>
            </div>
            <div class="info-item">
              <div class="info-label">Publisher</div>
              <div class="info-value">Sentia Lab</div>
            </div>
            <div class="info-item">
              <div class="info-label">Support</div>
              <div class="info-value">
                <a href="https://github.com/nghiavan0610/auralis/issues" target="_blank" rel="noopener noreferrer" class="info-link">
                  Report issues
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <line x1="5" y1="12" x2="19" y2="12"/>
                    <polyline points="12 5 19 12 12 19"/>
                  </svg>
                </a>
              </div>
            </div>
            <div class="info-item">
              <div class="info-label">Feedback</div>
              <div class="info-value">
                <a href="https://github.com/nghiavan0610/auralis/issues" target="_blank" rel="noopener noreferrer" class="info-link">
                  Feature requests
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <line x1="5" y1="12" x2="19" y2="12"/>
                    <polyline points="12 5 19 12 12 19"/>
                  </svg>
                </a>
              </div>
            </div>
          </div>
        </div>

        <!-- Footer -->
        <div class="about-footer">
          <div class="footer-content">
            <p class="footer-text">Made with care by <strong>Sentia Lab</strong></p>
            <p class="footer-copyright">© 2025 Sentia Lab. All rights reserved.</p>
          </div>
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .settings-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    animation: fadeIn 0.15s ease;
  }

  /* Header */
  .settings-header {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    height: 42px;
    padding: 0 var(--space-sm);
    flex-shrink: 0;
    user-select: none;
    -webkit-user-select: none;
  }

  .settings-title {
    font-size: var(--font-size-base);
    font-weight: 600;
    color: var(--text-primary);
  }

  /* Tabs — pill style */
  .tabs {
    display: flex;
    gap: 4px;
    padding: var(--space-sm) var(--space-lg);
    flex-shrink: 0;
  }

  .tab {
    padding: 6px 14px;
    background: transparent;
    border: none;
    border-radius: 20px;
    color: var(--text-secondary);
    font-size: var(--font-size-sm);
    font-weight: 500;
    font-family: var(--font-family);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .tab:hover {
    color: var(--text-primary);
    background: var(--bg-secondary);
  }

  .tab.active {
    color: white;
    background: var(--accent);
  }

  /* Tab content */
  .tab-content {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-md) var(--space-lg) var(--space-lg);
  }

  .settings-section {
    display: flex;
    flex-direction: column;
    gap: var(--space-lg);
  }

  .section-label {
    font-size: var(--font-size-xs);
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .section-desc {
    font-size: var(--font-size-xs);
    color: var(--text-dim);
    margin: 0;
    margin-top: -6px;
    line-height: 1.5;
  }

  /* Mode cards — rounder */
  .mode-card {
    cursor: pointer;
    min-width: 0; /* Allow flex items to shrink properly */
  }

  .mode-card.disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  .mode-card.disabled .mode-card-content {
    border-color: var(--border);
    background: transparent;
  }

  .mode-card input[type="radio"] {
    display: none;
  }

  .mode-card-content {
    padding: var(--space-sm) var(--space-md);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    display: flex;
    flex-direction: column;
    gap: 2px;
    transition: all 0.2s ease;
    align-items: flex-start;
  }

  .mode-card:hover .mode-card-content {
    border-color: var(--border-hover);
    background: var(--bg-secondary);
  }

  .mode-card.active .mode-card-content {
    border-color: var(--accent);
    background: var(--accent-dim);
  }

  .mode-name {
    font-size: var(--font-size-base);
    font-weight: 600;
    color: var(--text-primary);
  }

  .mode-desc {
    font-size: var(--font-size-xs);
    color: var(--text-dim);
  }

  .direction-cards {
    display: flex;
    flex-wrap: nowrap;
    gap: var(--space-sm);
    align-items: stretch;
  }

  .direction-cards .mode-card {
    flex: 1 1 auto;
    min-width: 0;
    max-width: 400px;
  }

  .source-cards {
    display: flex;
    gap: var(--space-sm);
  }

  .source-card {
    flex: 1;
    min-width: 0;
  }

  /* Quick Presets */
  .presets-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: var(--space-sm);
    margin-bottom: var(--space-lg);
  }

  .preset-card {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-md);
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all 0.2s ease;
    text-align: left;
  }

  .preset-card:hover:not(:disabled) {
    background: rgba(99, 140, 255, 0.05);
    border-color: rgba(99, 140, 255, 0.3);
  }

  .preset-card:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .preset-card.active {
    background: rgba(99, 140, 255, 0.1);
    border-color: var(--accent);
  }

  .preset-icon {
    font-size: 20px;
    flex-shrink: 0;
  }

  .preset-content {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1;
  }

  .preset-name {
    font-weight: 600;
    font-size: var(--font-size-sm);
    color: var(--text-primary);
  }

  .preset-desc {
    font-size: var(--font-size-xs);
    color: var(--text-dim);
    line-height: 1.3;
  }

  .preset-arrow {
    flex-shrink: 0;
    color: var(--text-dim);
    transition: transform 0.2s ease;
  }

  .preset-card:hover .preset-arrow {
    transform: translateX(2px);
  }

  /* Fields — rounder inputs */
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }

  .field-desc {
    font-size: var(--font-size-xs);
    color: var(--text-dim);
    margin: 0;
    line-height: 1.4;
  }

  .field-error {
    font-size: var(--font-size-xs);
    color: var(--danger);
    margin: 0;
    line-height: 1.4;
    font-weight: 500;
  }

  .label-row {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }

  .badge-required {
    font-size: 0.6rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: #1a1b2e;
    background: var(--warning);
    padding: 1px 6px;
    border-radius: 4px;
    line-height: 1.5;
  }

  .badge-optional {
    font-size: 0.6rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-dim);
    background: var(--bg-secondary);
    padding: 1px 6px;
    border-radius: 4px;
    line-height: 1.5;
  }

  .input-error {
    border-color: var(--danger) !important;
    box-shadow: 0 0 0 3px rgba(255, 77, 77, 0.12) !important;
    animation: shake 0.3s ease;
  }

  @keyframes shake {
    0%, 100% { transform: translateX(0); }
    20%, 60% { transform: translateX(-4px); }
    40%, 80% { transform: translateX(4px); }
  }

  .field-link {
    font-size: var(--font-size-xs);
    align-self: flex-start;
  }

  /* Language row */
  .language-row {
    display: flex;
    align-items: flex-end;
    gap: var(--space-sm);
  }

  .lang-arrow {
    color: var(--text-dim);
    padding-bottom: var(--space-xs);
  }

  /* About Hero Section */
  .about-hero {
    text-align: center;
    padding: var(--space-xl) var(--space-md) var(--space-lg);
    border-radius: var(--radius-lg);
    margin-bottom: var(--space-lg);
  }

  .about-logo-container {
    display: flex;
    justify-content: center;
    margin-bottom: var(--space-md);
  }

  .about-logo {
    width: 80px;
    height: 80px;
    border-radius: 20px;
    background: linear-gradient(135deg, rgba(99, 102, 241, 0.1) 0%, rgba(139, 92, 246, 0.1) 100%);
    border: 2px solid rgba(99, 102, 241, 0.2);
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 8px 24px rgba(99, 102, 241, 0.15);
    transition: transform 0.3s ease, box-shadow 0.3s ease;
  }

  .about-logo:hover {
    transform: translateY(-2px);
    box-shadow: 0 12px 32px rgba(99, 102, 241, 0.2);
  }

  .about-title {
    font-size: 28px;
    font-weight: 700;
    color: var(--text-primary);
    margin: 0 0 var(--space-xs) 0;
    letter-spacing: -0.5px;
  }

  .about-subtitle {
    font-size: var(--font-size-base);
    color: var(--text-secondary);
    margin: 0 0 var(--space-sm) 0;
    line-height: 1.5;
  }

  .about-version {
    display: inline-block;
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--accent);
    background: rgba(99, 102, 241, 0.1);
    padding: 4px 12px;
    border-radius: 12px;
    margin-top: var(--space-sm);
  }

  /* Features Grid */
  .about-features {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
    margin-bottom: var(--space-lg);
    background: transparent !important;
    border: none !important;
    box-shadow: none !important;
  }

  .feature-item {
    display: flex;
    align-items: flex-start;
    gap: var(--space-sm);
    background: transparent !important;
    border: none !important;
    box-shadow: none !important;
    padding: 0 !important;
  }

  .feature-item svg {
    flex-shrink: 0;
    width: 20px;
    height: 20px;
    background: transparent !important;
    border: none !important;
    outline: none !important;
    box-shadow: none !important;
    display: block;
  }

  .feature-emoji {
    font-size: 20px;
    flex-shrink: 0;
    display: block;
  }

  .feature-content {
    flex: 1;
    min-width: 0;
  }

  .feature-title {
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 4px 0;
  }

  .feature-desc {
    font-size: var(--font-size-xs);
    color: var(--text-dim);
    margin: 0;
    line-height: 1.4;
  }

  /* About Sections */
  .about-section {
    margin-bottom: var(--space-lg);
  }

  .section-header {
    margin-bottom: var(--space-md);
  }

  .section-title {
    font-size: var(--font-size-lg);
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  /* Changelog */
  .changelog-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }

  .changelog-item {
    padding: var(--space-md);
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }

  .changelog-version {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    margin-bottom: var(--space-sm);
  }

  .version-tag {
    padding: 4px 10px;
    background: rgba(99, 140, 255, 0.15);
    color: var(--accent);
    font-size: var(--font-size-xs);
    font-weight: 600;
    border-radius: var(--radius-sm);
  }

  .version-date {
    font-size: var(--font-size-xs);
    color: var(--text-dim);
  }

  .changelog-features {
    margin: 0;
    padding-left: var(--space-md);
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }

  .changelog-features li {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    line-height: 1.5;
  }

  /* Keyboard Shortcuts */
  .shortcuts-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: var(--space-md);
    margin-bottom: var(--space-md);
  }

  .shortcut-category {
    padding: var(--space-md);
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }

  .shortcut-category-title {
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 var(--space-sm) 0;
  }

  .shortcut-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }

  .shortcut-item {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }

  .shortcut-keys {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  .key {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 24px;
    height: 24px;
    padding: 0 6px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    font-size: 11px;
    font-weight: 500;
    color: var(--text-primary);
    font-family: var(--font-family);
  }

  .key-plus {
    color: var(--text-dim);
    font-size: 12px;
  }

  .key-inline {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 20px;
    height: 20px;
    padding: 0 4px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 3px;
    font-size: 10px;
    font-weight: 500;
    color: var(--text-primary);
    font-family: var(--font-family);
  }

  .shortcut-desc {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
  }

  .shortcuts-note {
    display: flex;
    align-items: center;
    padding: var(--space-sm) var(--space-md);
    background: rgba(99, 140, 255, 0.05);
    border: 1px solid rgba(99, 140, 255, 0.1);
    border-radius: var(--radius-md);
    font-size: var(--font-size-xs);
    color: var(--text-dim);
    margin: 0;
  }

  /* Links Grid */
  .links-grid {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }

  .link-card {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-md);
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    text-decoration: none;
    transition: all 0.2s ease;
  }

  .link-card:hover {
    border-color: var(--accent);
    transform: translateX(4px);
    box-shadow: var(--shadow-sm);
  }

  .link-icon {
    width: 40px;
    height: 40px;
    border-radius: 10px;
    background: rgba(99, 102, 241, 0.1);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .link-content {
    flex: 1;
    min-width: 0;
  }

  .link-title {
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 2px;
  }

  .link-desc {
    font-size: var(--font-size-xs);
    color: var(--text-dim);
  }

  .link-arrow {
    color: var(--text-dim);
    transition: color 0.2s ease;
  }

  .link-card:hover .link-arrow {
    color: var(--accent);
  }

  /* Info Grid */
  .info-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: var(--space-sm);
  }

  .info-item {
    padding: var(--space-md);
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }

  .info-label {
    font-size: var(--font-size-xs);
    color: var(--text-dim);
    margin-bottom: 4px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .info-value {
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--text-primary);
  }

  .info-link {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--accent);
    text-decoration: none;
    font-weight: 500;
    transition: all 0.2s ease;
  }

  .info-link:hover {
    color: var(--accent-hover);
    text-decoration: underline;
  }

  /* Footer Enhancement */
  .about-footer {
    text-align: center;
    padding: var(--space-lg) var(--space-md) var(--space-md);
    border-top: 1px solid var(--border);
    margin-top: var(--space-lg);
  }

  .footer-content {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    align-items: center;
  }

  .footer-text {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
    margin: 0;
  }

  .footer-text strong {
    color: var(--text-primary);
    font-weight: 600;
  }

  .footer-copyright {
    font-size: var(--font-size-xs);
    color: var(--text-dim);
    margin: 0;
  }

  /* Update section */
  .update-section {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }

  .update-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-md);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--bg-secondary);
  }

  .update-info {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }

  .update-status-icon {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    border-radius: 50%;
    background: var(--bg-primary);
  }

  .update-text {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .update-label {
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-primary);
  }

  .update-sub {
    font-size: var(--font-size-xs);
    color: var(--text-dim);
  }

  .update-refresh {
    color: var(--text-dim);
  }

  .update-refresh:hover {
    color: var(--text-primary);
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  .update-spinner {
    width: 16px;
    height: 16px;
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    flex-shrink: 0;
    margin: 0 4px;
  }

  /* Slider cards */
  .slider-card {
    padding: var(--space-md);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--bg-secondary);
    display: flex;
    flex-direction: column;
    gap: 10px;
    transition: border-color 0.2s ease;
  }

  .slider-card:hover {
    border-color: var(--border-hover);
  }

  .slider-card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .slider-card-label {
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-primary);
  }

  .slider-card-value {
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--accent);
    font-variant-numeric: tabular-nums;
    min-width: 40px;
    text-align: right;
  }

  .slider-card-desc {
    font-size: var(--font-size-xs);
    color: var(--text-dim);
    margin: 0;
    line-height: 1.4;
  }

  /* Display Preview */
  .display-preview-section {
    margin-bottom: var(--space-lg);
  }

  .preview-header {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    margin-bottom: var(--space-sm);
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--accent);
  }

  .preview-overlay {
    padding: var(--space-md) var(--space-lg);
    border-radius: var(--radius-md);
    border: 1px solid var(--border);
    background: var(--bg-secondary);
    min-height: 120px;
    transition: all 0.2s ease;
  }

  .preview-transcript {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }

  .preview-entry {
    margin: 0;
    line-height: 1.6;
    color: var(--text-primary);
  }

  /* Slider track - Modern design */
  .slider {
    width: 100%;
    -webkit-appearance: none;
    appearance: none;
    height: 6px;
    border-radius: 3px;
    outline: none;
    cursor: pointer;
    background: linear-gradient(to right, var(--accent) 0%, var(--accent) var(--fill, 50%), rgba(255,255,255,0.12) var(--fill, 50%), rgba(255,255,255,0.12) 100%);
    transition: all 0.2s ease;
    border: none;
  }

  .slider:hover {
    height: 6px;
    background: linear-gradient(to right, var(--accent) 0%, var(--accent) var(--fill, 50%), rgba(255,255,255,0.15) var(--fill, 50%), rgba(255,255,255,0.15) 100%);
  }

  .slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: white;
    cursor: pointer;
    border: none;
    box-shadow:
      0 2px 8px rgba(0, 0, 0, 0.3),
      0 0 0 1px rgba(255, 255, 255, 0.1);
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .slider::-webkit-slider-thumb:hover {
    transform: scale(1.1);
    box-shadow:
      0 2px 12px rgba(99, 140, 255, 0.5),
      0 0 0 1px rgba(255, 255, 255, 0.2);
  }

  .slider::-webkit-slider-thumb:active {
    transform: scale(1.05);
    box-shadow:
      0 2px 8px rgba(99, 140, 255, 0.6),
      0 0 0 2px rgba(99, 140, 255, 0.2);
  }

  /* Firefox slider support */
  .slider::-moz-range-thumb {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: white;
    cursor: pointer;
    border: none;
    box-shadow:
      0 2px 8px rgba(0, 0, 0, 0.3),
      0 0 0 1px rgba(255, 255, 255, 0.1);
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .slider::-moz-range-thumb:hover {
    transform: scale(1.1);
    box-shadow:
      0 2px 12px rgba(99, 140, 255, 0.5),
      0 0 0 1px rgba(255, 255, 255, 0.2);
  }

  .slider::-moz-range-thumb:active {
    transform: scale(1.05);
    box-shadow:
      0 2px 8px rgba(99, 140, 255, 0.6),
      0 0 0 2px rgba(99, 140, 255, 0.2);
  }

  .slider::-moz-range-track {
    height: 6px;
    border-radius: 3px;
    background: linear-gradient(to right, var(--accent) 0%, var(--accent) var(--fill, 50%), rgba(255,255,255,0.12) var(--fill, 50%), rgba(255,255,255,0.12) 100%);
  }

  /* Toggle switch */
  .toggle-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-md);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    background: var(--bg-secondary);
    transition: border-color 0.2s ease;
  }

  .toggle-card:hover {
    border-color: var(--border-hover);
  }

  .toggle-card-label {
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-primary);
  }

  .toggle {
    width: 44px;
    height: 24px;
    border-radius: 12px;
    border: none;
    background: rgba(255, 255, 255, 0.1);
    position: relative;
    cursor: pointer;
    transition: background 0.2s ease;
    flex-shrink: 0;
  }

  .toggle.active {
    background: var(--accent);
  }

  .toggle-thumb {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: white;
    transition: transform 0.2s ease;
    pointer-events: none;
  }

  .toggle.active .toggle-thumb {
    transform: translateX(20px);
  }

  .toggle:disabled {
    opacity: 0.4;
    cursor: default;
  }

  /* Voice dropdown */
  .voice-dropdown-container {
    position: relative;
    z-index: var(--z-modal);
  }

  .voice-dropdown-container.open {
    z-index: var(--z-dropdown);
  }

  .voice-trigger {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    border-radius: 12px;
    border: 1px solid var(--border);
    background: var(--bg-solid);
    color: var(--text-primary);
    font-size: var(--font-size-sm);
    font-family: var(--font-family);
    cursor: pointer;
    outline: none;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    position: relative;
  }

  .voice-trigger:hover {
    border-color: var(--border-hover);
    background: rgba(255, 255, 255, 0.02);
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
  }

  .voice-trigger:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px rgba(99, 140, 255, 0.15), 0 4px 12px rgba(0, 0, 0, 0.2);
  }

  .voice-trigger:active {
    transform: translateY(0);
  }

  .voice-trigger:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .voice-trigger-text {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .voice-trigger-flag {
    font-size: 16px;
  }

  .voice-trigger-placeholder {
    color: var(--text-dim);
  }

  .voice-trigger-arrow {
    flex-shrink: 0;
    color: var(--text-dim);
    transition: transform 0.2s ease;
  }

  .voice-trigger-arrow.open {
    transform: rotate(180deg);
  }

  /* Enhanced open state for trigger */
  .voice-dropdown-container.open .voice-trigger {
    border-color: var(--accent);
    background: rgba(99, 140, 255, 0.05);
    box-shadow: 0 0 0 3px rgba(99, 140, 255, 0.1);
  }

  .voice-dropdown {
    position: fixed;
    max-height: 280px;
    overflow-y: auto;
    border-radius: var(--radius-lg);

    /* Enhanced border and shadows using CSS variables */
    border: 1px solid var(--border);
    box-shadow:
      var(--shadow-lg),
      0 0 0 1px rgba(255, 255, 255, 0.05),
      inset 0 1px 0 rgba(255, 255, 255, 0.1);

    /* Glassmorphism background using CSS variables */
    background: var(--bg-solid);
    backdrop-filter: blur(24px) saturate(1.2);
    -webkit-backdrop-filter: blur(24px) saturate(1.2);

    /* Enhanced spacing */
    padding: 6px;
    z-index: var(--z-dropdown) !important;

    /* Improved animation */
    animation: dropdownSlideIn 0.15s cubic-bezier(0.4, 0, 0.2, 1);

    /* Enhanced scrollbar */
    scrollbar-width: thin;
    scrollbar-color: rgba(255, 255, 255, 0.2) transparent;
  }

  /* WebKit scrollbar enhancement */
  .voice-dropdown::-webkit-scrollbar {
    width: 6px;
  }

  .voice-dropdown::-webkit-scrollbar-track {
    background: rgba(255, 255, 255, 0.02);
    border-radius: 3px;
  }

  .voice-dropdown::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.15);
    border-radius: 3px;
  }

  .voice-dropdown::-webkit-scrollbar-thumb:hover {
    background: rgba(255, 255, 255, 0.25);
  }

  @keyframes dropdownSlideIn {
    from {
      opacity: 0;
      transform: translateY(-8px) scale(0.98);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  .voice-option {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 10px 12px;
    border-radius: 8px;
    background: transparent;
    color: var(--text-primary);
    font-size: var(--font-size-sm);
    font-family: var(--font-family);
    cursor: pointer;
    text-align: left;
    transition: all 0.15s cubic-bezier(0.4, 0, 0.2, 1);
    position: relative;
    border: 1px solid transparent;
  }

  .voice-option:hover {
    background: rgba(255, 255, 255, 0.08);
    border-color: rgba(255, 255, 255, 0.1);
    transform: translateX(2px);
  }

  .voice-option:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px rgba(99, 140, 255, 0.15);
    z-index: var(--z-base);
  }

  .voice-option.active {
    background: var(--accent-dim);
    border-color: rgba(99, 140, 255, 0.2);
  }

  .voice-option.active:hover {
    background: rgba(99, 140, 255, 0.2);
  }

  .voice-option.disabled {
    opacity: 0.5;
    cursor: not-allowed;
    pointer-events: none;
  }

  .voice-option.disabled:hover {
    background: transparent;
    transform: none;
  }

  .voice-option-check {
    width: 18px;
    height: 18px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    color: var(--accent);
    font-weight: 600;
    background: rgba(99, 140, 255, 0.1);
    border-radius: 4px;
    transition: all 0.15s ease;
  }

  .voice-option:hover .voice-option-check {
    background: rgba(99, 140, 255, 0.15);
    transform: scale(1.1);
  }

  .voice-option.active .voice-option-check {
    background: var(--accent);
    color: white;
  }

  .voice-option-content {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
    flex: 1;
  }

  .voice-option-name {
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text-primary);
    transition: color 0.15s ease;
  }

  .voice-option-desc {
    font-size: var(--font-size-xs);
    color: var(--text-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    line-height: 1.4;
    transition: color 0.15s ease;
  }

  .voice-option:hover .voice-option-name {
    color: var(--text-primary);
  }

  .voice-option:hover .voice-option-desc {
    color: var(--text-secondary);
  }

  .voice-option.active .voice-option-name {
    color: var(--accent);
  }

  .voice-option.active .voice-option-desc {
    color: rgba(99, 140, 255, 0.7);
  }

  .voice-option-main {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }

  .voice-preview-btn {
    flex-shrink: 0;
    width: 20px;
    height: 20px;
    padding: 2px;
    border: none;
    background: transparent;
    border-radius: 4px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.2s ease;
  }

  .voice-preview-btn:hover:not(:disabled) {
    background: rgba(99, 140, 255, 0.1);
  }

  .voice-preview-btn:disabled {
    cursor: default;
    opacity: 0.7;
  }

  .previewing-icon {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
      stroke-dashoffset: 32;
    }
    to {
      transform: rotate(360deg);
      stroke-dashoffset: 0;
    }
  }

  .voice-option-flag {
    margin-left: auto;
    font-size: 16px;
    flex-shrink: 0;
  }

  /* Plan Status Banner - Main visual indicator */
  .plan-status-banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-lg);
    border-radius: var(--radius-lg);
    margin-bottom: var(--space-lg);
    border: 2px solid;
    transition: all var(--transition-normal);
  }

  .plan-status-banner.free-plan {
    background: linear-gradient(135deg, rgba(107, 114, 128, 0.1) 0%, rgba(107, 114, 128, 0.05) 100%);
    border-color: var(--text-secondary);
    box-shadow: 0 4px 12px rgba(107, 114, 128, 0.15);
  }

  .plan-status-banner.pro-plan {
    background: linear-gradient(135deg, rgba(99, 102, 241, 0.2) 0%, rgba(139, 92, 246, 0.1) 100%);
    border-color: var(--accent);
    box-shadow: 0 4px 16px rgba(99, 102, 241, 0.25);
  }

  .plan-status-content {
    display: flex;
    align-items: center;
    gap: var(--space-md);
  }

  .plan-status-icon {
    width: 48px;
    height: 48px;
    border-radius: var(--radius-md);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .free-plan .plan-status-icon {
    background: rgba(107, 114, 128, 0.15);
    color: var(--text-secondary);
  }

  .pro-plan .plan-status-icon {
    background: var(--accent);
    color: white;
  }

  .plan-status-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .plan-status-title {
    font-size: var(--font-size-xl);
    font-weight: 700;
    letter-spacing: 0.5px;
    color: var(--text-primary);
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }

  .pro-plan .plan-status-title {
    color: var(--accent);
  }

  .trial-badge {
    font-size: var(--font-size-xs);
    font-weight: 600;
    padding: 2px 8px;
    border-radius: 10px;
    background: rgba(99, 102, 241, 0.2);
    color: var(--accent);
    border: 1px solid rgba(99, 102, 241, 0.3);
  }

  .plan-status-subtitle {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
  }

  .plan-status-action {
    flex-shrink: 0;
  }

  .banner-upgrade-btn {
    padding: 10px 20px;
    background: var(--accent);
    color: white;
    font-size: var(--font-size-sm);
    font-weight: 600;
    border: none;
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all var(--transition-fast);
    box-shadow: 0 2px 8px rgba(99, 102, 241, 0.3);
  }

  .banner-upgrade-btn:hover {
    background: var(--accent-hover);
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(99, 102, 241, 0.4);
  }

  .banner-manage-btn {
    padding: 10px 20px;
    background: transparent;
    color: var(--text-secondary);
    font-size: var(--font-size-sm);
    font-weight: 600;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .banner-manage-btn:hover {
    background: var(--bg-secondary);
    color: var(--text-primary);
    border-color: var(--accent);
  }

  /* Usage Visualization */
  .usage-section {
    margin-bottom: var(--space-lg);
  }

  .usage-visual-card {
    display: flex;
    align-items: center;
    gap: var(--space-lg);
    padding: var(--space-lg);
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
  }

  .usage-progress-container {
    position: relative;
    flex-shrink: 0;
  }

  .usage-progress-ring {
    transform: rotate(-90deg);
  }

  .usage-progress-bg {
    stroke: var(--border);
  }

  .usage-progress-fill {
    stroke: var(--accent);
    stroke-dasharray: 314;
    stroke-dashoffset: calc(314 * (1 - var(--progress)));
    transition: stroke-dashoffset 0.5s ease;
  }

  .usage-progress-text {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    text-align: center;
  }

  .usage-progress-value {
    font-size: 24px;
    font-weight: 700;
    color: var(--text-primary);
    line-height: 1;
  }

  .usage-progress-label {
    font-size: var(--font-size-xs);
    color: var(--text-dim);
    margin-top: 2px;
  }

  .usage-stats {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
    flex: 1;
  }

  .usage-stat {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--space-sm) var(--space-md);
    background: var(--bg-primary);
    border-radius: var(--radius-sm);
  }

  .usage-stat-label {
    font-size: var(--font-size-sm);
    color: var(--text-dim);
  }

  .usage-stat-value {
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--text-primary);
  }

  .usage-stat-value.free-stat {
    color: var(--text-secondary);
  }

  .usage-stat-value.pro-stat {
    color: var(--accent);
  }

  .usage-stat-value.low-remaining {
    color: var(--warning);
  }

  .usage-warning {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-md);
    margin-top: var(--space-md);
    background: rgba(251, 191, 36, 0.1);
    border: 1px solid rgba(251, 191, 36, 0.2);
    border-radius: var(--radius-md);
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
  }

  /* Restore section with inline help */
  .restore-section {
    margin-bottom: var(--space-lg);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-md);
    background: rgba(99, 102, 241, 0.05);
    border: 1px solid rgba(99, 102, 241, 0.1);
    border-radius: var(--radius-md);
  }

  .restore-help {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    font-size: var(--font-size-xs);
    color: var(--text-dim);
    text-align: center;
    line-height: 1.4;
  }

  .restore-help svg {
    flex-shrink: 0;
    opacity: 0.7;
  }

  /* Subscription tier cards */
  .tier-card {
    background: var(--bg-secondary);
    border: 2px solid var(--border);
    border-radius: var(--radius-lg);
    padding: var(--space-lg);
    transition: all var(--transition-normal);
    position: relative;
    overflow: hidden;
  }

  .tier-card:not(.current-tier) {
    opacity: 0.7;
  }

  .tier-card:not(.current-tier):hover {
    opacity: 1;
    transform: translateY(-2px);
    box-shadow: var(--shadow-lg);
  }

  .tier-card.current-tier {
    opacity: 1;
    transform: scale(1.01);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
  }

  .tier-card.current-tier:not(.pro-tier) {
    border-color: var(--text-secondary);
    background: linear-gradient(135deg, var(--bg-secondary) 0%, rgba(107, 114, 128, 0.08) 100%);
  }

  .tier-card.pro-tier.current-tier {
    border-color: var(--accent);
    background: linear-gradient(135deg, rgba(99, 102, 241, 0.1) 0%, rgba(139, 92, 246, 0.05) 100%);
  }

  .tier-card.pro-tier:not(.current-tier) {
    background: linear-gradient(135deg, var(--bg-secondary) 0%, rgba(99, 102, 241, 0.05) 100%);
    border-color: rgba(99, 102, 241, 0.3);
  }

  .tier-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-md);
  }

  .tier-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .tier-name {
    font-size: var(--font-size-lg);
    font-weight: 700;
    color: var(--text-primary);
  }

  .tier-price {
    font-size: var(--font-size-sm);
    color: var(--text-dim);
  }

  .tier-badge {
    font-size: var(--font-size-xs);
    font-weight: 600;
    padding: 4px 12px;
    border-radius: 12px;
  }

  .free-badge {
    background: var(--bg-tertiary);
    color: var(--text-secondary);
  }

  .pro-badge {
    background: var(--accent);
    color: white;
  }

  .tier-features {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }

  .tier-feature {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
  }

  .tier-feature svg {
    flex-shrink: 0;
  }

  .tier-upgrade-btn {
    width: 100%;
    margin-top: var(--space-md);
    padding: 10px 16px;
    background: var(--accent);
    color: white;
    font-size: var(--font-size-sm);
    font-weight: 600;
    border: none;
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: opacity var(--transition-fast);
  }

  .tier-upgrade-btn:hover {
    opacity: 0.9;
  }

  .tier-upgrade-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .tier-upgrade-btn.loading {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-xs);
  }

  .tier-upgrade-btn .spin {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  .tier-card.current-tier {
    border-color: var(--accent);
    background: linear-gradient(135deg, var(--bg-secondary) 0%, rgba(99, 102, 241, 0.05) 100%);
  }

  .tier-card.current-tier .tier-name {
    color: var(--accent);
  }

  .tier-badge.free-badge {
    background: var(--bg-tertiary);
    color: var(--text-secondary);
  }

  .tier-badge.pro-badge {
    background: var(--accent);
    color: white;
  }

  /* Status messages */
  .status-message {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-sm) var(--space-md);
    border-radius: var(--radius-md);
    font-size: var(--font-size-sm);
  }

  .status-message.success {
    background: rgba(34, 197, 94, 0.1);
    border: 1px solid rgba(34, 197, 94, 0.2);
    color: var(--success);
  }

  .status-message.error {
    background: rgba(255, 77, 77, 0.1);
    border: 1px solid rgba(255, 77, 77, 0.2);
    color: var(--danger);
  }

  .status-message span {
    flex: 1;
  }

  .status-dismiss {
    background: none;
    border: none;
    padding: var(--space-xs);
    cursor: pointer;
    color: inherit;
    opacity: 0.7;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .status-dismiss:hover {
    opacity: 1;
  }

  /* Restore button */
  .restore-btn {
    display: flex;
    align-items: center;
    padding: var(--space-xs) var(--space-sm);
    background: transparent;
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    font-size: var(--font-size-sm);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .restore-btn:hover:not(:disabled) {
    background: var(--bg-secondary);
    color: var(--text-primary);
    border-color: var(--accent);
  }

  .restore-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Manage subscription button */
  .tier-manage-btn {
    width: 100%;
    margin-top: var(--space-md);
    padding: 10px 16px;
    background: transparent;
    color: var(--text-secondary);
    font-size: var(--font-size-sm);
    font-weight: 500;
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all var(--transition-fast);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-xs);
  }

  .tier-manage-btn:hover {
    background: var(--bg-secondary);
    color: var(--text-primary);
    border-color: var(--accent);
  }

  /* FAQ Section */
  .faq-section {
    margin-top: var(--space-xl);
    padding-top: var(--space-lg);
    border-top: 1px solid var(--border);
  }

  .faq-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }

  .faq-item {
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    overflow: hidden;
    background: var(--bg-secondary);
  }

  .faq-question {
    width: 100%;
    padding: var(--space-md);
    background: transparent;
    border: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-sm);
    cursor: pointer;
    text-align: left;
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-primary);
    transition: all 0.2s ease;
  }

  .faq-question:hover {
    background: rgba(99, 140, 255, 0.05);
  }

  .faq-icon {
    flex-shrink: 0;
    transition: transform 0.3s ease;
    color: var(--text-dim);
  }

  .faq-icon.expanded {
    transform: rotate(180deg);
  }

  .faq-answer {
    max-height: 0;
    overflow: hidden;
    transition: max-height 0.3s ease, padding 0.3s ease;
  }

  .faq-answer.expanded {
    max-height: 500px;
    padding: 0 var(--space-md) var(--space-md);
  }

  .faq-answer p {
    margin: 0;
    font-size: var(--font-size-sm);
    line-height: 1.5;
    color: var(--text-secondary);
  }
</style>
