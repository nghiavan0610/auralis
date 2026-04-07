<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  interface ModelStatus {
    stt_available: boolean;
    stt_model: string;
    translation_available: boolean;
    translation_model: string;
    vad_available: boolean;
    vad_model: string;
    system_ready: boolean;
  }

  interface ModelDownloadProgress {
    model: string;
    progress: number;
    status: 'idle' | 'downloading' | 'completed' | 'error';
    error?: string;
  }

  let modelStatus: ModelStatus | null = null;
  let downloadProgress: ModelDownloadProgress[] = [
    { model: 'Whisper', progress: 0, status: 'idle' },
    { model: 'MADLAD', progress: 0, status: 'idle' },
    { model: 'Silero', progress: 0, status: 'idle' },
  ];

  let isDownloading = false;

  onMount(async () => {
    await refreshModelStatus();
  });

  async function refreshModelStatus() {
    try {
      modelStatus = await invoke<ModelStatus>('get_model_status');
      updateDownloadProgressFromStatus();
    } catch (error) {
      console.error('Failed to get model status:', error);
    }
  }

  function updateDownloadProgressFromStatus() {
    if (!modelStatus) return;

    downloadProgress = downloadProgress.map((item) => {
      if (item.model === 'Whisper') {
        return {
          ...item,
          status: modelStatus!.stt_available ? 'completed' : 'idle',
          progress: modelStatus!.stt_available ? 100 : 0,
        };
      } else if (item.model === 'MADLAD') {
        return {
          ...item,
          status: modelStatus!.translation_available ? 'completed' : 'idle',
          progress: modelStatus!.translation_available ? 100 : 0,
        };
      } else if (item.model === 'Silero') {
        return {
          ...item,
          status: modelStatus!.vad_available ? 'completed' : 'idle',
          progress: modelStatus!.vad_available ? 100 : 0,
        };
      }
      return item;
    });
  }

  async function downloadModel(modelName: string) {
    try {
      isDownloading = true;

      // Update status to downloading
      downloadProgress = downloadProgress.map((item) =>
        item.model === modelName ? { ...item, status: 'downloading' as const, progress: 0 } : item
      );

      // Simulate download progress (in real implementation, this would be from Tauri events)
      const interval = setInterval(() => {
        downloadProgress = downloadProgress.map((item) => {
          if (item.model === modelName && item.status === 'downloading') {
            const newProgress = Math.min(item.progress + 10, 100);
            if (newProgress >= 100) {
              clearInterval(interval);
              return { ...item, progress: 100, status: 'completed' as const };
            }
            return { ...item, progress: newProgress };
          }
          return item;
        });
      }, 500);

      // In a real implementation, you would call a Tauri command to download the model
      // and listen to progress events
      // await invoke('download_model', { modelName });

      // Refresh model status after "download" completes
      setTimeout(async () => {
        clearInterval(interval);
        await refreshModelStatus();
        isDownloading = false;
      }, 5500);

    } catch (error) {
      downloadProgress = downloadProgress.map((item) =>
        item.model === modelName
          ? { ...item, status: 'error' as const, error: String(error) }
          : item
      );
      isDownloading = false;
    }
  }

  function getStatusIcon(status: string) {
    switch (status) {
      case 'completed':
        return '✓';
      case 'downloading':
        return '⏳';
      case 'error':
        return '✗';
      default:
        return '○';
    }
  }

  function getStatusClass(status: string) {
    switch (status) {
      case 'completed':
        return 'status-success';
      case 'downloading':
        return 'status-downloading';
      case 'error':
        return 'status-error';
      default:
        return 'status-idle';
    }
  }
</script>

<div class="model-downloader">
  <h3>Model Status</h3>

  {#if modelStatus && modelStatus.system_ready}
    <div class="system-status ready">
      <span class="status-dot"></span>
      <span>All models downloaded and ready</span>
    </div>
  {:else}
    <div class="system-status pending">
      <span class="status-dot"></span>
      <span>Download required models to start</span>
    </div>
  {/if}

  <div class="models-list">
    {#each downloadProgress as item}
      <div class="model-item">
        <div class="model-info">
          <div class="model-header">
            <span class="model-icon {getStatusClass(item.status)}">{getStatusIcon(item.status)}</span>
            <span class="model-name">{item.model}</span>
          </div>
          <div class="model-description">
            {#if item.model === 'Whisper'}
              Speech-to-text engine for transcription
            {:else if item.model === 'MADLAD'}
              Neural machine translation model
            {:else if item.model === 'Silero'}
              Voice activity detection model
            {/if}
          </div>
          {#if item.error}
            <div class="model-error">{item.error}</div>
          {/if}
        </div>

        <div class="model-actions">
          {#if item.status === 'idle' || item.status === 'error'}
            <button
              on:click={() => downloadModel(item.model)}
              disabled={isDownloading}
              class:download-btn={true}
            >
              Download
            </button>
          {:else if item.status === 'downloading'}
            <div class="progress-info">{item.progress}%</div>
          {:else if item.status === 'completed'}
            <div class="status-success">Ready</div>
          {/if}
        </div>

        {#if item.status === 'downloading'}
          <div class="progress-bar-container">
            <div class="progress-bar">
              <div class="progress-fill" style="width: {item.progress}%"></div>
            </div>
          </div>
        {/if}
      </div>
    {/each}
  </div>

  <div class="model-info">
    <p><strong>Note:</strong> Models are required for real-time translation. Download sizes:</p>
    <ul>
      <li>Whisper: ~1.5 GB</li>
      <li>MADLAD: ~2.8 GB</li>
      <li>Silero: ~65 MB</li>
    </ul>
    <p><strong>Total:</strong> ~4.4 GB</p>
  </div>
</div>

<style>
  .model-downloader {
    margin-top: 2rem;
    padding: 1.5rem;
    background-color: rgba(255, 255, 255, 0.03);
    border-radius: 8px;
  }

  h3 {
    font-size: 1.25rem;
    margin-bottom: 1rem;
    color: rgba(255, 255, 255, 0.9);
  }

  .system-status {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    border-radius: 6px;
    margin-bottom: 1rem;
    font-weight: 500;
  }

  .system-status.ready {
    background-color: rgba(76, 175, 80, 0.2);
    color: #4caf50;
    border: 1px solid #4caf50;
  }

  .system-status.pending {
    background-color: rgba(255, 152, 0, 0.2);
    color: #ff9800;
    border: 1px solid #ff9800;
  }

  .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background-color: currentColor;
    animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
  }

  @keyframes pulse {
    0%, 100% {
      opacity: 1;
    }
    50% {
      opacity: 0.5;
    }
  }

  .models-list {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .model-item {
    padding: 1rem;
    background-color: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    position: relative;
  }

  .model-info {
    flex: 1;
  }

  .model-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
  }

  .model-icon {
    font-size: 1.2rem;
    width: 24px;
    text-align: center;
  }

  .model-icon.status-success {
    color: #4caf50;
  }

  .model-icon.status-error {
    color: #f44336;
  }

  .model-icon.status-downloading {
    color: #ff9800;
  }

  .model-icon.status-idle {
    color: rgba(255, 255, 255, 0.4);
  }

  .model-name {
    font-weight: 600;
    font-size: 1rem;
    color: rgba(255, 255, 255, 0.9);
  }

  .model-description {
    font-size: 0.85rem;
    color: rgba(255, 255, 255, 0.6);
    margin-bottom: 0.25rem;
  }

  .model-error {
    color: #f44336;
    font-size: 0.85rem;
    margin-top: 0.5rem;
  }

  .model-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    margin-top: 0.75rem;
  }

  .progress-info {
    font-weight: 500;
    color: #ff9800;
  }

  .status-success {
    color: #4caf50;
    font-weight: 500;
  }

  .progress-bar-container {
    margin-top: 1rem;
  }

  .progress-bar {
    width: 100%;
    height: 6px;
    background-color: rgba(255, 255, 255, 0.1);
    border-radius: 3px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background-color: #646cff;
    transition: width 0.3s ease;
  }

  button.download-btn {
    padding: 0.5rem 1rem;
    border-radius: 6px;
    border: 1px solid #646cff;
    background-color: transparent;
    color: #646cff;
    font-size: 0.9rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  button.download-btn:hover:not(:disabled) {
    background-color: #646cff;
    color: white;
  }

  button.download-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .model-info {
    margin-top: 1.5rem;
    padding: 1rem;
    background-color: rgba(255, 255, 255, 0.02);
    border-radius: 6px;
    font-size: 0.9rem;
    color: rgba(255, 255, 255, 0.7);
  }

  .model-info p {
    margin-bottom: 0.5rem;
  }

  .model-info ul {
    margin-left: 1.5rem;
    margin-bottom: 0.5rem;
  }

  .model-info li {
    margin-bottom: 0.25rem;
  }
</style>
