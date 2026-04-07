<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import StatusIndicator from './components/StatusIndicator.svelte';
  import DualPanel from './components/DualPanel.svelte';
  import ModelDownloader from './components/ModelDownloader.svelte';

  // State
  let isTranslating = false;
  let isSystemReady = false;
  let sourceLanguage = 'en';
  let targetLanguage = 'vi';

  // Event data
  interface STTSegment {
    text: string;
    confidence: number;
    start: number;
    end: number;
    is_final: boolean;
  }

  interface Translation {
    source_lang: string;
    target_lang: string;
    source_text: string;
    translated_text: string;
    score: number;
  }

  interface AuralisEvent {
    STTResult?: { segment: STTSegment; language: string };
    TranslationResult?: { translation: Translation };
    Error?: { component: string; message: string };
    SpeechActivityChanged?: { is_speech: boolean; probability: number };
    AudioCaptureChanged?: { is_capturing: boolean };
    StatusUpdate?: { component: string; status: string };
  }

  let transcriptions: Array<{ segment: STTSegment; language: string }> = [];
  let translations: Array<{ translation: Translation }> = [];
  let speechActivity = { is_speech: false, probability: 0 };
  let audioCapturing = false;
  let statusMessage = 'Initializing...';
  let errorMessage = '';

  // Event listeners cleanup
  const unlisteners: Array<() => void> = [];

  onMount(async () => {
    try {
      // Check initial system status
      await checkModelStatus();

      // Get current languages
      const [source, target] = await invoke<[string, string]>('get_languages');
      sourceLanguage = source;
      targetLanguage = target;

      // Set up event listeners
      const sttUnlisten = await listen<AuralisEvent>('stt-result', (event) => {
        if (event.payload.STTResult) {
          transcriptions = [
            ...transcriptions,
            {
              segment: event.payload.STTResult.segment,
              language: event.payload.STTResult.language,
            },
          ];
          // Keep only last 50 transcriptions
          if (transcriptions.length > 50) {
            transcriptions = transcriptions.slice(-50);
          }
        }
      });

      const translationUnlisten = await listen<AuralisEvent>('translation-result', (event) => {
        if (event.payload.TranslationResult) {
          translations = [
            ...translations,
            { translation: event.payload.TranslationResult.translation },
          ];
          // Keep only last 50 translations
          if (translations.length > 50) {
            translations = translations.slice(-50);
          }
        }
      });

      const errorUnlisten = await listen<AuralisEvent>('error', (event) => {
        if (event.payload.Error) {
          errorMessage = `${event.payload.Error.component}: ${event.payload.Error.message}`;
          statusMessage = 'Error occurred';
        }
      });

      const speechUnlisten = await listen<AuralisEvent>('speech-activity', (event) => {
        if (event.payload.SpeechActivityChanged) {
          speechActivity = {
            is_speech: event.payload.SpeechActivityChanged.is_speech,
            probability: event.payload.SpeechActivityChanged.probability,
          };
        }
      });

      const audioUnlisten = await listen<AuralisEvent>('audio-capture', (event) => {
        if (event.payload.AudioCaptureChanged) {
          audioCapturing = event.payload.AudioCaptureChanged.is_capturing;
        }
      });

      const statusUnlisten = await listen<AuralisEvent>('status-update', (event) => {
        if (event.payload.StatusUpdate) {
          statusMessage = `${event.payload.StatusUpdate.component}: ${event.payload.StatusUpdate.status}`;
        }
      });

      unlisteners.push(
        sttUnlisten,
        translationUnlisten,
        errorUnlisten,
        speechUnlisten,
        audioUnlisten,
        statusUnlisten
      );

      statusMessage = 'Ready to start';
    } catch (error) {
      errorMessage = `Failed to initialize: ${error}`;
      statusMessage = 'Initialization failed';
    }
  });

  onDestroy(() => {
    // Clean up event listeners
    unlisteners.forEach((unlisten) => unlisten());
  });

  async function checkModelStatus() {
    try {
      const status = await invoke('get_model_status');
      isSystemReady = status.system_ready;
    } catch (error) {
      console.error('Failed to check model status:', error);
    }
  }

  async function handleStart() {
    try {
      errorMessage = '';
      const result = await invoke<string>('start_translation');
      statusMessage = result;
      isTranslating = true;
    } catch (error) {
      errorMessage = `Failed to start: ${error}`;
      statusMessage = 'Start failed';
    }
  }

  async function handleStop() {
    try {
      const result = await invoke<string>('stop_translation');
      statusMessage = result;
      isTranslating = false;
    } catch (error) {
      errorMessage = `Failed to stop: ${error}`;
      statusMessage = 'Stop failed';
    }
  }

  async function handleSourceLanguageChange(event: Event) {
    const target = event.target as HTMLSelectElement;
    sourceLanguage = target.value;
    try {
      await invoke('set_source_language', { language: sourceLanguage });
    } catch (error) {
      errorMessage = `Failed to set source language: ${error}`;
    }
  }

  async function handleTargetLanguageChange(event: Event) {
    const target = event.target as HTMLSelectElement;
    targetLanguage = target.value;
    try {
      await invoke('set_target_language', { language: targetLanguage });
    } catch (error) {
      errorMessage = `Failed to set target language: ${error}`;
    }
  }

  function getStatusIndicator() {
    if (errorMessage) return 'error';
    if (isTranslating) return 'processing';
    if (isSystemReady) return 'ready';
    return 'idle';
  }

  function getStatusText() {
    if (errorMessage) return 'Error';
    if (isTranslating) return 'Translating...';
    if (isSystemReady) return 'Ready';
    return 'Not Ready';
  }
</script>

<div class="container">
  <header>
    <h1>Auralis</h1>
    <p>Real-time Speech Translation System</p>
  </header>

  <div style="margin-top: 2rem;">
    <StatusIndicator
      status={getStatusIndicator()}
      text={getStatusText()}
    />
    {#if errorMessage}
      <div style="margin-top: 1rem; color: #f44336; padding: 0.75rem; background-color: rgba(244, 67, 54, 0.1); border-radius: 6px;">
        {errorMessage}
      </div>
    {/if}
  </div>

  <ModelDownloader />

  <div class="controls">
    <div class="language-selector">
      <label for="source-language">Source:</label>
      <select id="source-language" value={sourceLanguage} on:change={handleSourceLanguageChange} disabled={isTranslating}>
        <option value="en">English</option>
        <option value="vi">Vietnamese</option>
        <option value="es">Spanish</option>
        <option value="fr">French</option>
        <option value="de">German</option>
        <option value="zh">Chinese</option>
        <option value="ja">Japanese</option>
      </select>
    </div>

    <div class="language-selector">
      <label for="target-language">Target:</label>
      <select id="target-language" value={targetLanguage} on:change={handleTargetLanguageChange} disabled={isTranslating}>
        <option value="en">English</option>
        <option value="vi">Vietnamese</option>
        <option value="es">Spanish</option>
        <option value="fr">French</option>
        <option value="de">German</option>
        <option value="zh">Chinese</option>
        <option value="ja">Japanese</option>
      </select>
    </div>

    <div style="flex: 1;"></div>

    {#if !isTranslating}
      <button
        class="primary"
        on:click={handleStart}
        disabled={!isSystemReady}
      >
        Start Translation
      </button>
    {:else}
      <button
        class="danger"
        on:click={handleStop}
      >
        Stop Translation
      </button>
    {/if}
  </div>

  <DualPanel
    {sourceLanguage}
    {targetLanguage}
    {transcriptions}
    {translations}
  />

  <div style="margin-top: 2rem; display: flex; gap: 2rem; font-size: 0.9rem; color: rgba(255,255,255,0.6);">
    <div>
      <strong>Speech Activity:</strong> {speechActivity.is_speech ? 'Active' : 'Inactive'}
      ({(speechActivity.probability * 100).toFixed(0)}%)
    </div>
    <div>
      <strong>Audio:</strong> {audioCapturing ? 'Capturing' : 'Idle'}
    </div>
    <div>
      <strong>Status:</strong> {statusMessage}
    </div>
  </div>
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

  .controls {
    display: flex;
    gap: 1rem;
    margin-top: 1rem;
    padding: 1rem;
    background-color: rgba(255, 255, 255, 0.03);
    border-radius: 8px;
  }

  .language-selector {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  label {
    font-weight: 500;
    color: rgba(255, 255, 255, 0.8);
  }

  button {
    border-radius: 8px;
    border: 1px solid transparent;
    padding: 0.6em 1.2em;
    font-size: 1em;
    font-weight: 500;
    font-family: inherit;
    background-color: #1a1a1a;
    cursor: pointer;
    transition: border-color 0.25s;
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
</style>
