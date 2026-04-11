<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import type { AudioSource } from '../types';

  interface PlatformInfo {
    os: string;
    system_audio_available: boolean;
    offline_mode_available: boolean;
  }

  let {
    isTranslating = false,
    statusText = 'Ready',
    statusType = 'idle',
    isPinned = false,
    audioSource = 'microphone',
    ttsEnabled = false,
    platformInfo = null as PlatformInfo | null,
    onToggleRecord,
    onOpenSettings,
    onClear,
    onTogglePin,
    onSetAudioSource,
    onToggleTts,
  }: {
    isTranslating?: boolean;
    statusText?: string;
    statusType?: 'idle' | 'recording' | 'error' | 'ready';
    isPinned?: boolean;
    audioSource?: AudioSource;
    ttsEnabled?: boolean;
    platformInfo?: PlatformInfo | null;
    onToggleRecord: () => void;
    onOpenSettings: () => void;
    onClear: () => void;
    onTogglePin: () => void;
    onSetAudioSource: (source: AudioSource) => void;
    onToggleTts: () => void;
  } = $props();

  let systemAudioAvailable = $derived(platformInfo?.system_audio_available ?? true);

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
  <div class="bar-section">
    <button class="bar-btn" onclick={onOpenSettings} title="Open settings">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="12" cy="12" r="3"/>
        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
      </svg>
    </button>
    <span class="status-dot" style="background: {statusColor()}"></span>
    <span class="status-text" data-tauri-drag-region>{statusText}</span>
  </div>

  <!-- Center: Main action buttons -->
  <div class="bar-section center" data-tauri-drag-region>
    <div class="src-group">
      <button class="src-btn" class:active={audioSource === 'microphone'} onclick={() => onSetAudioSource('microphone')} title="Microphone">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z"/>
          <path d="M19 10v2a7 7 0 0 1-14 0v-2"/>
          <line x1="12" y1="19" x2="12" y2="23"/>
          <line x1="8" y1="23" x2="16" y2="23"/>
        </svg>
      </button>
      {#if systemAudioAvailable}
        <button class="src-btn" class:active={audioSource === 'system'} onclick={() => onSetAudioSource('system')} title="System audio">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"/>
            <path d="M19.07 4.93a10 10 0 0 1 0 14.14"/>
            <path d="M15.54 8.46a5 5 0 0 1 0 7.07"/>
          </svg>
        </button>
        <button class="src-btn" class:active={audioSource === 'both'} onclick={() => onSetAudioSource('both')} title="Mic + system">
          <svg viewBox="0 0 28 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M9 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z"/>
            <path d="M16 10v2a7 7 0 0 1-14 0v-2"/>
            <line x1="9" y1="19" x2="9" y2="23"/>
            <line x1="5" y1="23" x2="13" y2="23"/>
            <path d="M19 9.5a3.5 3.5 0 0 1 0 5" opacity="0.55"/>
            <path d="M23 6.5a7 7 0 0 1 0 11" opacity="0.55"/>
          </svg>
        </button>
      {/if}
    </div>

    <button class="start-btn" class:recording={isTranslating} onclick={onToggleRecord} title={isTranslating ? 'Stop recording' : 'Start recording'}>
      {#if isTranslating}
        <svg viewBox="0 0 24 24" fill="white"><rect x="6" y="6" width="12" height="12" rx="2"/></svg>
      {:else}
        <svg viewBox="0 0 24 24" fill="white"><polygon points="9 5 21 12 9 19"/></svg>
      {/if}
    </button>

    <button class="tts-btn" class:active={ttsEnabled} onclick={onToggleTts} title={ttsEnabled ? 'Disable text-to-speech' : 'Enable text-to-speech'}>
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"/>
        <path d="M15.54 8.46a5 5 0 0 1 0 7.07"/>
      </svg>
      <span class="tts-label">TTS</span>
    </button>

    <div class="bar-sep"></div>

    <button class="bar-btn" onclick={onClear} title="Clear transcript">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <polyline points="3 6 5 6 21 6"/>
        <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>
      </svg>
    </button>
  </div>

  <!-- Right: Window controls -->
  <div class="bar-section right">
    <button class="bar-btn" class:active={isPinned} onclick={onTogglePin} title={isPinned ? 'Unpin from top' : 'Pin to top'}>
      <svg viewBox="0 0 24 24" fill={isPinned ? 'currentColor' : 'none'} stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M12 17v5"/>
        <path d="M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V17h14v-1.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V6h1a2 2 0 0 0 0-4H8a2 2 0 0 0 0 4h1v4.76z"/>
      </svg>
    </button>
    <button class="bar-btn" onclick={handleMinimize} title="Minimize window">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="5" y1="12" x2="19" y2="12"/></svg>
    </button>
    <button class="bar-btn bar-btn-close" onclick={handleClose} title="Close application">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
        <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
      </svg>
    </button>
  </div>
</div>

<style>
  .control-bar {
    display: flex;
    align-items: center;
    height: 44px;
    padding: 0 8px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    user-select: none;
    -webkit-user-select: none;
  }

  .bar-section {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .bar-section.center {
    flex: 1;
    justify-content: center;
    gap: 8px;
    /* Allow drag on center area but not on interactive children */
    pointer-events: auto;
  }

  /* Make children clickable within drag region */
  .bar-section.center > * {
    pointer-events: auto;
  }

  .bar-section.right {
    gap: 2px;
  }

  .bar-sep {
    width: 1px;
    height: 18px;
    background: rgba(255, 255, 255, 0.08);
  }

  /* Status */
  .status-dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .status-text {
    font-size: 11px;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100px;
    font-weight: 500;
  }

  /* Shared bar button */
  .bar-btn {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    border: none;
    background: transparent;
    color: var(--text-dim);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: color 0.2s, background 0.2s;
    flex-shrink: 0;
    padding: 0;
  }

  .bar-btn svg { width: 14px; height: 14px; }
  .bar-btn:hover { color: var(--text-primary); background: rgba(255, 255, 255, 0.06); }
  .bar-btn.active { color: var(--accent); }
  .bar-btn-close:hover { color: var(--danger); background: rgba(255, 77, 77, 0.1); }

  /* Audio source group */
  .src-group {
    display: flex;
    align-items: center;
    background: rgba(255, 255, 255, 0.04);
    border-radius: 10px;
    padding: 3px;
    gap: 2px;
    flex-shrink: 0;
  }

  .src-btn {
    width: 34px;
    height: 30px;
    border-radius: 7px;
    border: none;
    background: transparent;
    color: var(--text-dim);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: color 0.2s, background 0.2s;
    flex-shrink: 0;
    padding: 0;
  }

  .src-btn svg { width: 15px; height: 15px; }
  .src-btn:hover { color: var(--text-secondary); }
  .src-btn.active { color: white; background: var(--accent); box-shadow: 0 1px 4px rgba(99, 140, 255, 0.3); }
  .src-btn.active:hover { filter: brightness(1.1); }

  /* Start / Stop */
  .start-btn {
    width: 36px;
    height: 36px;
    border-radius: 10px;
    border: none;
    background: var(--accent);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.2s, transform 0.15s, box-shadow 0.2s;
    box-shadow: 0 2px 8px rgba(99, 140, 255, 0.35);
    flex-shrink: 0;
    padding: 0;
  }

  .start-btn svg { width: 15px; height: 15px; }
  .start-btn:hover { transform: scale(1.08); box-shadow: 0 3px 12px rgba(99, 140, 255, 0.4); }
  .start-btn:active { transform: scale(0.95); }
  .start-btn.recording { background: var(--danger); box-shadow: 0 2px 8px rgba(255, 77, 77, 0.35); animation: pulse 2s ease-in-out infinite; }
  .start-btn.recording:hover { box-shadow: 0 3px 12px rgba(255, 77, 77, 0.4); }

  /* TTS */
  .tts-btn {
    height: 30px;
    padding: 0 10px;
    border-radius: 7px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(255, 255, 255, 0.03);
    color: var(--text-dim);
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 4px;
    font-family: var(--font-family);
    transition: all 0.2s ease;
    flex-shrink: 0;
  }

  .tts-btn svg { width: 13px; height: 13px; }
  .tts-label { font-size: 10px; font-weight: 600; letter-spacing: 0.5px; }
  .tts-btn:hover { border-color: rgba(255, 255, 255, 0.15); color: var(--text-secondary); }
  .tts-btn.active { background: rgba(99, 140, 255, 0.12); border-color: rgba(99, 140, 255, 0.4); color: var(--accent); }
  .tts-btn.active:hover { background: rgba(99, 140, 255, 0.18); }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.75; }
  }
</style>
