<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  let {
    progress = $bindable(0),
    progressMessage = $bindable(''),
    progressStep = $bindable(''),
    offlineReady = $bindable(false),
  }: {
    progress?: number;
    progressMessage?: string;
    progressStep?: string;
    offlineReady?: boolean;
  } = $props();

  let checking = $state(true);
  let setting = $state(false);
  let errorMsg = $state('');

  async function checkReady(): Promise<void> {
    checking = true;
    try {
      const result = await invoke<{ venv_exists: boolean; packages_installed: boolean; ready: boolean }>('check_offline_ready');
      offlineReady = result.ready;
    } catch {
      offlineReady = false;
    }
    checking = false;
  }

  async function handleSetup(): Promise<void> {
    setting = true;
    progress = 0;
    progressMessage = 'Starting setup...';
    progressStep = '';
    errorMsg = '';

    try {
      await invoke('setup_offline_environment');
      offlineReady = true;
      progressMessage = 'Setup complete!';
    } catch (err) {
      errorMsg = `${err}`;
    }
    setting = false;
  }

  onMount(() => {
    checkReady();
  });
</script>

<div class="offline-setup">
  {#if checking}
    <div class="status-line">Checking offline environment...</div>
  {:else if offlineReady}
    <div class="status-line ready">
      <span class="dot green"></span>
      Offline mode is ready
    </div>
  {:else if setting}
    <div class="progress-section">
      <div class="progress-header">
        <span class="step-label">{progressMessage}</span>
        <span class="progress-pct">{progress}%</span>
      </div>
      <div class="progress-bar">
        <div class="progress-fill" style="width: {progress}%"></div>
      </div>
      {#if progressStep}
        <div class="step-detail">Step: {progressStep}</div>
      {/if}
      {#if errorMsg}
        <div class="error">{errorMsg}</div>
      {/if}
    </div>
  {:else}
    <div class="setup-prompt">
      <p>Offline mode requires downloading MLX Whisper and Gemma-3 LLM (~3 GB).</p>
      <button class="btn-primary" onclick={handleSetup}>
        Setup Offline Mode
      </button>
      {#if errorMsg}
        <div class="error">{errorMsg}</div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .offline-setup {
    padding: var(--space-lg);
    background: var(--bg-secondary);
    border-radius: var(--radius-md);
    border: 1px solid var(--border);
  }

  .status-line {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
  }

  .status-line.ready {
    color: var(--success);
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    display: inline-block;
  }

  .dot.green {
    background-color: var(--success);
  }

  .progress-section {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }

  .progress-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .step-label {
    font-size: var(--font-size-sm);
    color: var(--text-primary);
  }

  .progress-pct {
    font-size: var(--font-size-xs);
    color: var(--text-dim);
    font-family: monospace;
  }

  .progress-bar {
    height: 4px;
    background: var(--bg-active);
    border-radius: 2px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg, var(--accent), var(--accent-hover), var(--accent));
    background-size: 200% 100%;
    border-radius: 2px;
    transition: width 0.3s ease;
    animation: shimmer 2s linear infinite;
  }

  .step-detail {
    font-size: var(--font-size-xs);
    color: var(--text-dim);
    font-family: monospace;
  }

  .setup-prompt {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-md);
    text-align: center;
  }

  .setup-prompt p {
    margin: 0;
    font-size: var(--font-size-sm);
    color: var(--text-dim);
    max-width: 280px;
  }

  .error {
    color: var(--danger);
    font-size: var(--font-size-xs);
    padding: var(--space-sm) var(--space-md);
    background: var(--danger-dim);
    border-radius: var(--radius-sm);
    border: 1px solid rgba(255, 77, 77, 0.2);
    width: 100%;
    text-align: center;
  }
</style>
