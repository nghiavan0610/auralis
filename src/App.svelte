<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { SonioxClient } from './js/soniox';
  import type { ConnectionStatus } from './js/soniox';
  import StatusIndicator from './components/StatusIndicator.svelte';
  import DualPanel from './components/DualPanel.svelte';
  import ModelDownloader from './components/ModelDownloader.svelte';

  // ---------------------------------------------------------------------------
  // Types
  // ---------------------------------------------------------------------------

  interface TranscriptEntry {
    text: string;
    language: string;
    timestamp: number;
    is_final: boolean;
  }

  interface TranslationEntry {
    original: string;
    translated: string;
    source_lang: string;
    target_lang: string;
    timestamp: number;
  }

  type OperatingMode = 'cloud' | 'offline';

  // ---------------------------------------------------------------------------
  // State
  // ---------------------------------------------------------------------------

  let mode: OperatingMode = $state('cloud');
  let sourceLanguage = $state('en');
  let targetLanguage = $state('vi');
  let sonioxApiKey = $state('');
  let isTranslating = $state(false);
  let audioCapturing = $state(false);
  let statusMessage = $state('Ready');
  let errorMessage = $state('');
  let sonioxConnectionStatus: ConnectionStatus | null = $state(null);

  let transcriptions: TranscriptEntry[] = $state([]);
  let translations: TranslationEntry[] = $state([]);

  // Provisional (in-progress) transcription from Soniox
  let provisionalText = $state('');

  // Soniox client instance for cloud mode
  let sonioxClient: SonioxClient | null = null;

  // Event listener cleanup handles
  const unlisteners: Array<() => void> = [];

  // ---------------------------------------------------------------------------
  // Computed helpers
  // ---------------------------------------------------------------------------

  function canStart(): boolean {
    if (isTranslating) return false;
    if (mode === 'cloud' && !sonioxApiKey.trim()) return false;
    return true;
  }

  function getStatusIndicator(): 'ready' | 'error' | 'processing' | 'idle' {
    if (errorMessage) return 'error';
    if (isTranslating) return 'processing';
    if (mode === 'cloud' && sonioxConnectionStatus === 'connected') return 'ready';
    return 'idle';
  }

  function getStatusText(): string {
    if (errorMessage) return 'Error';
    if (isTranslating) {
      if (mode === 'cloud') {
        const statusMap: Record<string, string> = {
          connecting: 'Connecting to Soniox...',
          connected: 'Translating (cloud)...',
          disconnected: 'Disconnected',
          error: 'Connection error',
        };
        return statusMap[sonioxConnectionStatus ?? ''] ?? 'Translating (cloud)...';
      }
      return 'Translating (offline)...';
    }
    return statusMessage;
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
      }>('get_settings');

      if (settings.mode === 'cloud' || settings.mode === 'offline') {
        mode = settings.mode;
      }
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
      },
    });
  }

  // ---------------------------------------------------------------------------
  // Cloud mode (Soniox)
  // ---------------------------------------------------------------------------

  async function startCloudMode(): Promise<void> {
    // Create the Soniox client with callbacks
    sonioxClient = new SonioxClient({
      api_key: sonioxApiKey,
      source_language: sourceLanguage,
      target_language: targetLanguage,
      translation_type: 'one_way',
      onOriginal: (text: string, is_final: boolean) => {
        if (is_final && text.trim()) {
          transcriptions = [
            ...transcriptions,
            {
              text: text.trim(),
              language: sourceLanguage,
              timestamp: Date.now(),
              is_final: true,
            },
          ];
          if (transcriptions.length > 100) {
            transcriptions = transcriptions.slice(-100);
          }
          provisionalText = '';
        } else if (!is_final) {
          // Update provisional (in-progress) text
          provisionalText = text;
        }
      },
      onTranslation: (text: string, _is_final: boolean) => {
        if (text.trim()) {
          // Find the most recent transcription to pair with this translation
          const lastOriginal =
            transcriptions.length > 0
              ? transcriptions[transcriptions.length - 1].text
              : '';
          translations = [
            ...translations,
            {
              original: lastOriginal,
              translated: text.trim(),
              source_lang: sourceLanguage,
              target_lang: targetLanguage,
              timestamp: Date.now(),
            },
          ];
          if (translations.length > 100) {
            translations = translations.slice(-100);
          }
        }
      },
      onStatusChange: (status: ConnectionStatus) => {
        sonioxConnectionStatus = status;
      },
      onError: (error: string) => {
        errorMessage = error;
        statusMessage = 'Connection error';
      },
    });

    // Start the Rust audio capture (streams PCM via "audio-data" events)
    const captureResult = await invoke<string>('start_audio_capture');
    statusMessage = captureResult;

    // Connect Soniox WebSocket
    sonioxClient.connect();
  }

  function stopCloudMode(): void {
    if (sonioxClient) {
      sonioxClient.disconnect();
      sonioxClient = null;
    }
    sonioxConnectionStatus = null;
    provisionalText = '';
  }

  // ---------------------------------------------------------------------------
  // Offline mode (MLX Python sidecar)
  // ---------------------------------------------------------------------------

  async function startOfflineMode(): Promise<void> {
    await invoke('start_local_pipeline');
    statusMessage = 'Starting offline pipeline...';
  }

  async function stopOfflineMode(): Promise<void> {
    await invoke('stop_local_pipeline');
    statusMessage = 'Pipeline stopped';
  }

  // ---------------------------------------------------------------------------
  // Start / Stop handlers
  // ---------------------------------------------------------------------------

  async function handleStart(): Promise<void> {
    try {
      errorMessage = '';
      isTranslating = true;
      statusMessage = 'Starting...';

      // Save settings before starting either mode
      await persistSettings();

      if (mode === 'cloud') {
        await startCloudMode();
      } else {
        await startOfflineMode();
      }
    } catch (error) {
      errorMessage = `Failed to start: ${error}`;
      statusMessage = 'Start failed';
      isTranslating = false;
    }
  }

  async function handleStop(): Promise<void> {
    try {
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

  // ---------------------------------------------------------------------------
  // Lifecycle
  // ---------------------------------------------------------------------------

  onMount(async () => {
    await loadSettings();

    // --- Cloud mode: audio-data forwarding to Soniox ---
    const audioDataUnlisten = await listen<ArrayBuffer>('audio-data', (event) => {
      if (sonioxClient && mode === 'cloud') {
        sonioxClient.sendAudio(event.payload);
      }
    });

    // --- Cloud mode: mic capture status ---
    const audioCaptureUnlisten = await listen<{ is_capturing: boolean }>(
      'audio-capture',
      (event) => {
        audioCapturing = event.payload.is_capturing;
      }
    );

    // --- Offline mode: pipeline results ---
    const pipelineResultUnlisten = await listen<string>('pipeline-result', (event) => {
      try {
        const data = JSON.parse(event.payload);
        if (data.type === 'result') {
          const original = data.original ?? data.source_text ?? '';
          const translated = data.translation ?? data.translated_text ?? '';

          if (original.trim()) {
            transcriptions = [
              ...transcriptions,
              {
                text: original.trim(),
                language: data.source_lang ?? sourceLanguage,
                timestamp: Date.now(),
                is_final: true,
              },
            ];
            if (transcriptions.length > 100) {
              transcriptions = transcriptions.slice(-100);
            }
          }

          if (translated.trim()) {
            translations = [
              ...translations,
              {
                original,
                translated: translated.trim(),
                source_lang: data.source_lang ?? sourceLanguage,
                target_lang: data.target_lang ?? targetLanguage,
                timestamp: Date.now(),
              },
            ];
            if (translations.length > 100) {
              translations = translations.slice(-100);
            }
          }
        }
      } catch {
        // Non-JSON or unexpected format; ignore
      }
    });

    // --- Offline mode: pipeline status ---
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
      audioCaptureUnlisten,
      pipelineResultUnlisten,
      pipelineStatusUnlisten
    );

    statusMessage = 'Ready';
  });

  onDestroy(() => {
    // Clean up Soniox client
    if (sonioxClient) {
      sonioxClient.disconnect();
      sonioxClient = null;
    }
    // Clean up all event listeners
    for (const unlisten of unlisteners) {
      unlisten();
    }
  });

  // ---------------------------------------------------------------------------
  // Language options
  // ---------------------------------------------------------------------------

  const languages = [
    { code: 'en', label: 'English' },
    { code: 'vi', label: 'Vietnamese' },
    { code: 'es', label: 'Spanish' },
    { code: 'fr', label: 'French' },
    { code: 'de', label: 'German' },
    { code: 'zh', label: 'Chinese' },
    { code: 'ja', label: 'Japanese' },
    { code: 'ko', label: 'Korean' },
    { code: 'pt', label: 'Portuguese' },
    { code: 'ru', label: 'Russian' },
    { code: 'ar', label: 'Arabic' },
    { code: 'hi', label: 'Hindi' },
  ];
</script>

<div class="container">
  <header>
    <h1>Auralis</h1>
    <p>Real-time Speech Translation</p>
  </header>

  <div style="margin-top: 1.5rem;">
    <StatusIndicator
      status={getStatusIndicator()}
      text={getStatusText()}
    />
    {#if errorMessage}
      <div class="error-banner">
        {errorMessage}
      </div>
    {/if}
  </div>

  <!-- Mode selector -->
  <div class="mode-selector">
    <span class="mode-label">Translation Mode</span>
    <div class="mode-options">
      <label class="mode-option" class:active={mode === 'cloud'}>
        <input
          type="radio"
          name="mode"
          value="cloud"
          bind:group={mode}
          disabled={isTranslating}
        />
        <div class="mode-card">
          <span class="mode-name">Cloud (Soniox)</span>
          <span class="mode-latency">~150ms latency</span>
        </div>
      </label>
      <label class="mode-option" class:active={mode === 'offline'}>
        <input
          type="radio"
          name="mode"
          value="offline"
          bind:group={mode}
          disabled={isTranslating}
        />
        <div class="mode-card">
          <span class="mode-name">Offline (MLX)</span>
          <span class="mode-latency">~1s latency</span>
        </div>
      </label>
    </div>
  </div>

  <!-- Settings -->
  <div class="controls">
    {#if mode === 'cloud'}
      <div class="setting-row">
        <label for="soniox-api-key">Soniox API Key</label>
        <div class="api-key-input">
          <input
            id="soniox-api-key"
            type="password"
            placeholder="Enter your Soniox API key"
            bind:value={sonioxApiKey}
            disabled={isTranslating}
          />
          <a
            href="https://soniox.com/api-keys"
            target="_blank"
            rel="noopener noreferrer"
            class="api-key-link"
          >
            Get API key
          </a>
        </div>
      </div>
    {/if}

    <div class="language-row">
      <div class="language-selector">
        <label for="source-language">Source</label>
        <select id="source-language" bind:value={sourceLanguage} disabled={isTranslating}>
          {#each languages as lang}
            <option value={lang.code}>{lang.label}</option>
          {/each}
        </select>
      </div>

      <div class="language-arrow">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M5 12h14M12 5l7 7-7 7"/>
        </svg>
      </div>

      <div class="language-selector">
        <label for="target-language">Target</label>
        <select id="target-language" bind:value={targetLanguage} disabled={isTranslating}>
          {#each languages as lang}
            <option value={lang.code}>{lang.label}</option>
          {/each}
        </select>
      </div>

      <div style="flex: 1;"></div>

      {#if !isTranslating}
        <button class="primary" onclick={handleStart} disabled={!canStart()}>
          Start Translation
        </button>
      {:else}
        <button class="danger" onclick={handleStop}>
          Stop Translation
        </button>
      {/if}
    </div>
  </div>

  <!-- Provisional text (in-progress transcription from cloud mode) -->
  {#if isTranslating && mode === 'cloud' && provisionalText}
    <div class="provisional-text">
      <span class="provisional-label">Hearing:</span> {provisionalText}
    </div>
  {/if}

  <!-- Dual panel: transcriptions and translations -->
  <DualPanel
    {sourceLanguage}
    {targetLanguage}
    {transcriptions}
    {translations}
  />

  <!-- Status footer -->
  <div class="status-footer">
    <div class="status-item">
      <strong>Mode:</strong>
      {mode === 'cloud' ? 'Cloud (Soniox)' : 'Offline (MLX)'}
    </div>
    <div class="status-item">
      <strong>Audio:</strong>
      {audioCapturing ? 'Capturing' : 'Idle'}
    </div>
    <div class="status-item">
      <strong>Status:</strong>
      {statusMessage}
    </div>
  </div>

  <!-- Model downloader for offline mode (kept separate from pipeline) -->
  <details class="model-section">
    <summary>Offline Model Downloads</summary>
    <ModelDownloader />
  </details>
</div>

<style>
  .container {
    max-width: 1400px;
    margin: 0 auto;
    padding: 2rem;
    width: 100%;
    height: 100%;
  }

  header {
    margin-bottom: 1rem;
  }

  h1 {
    font-size: 2.2em;
    line-height: 1.1;
    margin-bottom: 0.5rem;
  }

  p {
    color: rgba(255, 255, 255, 0.6);
  }

  /* Error banner */
  .error-banner {
    margin-top: 1rem;
    color: #f44336;
    padding: 0.75rem 1rem;
    background-color: rgba(244, 67, 54, 0.1);
    border-radius: 6px;
    font-size: 0.9rem;
    border: 1px solid rgba(244, 67, 54, 0.2);
  }

  /* Mode selector */
  .mode-selector {
    margin-top: 1.5rem;
  }

  .mode-label {
    display: block;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.8);
    margin-bottom: 0.5rem;
    font-size: 0.95rem;
  }

  .mode-options {
    display: flex;
    gap: 0.75rem;
  }

  .mode-option {
    cursor: pointer;
    flex: 1;
  }

  .mode-option input[type="radio"] {
    display: none;
  }

  .mode-card {
    padding: 0.75rem 1rem;
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.15);
    background-color: rgba(255, 255, 255, 0.03);
    transition: all 0.2s ease;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .mode-option:hover .mode-card {
    border-color: rgba(255, 255, 255, 0.3);
    background-color: rgba(255, 255, 255, 0.05);
  }

  .mode-option.active .mode-card {
    border-color: #646cff;
    background-color: rgba(100, 108, 255, 0.1);
  }

  .mode-name {
    font-weight: 600;
    font-size: 0.95rem;
    color: rgba(255, 255, 255, 0.9);
  }

  .mode-latency {
    font-size: 0.8rem;
    color: rgba(255, 255, 255, 0.5);
  }

  /* Controls */
  .controls {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    margin-top: 1rem;
    padding: 1rem;
    background-color: rgba(255, 255, 255, 0.03);
    border-radius: 8px;
  }

  .setting-row {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .setting-row label {
    font-weight: 500;
    color: rgba(255, 255, 255, 0.8);
    font-size: 0.9rem;
  }

  .api-key-input {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .api-key-input input {
    flex: 1;
    padding: 0.5rem 0.75rem;
    border-radius: 6px;
    border: 1px solid rgba(255, 255, 255, 0.2);
    background-color: rgba(255, 255, 255, 0.05);
    color: rgba(255, 255, 255, 0.9);
    font-size: 0.9rem;
    font-family: monospace;
  }

  .api-key-input input:focus {
    outline: none;
    border-color: #646cff;
  }

  .api-key-input input:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .api-key-input input::placeholder {
    color: rgba(255, 255, 255, 0.3);
  }

  .api-key-link {
    color: #646cff;
    font-size: 0.85rem;
    text-decoration: none;
    white-space: nowrap;
  }

  .api-key-link:hover {
    text-decoration: underline;
  }

  .language-row {
    display: flex;
    gap: 0.75rem;
    align-items: flex-end;
  }

  .language-selector {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }

  .language-selector label {
    font-weight: 500;
    color: rgba(255, 255, 255, 0.8);
    font-size: 0.85rem;
  }

  .language-arrow {
    color: rgba(255, 255, 255, 0.4);
    display: flex;
    align-items: center;
    padding-bottom: 0.35rem;
  }

  select {
    padding: 0.5rem;
    border-radius: 6px;
    border: 1px solid rgba(255, 255, 255, 0.2);
    background-color: rgba(255, 255, 255, 0.05);
    color: rgba(255, 255, 255, 0.9);
    font-size: 0.9rem;
    cursor: pointer;
  }

  select:hover:not(:disabled) {
    border-color: rgba(255, 255, 255, 0.3);
  }

  select:focus {
    outline: none;
    border-color: #646cff;
  }

  select:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Buttons */
  button {
    border-radius: 8px;
    border: 1px solid transparent;
    padding: 0.6em 1.2em;
    font-size: 1em;
    font-weight: 500;
    font-family: inherit;
    background-color: #1a1a1a;
    cursor: pointer;
    transition: border-color 0.25s, background-color 0.25s;
  }

  button:hover:not(:disabled) {
    border-color: #646cff;
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  button.primary {
    background-color: #646cff;
    color: white;
  }

  button.primary:hover:not(:disabled) {
    background-color: #535bf2;
  }

  button.danger {
    background-color: #f44336;
    color: white;
  }

  button.danger:hover:not(:disabled) {
    background-color: #d32f2f;
  }

  /* Provisional text */
  .provisional-text {
    margin-top: 0.75rem;
    padding: 0.5rem 0.75rem;
    background-color: rgba(100, 108, 255, 0.05);
    border-radius: 6px;
    font-size: 0.9rem;
    color: rgba(255, 255, 255, 0.6);
    border-left: 3px solid rgba(100, 108, 255, 0.3);
  }

  .provisional-label {
    color: rgba(100, 108, 255, 0.7);
    font-weight: 500;
  }

  /* Status footer */
  .status-footer {
    margin-top: 1.5rem;
    display: flex;
    gap: 2rem;
    font-size: 0.9rem;
    color: rgba(255, 255, 255, 0.6);
  }

  .status-item strong {
    color: rgba(255, 255, 255, 0.7);
  }

  /* Model section (collapsible) */
  .model-section {
    margin-top: 2rem;
  }

  .model-section summary {
    cursor: pointer;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.7);
    padding: 0.5rem 0;
    font-size: 0.95rem;
    user-select: none;
  }

  .model-section summary:hover {
    color: rgba(255, 255, 255, 0.9);
  }

  @media (max-width: 768px) {
    .mode-options {
      flex-direction: column;
    }

    .language-row {
      flex-wrap: wrap;
    }

    .status-footer {
      flex-direction: column;
      gap: 0.5rem;
    }
  }
</style>
