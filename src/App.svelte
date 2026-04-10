<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { SonioxClient } from './js/soniox';
  import type { ConnectionStatus } from './js/soniox';
  import type { Segment, OperatingMode, TranslationType, AudioSource } from './types';
  import ControlBar from './components/ControlBar.svelte';
  import Transcript from './components/Transcript.svelte';
  import SettingsView from './components/SettingsView.svelte';
  import { tts } from './js/tts';

  // ---------------------------------------------------------------------------
  // State
  // ---------------------------------------------------------------------------

  let mode: OperatingMode = $state('cloud');
  let sourceLanguage = $state('en');
  let targetLanguage = $state('vi');
  let translationType: TranslationType = $state('one_way');
  let audioSource: AudioSource = $state('microphone');
  let displayOpacity = $state(0.88);
  let displayFontSize = $state(14);
  let displayMaxLines = $state(100);
  let endpointDelay = $state(1.0);
  let ttsEnabled = $state(false);
  let ttsVoice = $state('');
  let ttsRate = $state(1.0);
  let sonioxApiKey = $state('');
  let isTranslating = $state(false);
  let statusMessage = $state('Ready');
  let errorMessage = $state('');
  let sonioxConnectionStatus: ConnectionStatus | null = $state(null);

  // Segment-based transcript model
  let segments: Segment[] = $state([]);
  let segmentIdCounter = 0;
  let provisionalText = $state('');
  let provisionalLang = $state('');

  // UI state
  let currentView: 'main' | 'settings' = $state('main');
  let isPinned = $state(false);

  // Apply display settings as CSS custom properties
  $effect(() => {
    document.documentElement.style.setProperty('--app-opacity', String(displayOpacity));
    document.documentElement.style.setProperty('--font-size-base', `${displayFontSize}px`);
  });

  // Sync TTS engine settings with state
  $effect(() => {
    tts.setVoice(ttsVoice);
    tts.setRate(ttsRate);
  });

  // Soniox client instance
  let sonioxClient: SonioxClient | null = null;

  // Event listener cleanup handles
  const unlisteners: Array<() => void> = [];

  // Error toast timer
  let errorTimer: ReturnType<typeof setTimeout> | null = null;

  // ---------------------------------------------------------------------------
  // Computed helpers
  // ---------------------------------------------------------------------------

  function getStatusType(): 'idle' | 'recording' | 'error' | 'ready' {
    if (errorMessage) return 'error';
    if (isTranslating) return 'recording';
    if (mode === 'cloud' && sonioxConnectionStatus === 'connected') return 'ready';
    return 'idle';
  }

  function getStatusText(): string {
    if (errorMessage) return 'Error';
    if (isTranslating) {
      if (mode === 'cloud') {
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
    if (segments.length > displayMaxLines) {
      segments.splice(0, segments.length - displayMaxLines);
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
    try {
      const settings = await invoke<{
        mode: string;
        soniox_api_key: string;
        source_language: string;
        target_language: string;
        translation_type: string;
        audio_source: string;
        opacity: number;
        font_size: number;
        max_lines: number;
        endpoint_delay?: number;
        tts_enabled?: boolean;
        tts_voice?: string;
        tts_rate?: number;
      }>('get_settings');

      if (settings.mode === 'cloud' || settings.mode === 'offline') {
        mode = settings.mode;
      }
      if (settings.translation_type === 'one_way' || settings.translation_type === 'two_way') {
        translationType = settings.translation_type;
      }
      if (settings.audio_source === 'microphone' || settings.audio_source === 'system' || settings.audio_source === 'both') {
        audioSource = settings.audio_source;
      }
      if (settings.opacity >= 0.3 && settings.opacity <= 1.0) {
        displayOpacity = settings.opacity;
      }
      if (settings.font_size >= 12 && settings.font_size <= 24) {
        displayFontSize = settings.font_size;
      }
      if (settings.max_lines >= 10 && settings.max_lines <= 200) {
        displayMaxLines = settings.max_lines;
      }
      endpointDelay = (settings.endpoint_delay as number) || 1.0;
      ttsEnabled = settings.tts_enabled as boolean;
      ttsVoice = settings.tts_voice ?? '';
      ttsRate = settings.tts_rate ?? 1.0;
      sonioxApiKey = settings.soniox_api_key;
      sourceLanguage = settings.source_language;
      targetLanguage = settings.target_language;
    } catch (err) {
      console.warn('Failed to load settings, using defaults:', err);
    }
  }

  async function persistSettings(): Promise<void> {
    await invoke('save_settings', {
      settings: {
        mode,
        soniox_api_key: sonioxApiKey,
        source_language: sourceLanguage,
        target_language: targetLanguage,
        translation_type: translationType,
        audio_source: audioSource,
        opacity: displayOpacity,
        font_size: displayFontSize,
        max_lines: displayMaxLines,
        endpoint_delay: endpointDelay,
        tts_enabled: ttsEnabled,
        tts_voice: ttsVoice,
        tts_rate: ttsRate,
      },
    });
  }

  // ---------------------------------------------------------------------------
  // Cloud mode (Soniox)
  // ---------------------------------------------------------------------------

  async function startCloudMode(): Promise<void> {
    sonioxClient = new SonioxClient({
      api_key: sonioxApiKey,
      source_language: sourceLanguage,
      target_language: targetLanguage,
      translation_type: translationType,
      endpoint_delay: endpointDelay,
      onOriginal: (text: string, is_final: boolean, language?: string) => {
        if (is_final && text.trim()) {
          const detectedLang = language ?? sourceLanguage;
          // Determine target lang: in two-way, it's the "other" language
          const target = translationType === 'two_way'
            ? (detectedLang === sourceLanguage ? targetLanguage : sourceLanguage)
            : targetLanguage;
          addSegment(text, detectedLang, target);
          provisionalText = '';
          provisionalLang = '';
        } else if (!is_final) {
          provisionalText = text;
          provisionalLang = language ?? '';
        } else {
          // Empty final = clear provisional
          provisionalText = '';
          provisionalLang = '';
        }
      },
      onTranslation: (text: string, _is_final: boolean) => {
        if (text.trim()) {
          pairTranslation(text);
          speakTranslation(text, targetLanguage);
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

    const captureResult = await invoke<string>('start_audio_capture', { source: audioSource });
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
    console.log('[Auralis] Starting offline pipeline, source:', audioSource);
    await invoke('start_local_pipeline', { source: audioSource });
    statusMessage = 'Starting offline pipeline...';
    console.log('[Auralis] Pipeline invoke returned');
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
      statusMessage = 'Starting...';
      console.log('[Auralis] handleStart, mode:', mode);

      await persistSettings();

      if (mode === 'cloud') {
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

      if (mode === 'cloud') {
        stopCloudMode();
        await invoke<string>('stop_audio_capture');
      } else {
        await stopOfflineMode();
      }

      isTranslating = false;
      statusMessage = 'Stopped';
    } catch (error) {
      errorMessage = `Failed to stop: ${error}`;
      statusMessage = 'Stop failed';
      isTranslating = false;
    }
  }

  function handleOpenSettings() {
    currentView = 'settings';
  }

  function handleSettingsBack() {
    currentView = 'main';
  }

  function handleSettingsSave(settings: {
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
  }) {
    mode = settings.mode;
    sonioxApiKey = settings.soniox_api_key;
    sourceLanguage = settings.source_language;
    targetLanguage = settings.target_language;
    translationType = settings.translation_type;
    audioSource = settings.audio_source;
    displayOpacity = settings.opacity;
    displayFontSize = settings.font_size;
    displayMaxLines = settings.max_lines;
    endpointDelay = settings.endpoint_delay;
    ttsEnabled = settings.tts_enabled;
    ttsVoice = settings.tts_voice;
    ttsRate = settings.tts_rate;
    persistSettings();
    currentView = 'main';
  }

  function handleClear() {
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
    audioSource = source;
    persistSettings();
  }

  function handleToggleTts() {
    ttsEnabled = !ttsEnabled;
    persistSettings();
  }

  function speakTranslation(text: string, targetLang: string): void {
    if (!ttsEnabled || !text.trim()) return;
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
    await loadSettings();

    const audioDataUnlisten = await listen<ArrayBuffer>('audio-data', (event) => {
      if (sonioxClient && mode === 'cloud') {
        sonioxClient.sendAudio(event.payload);
      }
    });

    const pipelineResultUnlisten = await listen<string>('pipeline-result', (event) => {
      try {
        const data = JSON.parse(event.payload);

        if (data.type === 'original') {
          // Original text from ASR — show immediately (translation will follow later)
          const text = (data.text ?? '').trim();
          if (text) {
            const detectedLang = data.source_lang ?? sourceLanguage;
            const target = data.target_lang ?? targetLanguage;
            addSegment(text, detectedLang, target);
          }
        } else if (data.type === 'result') {
          // Full result with translation — replace all pending segments with one clean entry
          const original = (data.original ?? '').trim();
          const translated = (data.translated ?? '').trim();
          const detectedLang = data.source_lang ?? sourceLanguage;
          const target = data.target_lang ?? targetLanguage;

          if (original && translated) {
            // Remove all pending segments (they were intermediate chunks)
            // and add one clean translated segment
            for (let i = segments.length - 1; i >= 0; i--) {
              if (segments[i].status === 'pending') segments.splice(i, 1);
            }
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
            if (segments.length > displayMaxLines) {
              segments.splice(0, segments.length - displayMaxLines);
            }
            segments = segments;
            // Speak translated text if TTS is enabled
            if (translated && target) {
              speakTranslation(translated, target);
            }
          } else if (translated) {
            pairTranslation(translated);
            speakTranslation(translated, targetLanguage);
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

    statusMessage = 'Ready';
  });

  onDestroy(() => {
    if (sonioxClient) {
      sonioxClient.disconnect();
      sonioxClient = null;
    }
    for (const unlisten of unlisteners) {
      unlisten();
    }
    if (errorTimer) clearTimeout(errorTimer);
  });
</script>

{#if currentView === 'main'}
  <ControlBar
    {isTranslating}
    statusText={getStatusText()}
    statusType={getStatusType()}
    {isPinned}
    audioSource={audioSource}
    ttsEnabled={ttsEnabled}
    onToggleRecord={handleToggleRecord}
    onOpenSettings={handleOpenSettings}
    onClear={handleClear}
    onTogglePin={handleTogglePin}
    onSetAudioSource={handleSetAudioSource}
    onToggleTts={handleToggleTts}
  />

  {#if errorMessage}
    <div class="error-toast">
      {errorMessage}
    </div>
  {/if}

  <Transcript
    {sourceLanguage}
    {targetLanguage}
    {translationType}
    {segments}
    {provisionalText}
    {provisionalLang}
    fontSize={displayFontSize}
  />
{:else}
  <SettingsView
    {mode}
    {sonioxApiKey}
    {sourceLanguage}
    {targetLanguage}
    {translationType}
    {audioSource}
    {isTranslating}
    opacity={displayOpacity}
    fontSize={displayFontSize}
    maxLines={displayMaxLines}
    endpointDelay={endpointDelay}
    ttsEnabled={ttsEnabled}
    ttsVoice={ttsVoice}
    ttsRate={ttsRate}
    onSave={handleSettingsSave}
    onBack={handleSettingsBack}
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
    z-index: 10;
    animation: fadeIn 0.2s ease;
    pointer-events: none;
  }
</style>
