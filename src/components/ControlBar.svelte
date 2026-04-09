<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';

  let {
    isTranslating = false,
    statusText = 'Ready',
    statusType = 'idle',
    isPinned = false,
    onToggleRecord,
    onOpenSettings,
    onClear,
    onTogglePin,
  }: {
    isTranslating?: boolean;
    statusText?: string;
    statusType?: 'idle' | 'recording' | 'error' | 'ready';
    isPinned?: boolean;
    onToggleRecord: () => void;
    onOpenSettings: () => void;
    onClear: () => void;
    onTogglePin: () => void;
  } = $props();

  const appWindow = getCurrentWindow();

  async function handleMinimize() {
    await appWindow.minimize();
  }

  async function handleClose() {
    await appWindow.close();
  }

  function statusColor(): string {
    if (statusType === 'recording') return 'var(--danger)';
    if (statusType === 'error') return 'var(--danger)';
    if (statusType === 'ready') return 'var(--success)';
    return 'var(--text-dim)';
  }
</script>

<div class="control-bar">
  <!-- Left: Gear + Status -->
  <div class="bar-left">
    <button class="btn-icon" onclick={onOpenSettings} title="Settings">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="12" cy="12" r="3"/>
        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
      </svg>
    </button>

    <span class="status-dot" style="background: {statusColor()}"></span>
    <span class="status-text" data-tauri-drag-region>{statusText}</span>
  </div>

  <!-- Center: drag region spacer -->
  <div class="bar-center" data-tauri-drag-region></div>

  <!-- Right: Actions -->
  <div class="bar-right">
    <button
      class="btn-icon record-btn"
      class:recording={isTranslating}
      onclick={onToggleRecord}
      title={isTranslating ? 'Stop' : 'Start'}
    >
      {#if isTranslating}
        <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
          <rect x="6" y="6" width="12" height="12" rx="2"/>
        </svg>
      {:else}
        <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
          <circle cx="12" cy="12" r="8"/>
        </svg>
      {/if}
    </button>

    <button class="btn-icon" onclick={onClear} title="Clear">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <polyline points="3 6 5 6 21 6"/>
        <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>
      </svg>
    </button>

    <button class="btn-icon" class:active={isPinned} onclick={onTogglePin} title="Always on top">
      <svg width="14" height="14" viewBox="0 0 24 24" fill={isPinned ? 'currentColor' : 'none'} stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M12 17v5"/>
        <path d="M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V17h14v-1.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V6h1a2 2 0 0 0 0-4H8a2 2 0 0 0 0 4h1v4.76z"/>
      </svg>
    </button>

    <button class="btn-icon" onclick={handleMinimize} title="Minimize">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <line x1="5" y1="12" x2="19" y2="12"/>
      </svg>
    </button>

    <button class="btn-icon danger" onclick={handleClose} title="Close">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
        <line x1="18" y1="6" x2="6" y2="18"/>
        <line x1="6" y1="6" x2="18" y2="18"/>
      </svg>
    </button>
  </div>
</div>

<style>
  .control-bar {
    display: flex;
    align-items: center;
    height: 42px;
    padding: 0 var(--space-sm);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    user-select: none;
    -webkit-user-select: none;
  }

  .bar-left {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: 0 var(--space-xs);
    flex-shrink: 0;
  }

  .bar-center {
    flex: 1;
    height: 100%;
  }

  .bar-right {
    display: flex;
    align-items: center;
    gap: 2px;
    flex-shrink: 0;
  }

  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .status-text {
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 120px;
  }

  .record-btn {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    color: var(--accent);
  }

  .record-btn:hover {
    background: var(--accent-dim);
  }

  .record-btn.recording {
    color: var(--danger);
    animation: recording-pulse 1.5s ease-in-out infinite;
  }

  .record-btn.recording:hover {
    background: var(--danger-dim);
  }

  .btn-icon.active {
    color: var(--accent);
  }
</style>
