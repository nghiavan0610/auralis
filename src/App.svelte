<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { check as checkForUpdate } from '@tauri-apps/plugin-updater';
  import { SonioxClient } from './js/soniox';
  import type { ConnectionStatus } from './js/soniox';
  import type { Segment, OperatingMode, TranslationType, AudioSource } from './types';
  import {
    startPeriodicSync,
    stopPeriodicSync
  } from './js/revenuecat';
  import {
    translationStore,
    loadTranslationSettings,
    displayStore,
    loadDisplaySettings,
    ttsStore,
    loadTTSSettings,
    subscriptionStore,
    loadSubscriptionStatus,
    loadApiKeys
  } from './stores';

  // Platform info from Rust backend
  interface PlatformInfo {
    os: string;
    system_audio_available: boolean;
    offline_mode_available: boolean;
  }
  let platformInfo: PlatformInfo | null = $state(null);
  import ControlBar from './components/ControlBar.svelte';
  import Transcript from './components/Transcript.svelte';
  import SettingsView from './components/SettingsView.svelte';
  import SavedTranscripts from './components/SavedTranscripts.svelte';
  import FirstRunOnboarding from './components/FirstRunOnboarding.svelte';
  import KeyboardShortcuts from './components/KeyboardShortcuts.svelte';
  import QuickLanguageSelector from './components/QuickLanguageSelector.svelte';
  import QuickModeSelector from './components/QuickModeSelector.svelte';
  import QuickTtsSelector from './components/QuickTtsSelector.svelte';
  import { tts } from './js/tts';

  // ---------------------------------------------------------------------------
  // State (using centralized stores)
  // ---------------------------------------------------------------------------

  // Reactive store values - these automatically update when stores change
  let translationSettings = $state(translationStore.get());
  let displaySettings = $state(displayStore.get());
  let ttsSettings = $state(ttsStore.get());
  let subscriptionState = $state(subscriptionStore.get());

  // Subscribe to store changes
  const unsubscribeTranslation = translationStore.subscribe((settings) => {
    translationSettings = settings;
  });

  const unsubscribeDisplay = displayStore.subscribe((settings) => {
    displaySettings = settings;
  });

  const unsubscribeTTS = ttsStore.subscribe((settings) => {
    ttsSettings = settings;
  });

  const unsubscribeSubscription = subscriptionStore.subscribe((state) => {
    subscriptionState = state;
  });

  // App state (local to App component)
  let updateAvailable = $state(false);
  let isTranslating = $state(false);
  let statusMessage = $state('Ready');
  let activeAudioSources = $state([] as AudioSource[]);
  let errorMessage = $state('');
  let sonioxConnectionStatus: ConnectionStatus | null = $state(null);

  // Auto-detection state for one-way translation
  let detectionState = $state<{
    status: 'idle' | 'detecting' | 'detected' | 'uncertain' | 'error';
    detectedLanguage?: string;
  }>({
    status: 'idle'
  });

  // Segment-based transcript model
  let segments: Segment[] = $state([]);
  let segmentIdCounter = 0;
  let provisionalText = $state('');
  let provisionalLang = $state('');

  // UI state
  let currentView: 'main' | 'settings' | 'saved' = $state('main');
  let isPinned = $state(false);
  let showOnboarding = $state(false);
  let showShortcuts = $state(false);
  let showLanguageSelector = $state(false);
  let showModeSelector = $state(false);
  let showTtsSelector = $state(false);
  let initialSettingsTab: 'translation' | 'display' | 'tts' | 'subscription' | 'about' = $state('translation');

  // Check if first run
  function checkFirstRun() {
    try {
      const completed = localStorage.getItem('auralis-onboarding-completed');
      if (!completed) {
        showOnboarding = true;
      }
    } catch (err) {
      console.error('[App] Failed to check first run:', err);
    }
  }

  // Offline setup state (lives here so it persists across tab/view switches)
  let offlineSetupProgress = $state(0);
  let offlineSetupMessage = $state('');
  let offlineSetupStep = $state('');
  let offlineReady = $state(false);

  // Apply display settings as CSS custom properties
  // Note: fontSize is passed directly to Transcript component, not set globally
  $effect(() => {
    const opacityValue = String(displaySettings.opacity);
    document.documentElement.style.setProperty('--app-opacity', opacityValue);
    // Apply partial opacity to #app for better visibility while keeping text readable
    // Use a milder range: when opacity is 0.3, apply 0.7; when opacity is 1.0, apply 1.0
    const adjustedOpacity = 0.7 + (displaySettings.opacity - 0.3) * 0.3 / 0.7;
    const appElement = document.getElementById('app');
    if (appElement) {
      appElement.style.opacity = String(adjustedOpacity);
    }
  });

  // Sync TTS engine settings with state
  $effect(() => {
    tts.setProvider(ttsSettings.provider);
    tts.setVoice(ttsSettings.voice);
    tts.setRate(ttsSettings.rate);
  });

  // Soniox client instance
  let sonioxClient: SonioxClient | null = null;

  // Event listener cleanup handles
  const unlisteners: Array<() => void> = [];

  // Error toast timer
  let errorTimer: ReturnType<typeof setTimeout> | null = null;

  // Settings save debounce timer
  let settingsSaveTimer: ReturnType<typeof setTimeout> | null = null;
  let pendingSettingsChanges = false;

  // Debounced settings persistence - waits 1 second after last change
  function queueSettingsSave(): void {
    pendingSettingsChanges = true;

    if (settingsSaveTimer) {
      clearTimeout(settingsSaveTimer);
    }

    settingsSaveTimer = setTimeout(() => {
      if (pendingSettingsChanges) {
        persistSettingsImpl();
        pendingSettingsChanges = false;
      }
    }, 1000); // Save after 1 second of inactivity
  }

  // ---------------------------------------------------------------------------
  // Computed helpers
  // ---------------------------------------------------------------------------

  function getStatusType(): 'idle' | 'recording' | 'error' | 'ready' {
    if (errorMessage) return 'error';
    if (isTranslating) return 'recording';
    if (translationSettings.mode === 'cloud' && sonioxConnectionStatus === 'connected') return 'ready';
    return 'idle';
  }

  function getStatusText(): string {
    if (errorMessage) return 'Error';
    if (isTranslating) {
      if (translationSettings.mode === 'cloud') {
        const statusMap: Record<string, string> = {
          connecting: 'Connecting...',
          connected: 'Translating...',
          disconnected: 'Disconnected',
          error: 'Connection error',
        };
        return statusMap[sonioxConnectionStatus ?? ''] ?? 'Translating...';
      }
      return 'Translating...';
    }
    return statusMessage;
  }

  // ---------------------------------------------------------------------------
  // Segment helpers
  // ---------------------------------------------------------------------------

  function addSegment(original: string, detectedLang: string, targetLang: string): void {
    // Filter out segments in unexpected languages
    if (translationSettings.translationType === 'two_way') {
      if (detectedLang !== translationSettings.sourceLanguage && detectedLang !== translationSettings.targetLanguage) return;
    }
    // One-way mode: accept any detected language (auto-detection enabled)
    segmentIdCounter++;
    segments.push({
      id: segmentIdCounter,
      original: original.trim(),
      translated: '',
      detectedLang,
      targetLang,
      status: 'pending',
      timestamp: Date.now(),
    });
    if (segments.length > displaySettings.maxLines) {
      segments.splice(0, segments.length - displaySettings.maxLines);
    }
    segments = segments;
  }

  /** Pair translation with the oldest pending segment */
  function pairTranslation(translatedText: string): void {
    const idx = segments.findIndex((s) => s.status === 'pending');
    if (idx !== -1) {
      segments[idx].translated = translatedText.trim();
      segments[idx].status = 'translated';
      segments = segments;
    } else {
      // Translation without a pending original — create a translated-only segment
      segmentIdCounter++;
      segments.push({
        id: segmentIdCounter,
        original: '',
        translated: translatedText.trim(),
        detectedLang: '',
        targetLang: '',
        status: 'translated',
        timestamp: Date.now(),
      });
      segments = segments;
    }
  }

  // ---------------------------------------------------------------------------
  // Settings persistence
  // ---------------------------------------------------------------------------

  async function loadSettings(): Promise<void> {
    // Load all settings using centralized store loaders
    try {
      await Promise.all([
        loadTranslationSettings().catch(err => console.error('[App] Failed to load translation settings:', err)),
        loadDisplaySettings().catch(err => console.error('[App] Failed to load display settings:', err)),
        loadTTSSettings().catch(err => console.error('[App] Failed to load TTS settings:', err)),
        loadApiKeys().catch(err => console.error('[App] Failed to load API keys:', err)),
        loadSubscriptionStatus().catch(err => console.error('[App] Failed to load subscription status:', err)),
      ]);
    } catch (err) {
      console.error('[App] Failed to load settings:', err);
    }
  }

  // Internal implementation of settings persistence (called by queueSettingsSave or directly for immediate save)
  async function persistSettingsImpl(): Promise<void> {
    // Stores handle their own persistence now
    // This function is kept for backward compatibility but can be removed later
  }

  // Public function - queues debounced save (most cases)
  function persistSettings(): void {
    queueSettingsSave();
  }

  // For immediate save (e.g., when starting translation or closing settings)
  async function saveSettingsImmediately(): Promise<void> {
    if (settingsSaveTimer) {
      clearTimeout(settingsSaveTimer);
      settingsSaveTimer = null;
    }
    pendingSettingsChanges = false;
    await persistSettingsImpl();
  }

  // ---------------------------------------------------------------------------
  // Cloud mode (Soniox)
  // ---------------------------------------------------------------------------

  async function startCloudMode(): Promise<void> {
    sonioxClient = new SonioxClient({
      api_key: subscriptionState.apiKey,
      source_language: translationSettings.sourceLanguage,
      target_language: translationSettings.targetLanguage,
      translation_type: translationSettings.translationType,
      endpoint_delay: translationSettings.endpointDelay,
      onOriginal: (text: string, is_final: boolean, language?: string) => {
        if (is_final && text.trim()) {
          const detectedLang = language ?? translationSettings.sourceLanguage;
          // Update detection state for one-way mode
          if (translationSettings.translationType === 'one_way' && language) {
            detectionState = {
              status: 'detected',
              detectedLanguage: language
              // Note: Confidence not provided by Soniox API, omitting field
            };
          }
          // Determine target lang: in two-way, it's the "other" language
          const target = translationSettings.translationType === 'two_way'
            ? (detectedLang === translationSettings.sourceLanguage ? translationSettings.targetLanguage : translationSettings.sourceLanguage)
            : translationSettings.targetLanguage;
          addSegment(text, detectedLang, target);
          provisionalText = '';
          provisionalLang = '';
        } else if (!is_final) {
          provisionalText = text;
          provisionalLang = language ?? '';
          // Update to detecting state during provisional transcription
          if (translationSettings.translationType === 'one_way' && isTranslating) {
            detectionState = { status: 'detecting' };
          }
        } else {
          // Empty final = clear provisional
          provisionalText = '';
          provisionalLang = '';
          // Reset to idle when endpoint detected
          if (translationSettings.translationType === 'one_way') {
            detectionState = { status: 'idle' };
          }
        }
      },
      onTranslation: (text: string, _is_final: boolean) => {
        if (text.trim()) {
          pairTranslation(text);
          speakTranslation(text, translationSettings.targetLanguage);
        }
      },
      onStatusChange: (status: ConnectionStatus) => {
        sonioxConnectionStatus = status;
      },
      onError: (error: string) => {
        showError(error);
        statusMessage = 'Connection error';
      },
    });

    const captureResult = await invoke<string>('start_audio_capture', { source: translationSettings.audioSource });
    statusMessage = captureResult;

    sonioxClient.connect();
  }

  function stopCloudMode(): void {
    if (sonioxClient) {
      sonioxClient.disconnect();
      sonioxClient = null;
    }
    sonioxConnectionStatus = null;
    provisionalText = '';
    provisionalLang = '';
  }

  // ---------------------------------------------------------------------------
  // Offline mode (MLX Python sidecar)
  // ---------------------------------------------------------------------------

  async function startOfflineMode(): Promise<void> {
    await invoke('start_local_pipeline', { source: translationSettings.audioSource });
    statusMessage = 'Starting offline pipeline...';
  }

  async function stopOfflineMode(): Promise<void> {
    await invoke('stop_local_pipeline');
    statusMessage = 'Pipeline stopped';
  }

  // ---------------------------------------------------------------------------
  // UI Handlers
  // ---------------------------------------------------------------------------

  async function handleToggleRecord(): Promise<void> {
    if (isTranslating) {
      await handleStop();
    } else {
      await handleStart();
    }
  }

  async function handleStart(): Promise<void> {
    try {
      errorMessage = '';
      isTranslating = true;
      // Set active audio sources based on current selection
      if (translationSettings.audioSource === 'both') {
        activeAudioSources = ['microphone', 'system'];
      } else {
        activeAudioSources = [translationSettings.audioSource];
      }
      statusMessage = 'Starting...';

      await saveSettingsImmediately();

      if (translationSettings.mode === 'cloud') {
        await startCloudMode();
      } else {
        await startOfflineMode();
      }
    } catch (error) {
      console.error('[Auralis] Start failed:', error);
      errorMessage = `Failed to start: ${error}`;
      statusMessage = 'Start failed';
      isTranslating = false;
    }
  }

  async function handleStop(): Promise<void> {
    try {
      // Stop any playing TTS
      tts.stop();

      if (translationSettings.mode === 'cloud') {
        stopCloudMode();
        await invoke<string>('stop_audio_capture');
      } else {
        await stopOfflineMode();
      }

      isTranslating = false;
      activeAudioSources = [];
      statusMessage = 'Stopped';
      detectionState = { status: 'idle' };
    } catch (error) {
      errorMessage = `Failed to stop: ${error}`;
      statusMessage = 'Stop failed';
      isTranslating = false;
    }
  }

  function handleOpenSettings() {
    initialSettingsTab = 'translation';
    currentView = 'settings';
  }

  function handleSettingsBack() {
    currentView = 'main';
    // Reload subscription status in case it changed in settings
    loadSubscriptionStatus();
  }

  function handleOpenSaved() {
    currentView = 'saved';
  }

  async function handleSettingsSave(settings: {
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
    summary_provider: string;
    claude_api_key: string;
    openai_api_key: string;
  }) {
    translationSettings.mode = settings.mode;
    subscriptionStore.update('apiKey', settings.soniox_api_key);
    subscriptionStore.update('googleApiKey', settings.google_api_key);
    subscriptionStore.update('elevenLabsApiKey', settings.elevenlabs_api_key);
    translationSettings.sourceLanguage = settings.source_language;
    translationSettings.targetLanguage = settings.target_language;
    translationSettings.translationType = settings.translation_type;
    translationSettings.audioSource = settings.audio_source;
    displaySettings.opacity = settings.opacity;
    displaySettings.fontSize = settings.font_size;
    displaySettings.maxLines = settings.max_lines;
    translationSettings.endpointDelay = settings.endpoint_delay;
    ttsSettings.enabled = settings.tts_enabled;
    ttsSettings.voice = settings.tts_voice;
    ttsSettings.rate = settings.tts_rate;
    ttsSettings.provider = settings.tts_provider;
    await saveSettingsImmediately();
    currentView = 'main';
  }

  async function handleClear() {
    // Auto-save before clearing if there are segments
    if (segments.length > 0) {
      try {
        const savedFilename = await invoke<string>('save_transcript', {
          segments: segments.map((s) => ({
            original: s.original,
            translated: s.translated,
            detected_lang: s.detectedLang,
            target_lang: s.targetLang,
            timestamp: s.timestamp,
          })),
        });

        // Auto-generate summary in the background with user's actual subscription tier
        invoke('generate_summary', { filename: savedFilename, tier: subscriptionState.tier })
          .catch((err) => console.warn('Failed to auto-generate summary:', err));
      } catch (err) {
        console.warn('Failed to auto-save transcript:', err);
      }
    }
    segments.length = 0;
    segments = segments;
    provisionalText = '';
    provisionalLang = '';
  }

  async function handleTogglePin() {
    isPinned = !isPinned;
    const appWindow = getCurrentWindow();
    await appWindow.setAlwaysOnTop(isPinned);
  }

  function handleSetAudioSource(source: AudioSource) {
    translationSettings.audioSource = source;
    persistSettings();
  }

  function handleToggleTts() {
    ttsSettings.enabled = !ttsSettings.enabled;
    persistSettings();
  }

  function handleShowShortcuts() {
    showShortcuts = true;
  }

  function handleShowLanguageSelector() {
    showLanguageSelector = true;
  }

  function handleSelectLanguage(source: string, target: string) {
    translationSettings.sourceLanguage = source;
    translationSettings.targetLanguage = target;
    persistSettings();
  }

  function handleShowModeSelector() {
    showModeSelector = true;
  }

  function handleSelectMode(mode: 'cloud' | 'offline', needsSettings: boolean) {
    if (needsSettings) {
      // Redirect to settings with Translation tab for API key
      initialSettingsTab = 'translation';
      currentView = 'settings';
    } else {
      translationSettings.mode = mode;
      persistSettings();
    }
  }

  function handleShowTtsSelector() {
    showTtsSelector = true;
  }

  function handleSelectTtsProvider(provider: 'webspeech' | 'edge' | 'google' | 'elevenlabs', needsSettings: boolean) {
    if (needsSettings) {
      // Redirect to settings with TTS tab
      initialSettingsTab = 'tts';
      currentView = 'settings';
    } else {
      ttsSettings.provider = provider;
      ttsSettings.enabled = true;
      persistSettings();
    }
  }

  function speakTranslation(text: string, targetLang: string): void {
    if (!ttsSettings.enabled || !text.trim()) return;
    tts.speak(text, targetLang);
  }

  function showError(msg: string) {
    errorMessage = msg;
    if (errorTimer) clearTimeout(errorTimer);
    errorTimer = setTimeout(() => {
      errorMessage = '';
    }, 5000);
  }

  // ---------------------------------------------------------------------------
  // Lifecycle
  // ---------------------------------------------------------------------------

  onMount(async () => {
    // Load all settings using centralized stores
    await loadSettings();

    // Start periodic sync with RevenueCat
    // This keeps subscription status up-to-date across all devices
    startPeriodicSync().catch((err) => {
      console.warn('Failed to start periodic sync:', err);
    });

    // Fetch platform capabilities from the Rust backend
    try {
      platformInfo = await invoke<PlatformInfo>('get_platform_info');
    } catch {
      platformInfo = { os: 'unknown', system_audio_available: true, offline_mode_available: true };
    }

    // Check if first run - show onboarding if needed
    checkFirstRun();

    // Background update check — silently check for updates on startup
    try {
      const update = await checkForUpdate();
      if (update?.available) {
        updateAvailable = true;
      }
    } catch {
      // Silently ignore — user can check manually in About
    }

    // Preload offline pipeline so models are ready when user clicks Start
    if (translationSettings.mode === 'offline') {
      invoke('preload_pipeline').catch(() => {
        // Silently ignore — will load on demand when user clicks Start
      });
    }

    const audioDataUnlisten = await listen<number[]>('audio-data', (event) => {
      if (sonioxClient && translationSettings.mode === 'cloud') {
        const pcm = new Uint8Array(event.payload);
        sonioxClient.sendAudio(pcm);
      }
    });

    const pipelineResultUnlisten = await listen<string>('pipeline-result', (event) => {
      try {
        const data = JSON.parse(event.payload);

        if (data.type === 'original') {
          // Original text from ASR — show immediately (translation will follow later)
          const text = (data.text ?? '').trim();
          if (text) {
            const detectedLang = data.source_lang ?? translationSettings.sourceLanguage;
            const target = data.target_lang ?? translationSettings.targetLanguage;
            // Update detection state for one-way mode
            if (translationSettings.translationType === 'one_way' && data.source_lang) {
              detectionState = {
                status: 'detected',
                detectedLanguage: data.source_lang
                // Note: Confidence not provided by Whisper MLX, omitting field
              };
            }
            addSegment(text, detectedLang, target);
          }
        } else if (data.type === 'result') {
          // Full result with translation — update the matching pending segment in place
          const original = (data.original ?? '').trim();
          const translated = (data.translated ?? '').trim();
          const detectedLang = data.source_lang ?? translationSettings.sourceLanguage;
          const target = data.target_lang ?? translationSettings.targetLanguage;

          if (original && translated) {
            // Find the pending segment that matches this original text and update it
            const pendingIdx = segments.findIndex(
              (s) => s.status === 'pending' && s.original === original
            );
            if (pendingIdx !== -1) {
              segments[pendingIdx].translated = translated;
              segments[pendingIdx].status = 'translated';
              segments[pendingIdx].targetLang = target;
              segments = segments;
            } else {
              // No matching pending segment — add as translated
              segmentIdCounter++;
              segments.push({
                id: segmentIdCounter,
                original,
                translated,
                detectedLang,
                targetLang: target,
                status: 'translated',
                timestamp: Date.now(),
              });
              if (segments.length > displaySettings.maxLines) {
                segments.splice(0, segments.length - displaySettings.maxLines);
              }
              segments = segments;
            }
            // Speak translated text if TTS is enabled
            if (translated && target) {
              speakTranslation(translated, target);
            }
          } else if (translated) {
            pairTranslation(translated);
            speakTranslation(translated, translationSettings.targetLanguage);
          }

          provisionalText = '';
          provisionalLang = '';
        }
      } catch {
        // Non-JSON or unexpected format; ignore
      }
    });

    const pipelineStatusUnlisten = await listen<string>('pipeline-status', (event) => {
      try {
        const data = JSON.parse(event.payload);
        if (data.message) {
          statusMessage = data.message;
        }
      } catch {
        statusMessage = event.payload;
      }
    });

    unlisteners.push(
      audioDataUnlisten,
      pipelineResultUnlisten,
      pipelineStatusUnlisten
    );

    // Offline setup progress — persists across view/tab switches
    const offlineSetupUnlisten = await listen<{ step: string; message: string; progress: number }>(
      'offline-setup-progress',
      (event) => {
        offlineSetupProgress = event.payload.progress;
        offlineSetupMessage = event.payload.message;
        offlineSetupStep = event.payload.step;
        if (event.payload.progress >= 100) {
          offlineReady = true;
        }
      }
    );
    unlisteners.push(offlineSetupUnlisten);

    // Keyboard shortcuts handler
    const handleKeydown = (e: KeyboardEvent) => {
      // '?' key to show keyboard shortcuts panel
      if (e.key === '?' && !e.metaKey && !e.ctrlKey) {
        e.preventDefault();
        showShortcuts = true;
      }
      // Escape to close shortcuts panel
      if (e.key === 'Escape') {
        if (showShortcuts) {
          showShortcuts = false;
        } else if (currentView === 'settings') {
          handleSettingsBack();
        }
      }
      // Space to toggle recording (only in main view and not in inputs)
      if (e.key === ' ' && currentView === 'main' && !(e.target as HTMLElement).matches('input, textarea, button')) {
        e.preventDefault();
        handleToggleRecord();
      }
    };
    window.addEventListener('keydown', handleKeydown);
    unlisteners.push({ unlisten: () => window.removeEventListener('keydown', handleKeydown) });

    statusMessage = 'Ready';
  });

  onDestroy(() => {
    // Auto-save transcript on close
    if (segments.length > 0) {
      invoke('save_transcript', {
        segments: segments.map((s) => ({
          original: s.original,
          translated: s.translated,
          detected_lang: s.detectedLang,
          target_lang: s.targetLang,
          timestamp: s.timestamp,
        })),
      }).catch((err) => {
        console.warn('Failed to auto-save transcript on close:', err);
      });
    }

    if (sonioxClient) {
      sonioxClient.disconnect();
      sonioxClient = null;
    }
    for (const unlisten of unlisteners) {
      unlisten();
    }
    if (errorTimer) clearTimeout(errorTimer);

    // Unsubscribe from stores
    unsubscribeTranslation();
    unsubscribeDisplay();
    unsubscribeTTS();
    unsubscribeSubscription();

    // Stop periodic subscription sync
    stopPeriodicSync();
  });
</script>

{#if currentView === 'main'}
  <ControlBar
    {isTranslating}
    statusText={getStatusText()}
    statusType={getStatusType()}
    {isPinned}
    audioSource={translationSettings.audioSource}
    {activeAudioSources}
    ttsEnabled={ttsSettings.enabled}
    {platformInfo}
    {updateAvailable}
    sourceLanguage={translationSettings.sourceLanguage}
    targetLanguage={translationSettings.targetLanguage}
    mode={translationSettings.mode}
    translationType={translationSettings.translationType}
    detectionState={detectionState}
    onToggleRecord={handleToggleRecord}
    onOpenSettings={handleOpenSettings}
    onOpenSaved={handleOpenSaved}
    onClear={handleClear}
    onShowShortcuts={handleShowShortcuts}
    onShowLanguageSelector={handleShowLanguageSelector}
    onShowModeSelector={handleShowModeSelector}
    onShowTtsSelector={handleShowTtsSelector}
    onTogglePin={handleTogglePin}
    onSetAudioSource={handleSetAudioSource}
    onToggleTts={handleToggleTts}
  />

  {#if errorMessage}
    <div class="error-toast">
      {errorMessage}
    </div>
  {/if}

  <FirstRunOnboarding show={showOnboarding} onFinish={() => showOnboarding = false} />

  <KeyboardShortcuts show={showShortcuts} onClose={() => showShortcuts = false} />

  <QuickLanguageSelector
    show={showLanguageSelector}
    sourceLanguage={translationSettings.sourceLanguage}
    targetLanguage={translationSettings.targetLanguage}
    onSelect={handleSelectLanguage}
    onClose={() => showLanguageSelector = false}
  />

  <QuickModeSelector
    show={showModeSelector}
    currentMode={translationSettings.mode}
    hasApiKey={!!subscriptionState.apiKey}
    onSelect={handleSelectMode}
    onClose={() => showModeSelector = false}
  />

  <QuickTtsSelector
    show={showTtsSelector}
    currentProvider={ttsSettings.provider}
    hasApiKey={{ google: !!subscriptionState.googleApiKey, elevenlabs: !!subscriptionState.elevenLabsApiKey }}
    onSelect={handleSelectTtsProvider}
    onClose={() => showTtsSelector = false}
  />

  <Transcript
    sourceLanguage={translationSettings.sourceLanguage}
    targetLanguage={translationSettings.targetLanguage}
    translationType={translationSettings.translationType}
    mode={translationSettings.mode}
    audioSource={translationSettings.audioSource}
    {segments}
    {provisionalText}
    {provisionalLang}
    fontSize={displaySettings.fontSize}
    onOpenSettings={handleOpenSettings}
  />
{:else if currentView === 'settings'}
  <SettingsView
    initialTab={initialSettingsTab}
    mode={translationSettings.mode}
    sonioxApiKey={subscriptionState.apiKey}
    googleApiKey={subscriptionState.googleApiKey}
    sourceLanguage={translationSettings.sourceLanguage}
    targetLanguage={translationSettings.targetLanguage}
    translationType={translationSettings.translationType}
    audioSource={translationSettings.audioSource}
    {isTranslating}
    opacity={displaySettings.opacity}
    fontSize={displaySettings.fontSize}
    maxLines={displaySettings.maxLines}
    endpointDelay={translationSettings.endpointDelay}
    ttsEnabled={ttsSettings.enabled}
    ttsVoice={ttsSettings.voice}
    ttsRate={ttsSettings.rate}
    ttsProvider={ttsSettings.provider}
    tier={subscriptionState.tier}
    apiKey={subscriptionState.apiKey}
    platformInfo={platformInfo}
    onBack={handleSettingsBack}
  />
{:else}
  <SavedTranscripts
    subscriptionTier={subscriptionState.tier}
    onBack={() => { currentView = 'main'; }}
  />
{/if}

<style>
  .error-toast {
    position: absolute;
    top: 48px;
    left: var(--space-md);
    right: var(--space-md);
    padding: var(--space-sm) var(--space-md);
    background: var(--danger-dim);
    color: var(--danger);
    font-size: var(--font-size-sm);
    border-radius: var(--radius-sm);
    border: 1px solid rgba(255, 77, 77, 0.2);
    z-index: var(--z-header);
    animation: fadeIn 0.2s ease, fadeOut 0.3s ease 4.7s forwards;
    pointer-events: none;
  }

  @keyframes fadeOut {
    from { opacity: 1; transform: translateY(0); }
    to { opacity: 0; transform: translateY(-8px); }
  }
</style>
