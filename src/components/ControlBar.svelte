<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import type { AudioSource } from '../types';
  import { getLangLabel, getLangFlag } from '../js/lang';
  import { onDestroy } from 'svelte';
  import Tooltip from './Tooltip.svelte';

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
    updateAvailable = false,
    platformInfo = null as PlatformInfo | null,
    sourceLanguage = 'en',
    targetLanguage = 'vi',
    mode = 'cloud',
    activeAudioSources = [],
    onToggleRecord,
    onOpenSettings,
    onClear,
    onTogglePin,
    onSetAudioSource,
    onToggleTts,
    onOpenSaved,
    onShowShortcuts,
    onShowLanguageSelector,
    onShowModeSelector,
    onShowTtsSelector,
  }: {
    isTranslating?: boolean;
    statusText?: string;
    statusType?: 'idle' | 'recording' | 'error' | 'ready';
    isPinned?: boolean;
    audioSource?: AudioSource;
    ttsEnabled?: boolean;
    updateAvailable?: boolean;
    platformInfo?: PlatformInfo | null;
    sourceLanguage?: string;
    targetLanguage?: string;
    mode?: 'cloud' | 'offline';
    activeAudioSources?: AudioSource[];
    onToggleRecord: () => void;
    onOpenSettings: () => void;
    onClear: () => void;
    onTogglePin: () => void;
    onSetAudioSource: (source: AudioSource) => void;
    onToggleTts: () => void;
    onOpenSaved: () => void;
    onShowShortcuts?: () => void;
    onShowLanguageSelector?: () => void;
    onShowModeSelector?: () => void;
    onShowTtsSelector?: () => void;
  } = $props();

  let systemAudioAvailable = $derived(platformInfo?.system_audio_available ?? true);

  // Recording timer state
  let recordingTime = $state(0);
  let timerInterval: ReturnType<typeof setInterval> | null = null;

  // Audio level visualization state (simulated for now)
  let audioLevels = $state([0, 0, 0, 0, 0]);
  let audioLevelInterval: ReturnType<typeof setInterval> | null = null;

  // Format seconds to MM:SS
  function formatTime(seconds: number): string {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
  }

  // Update timer when recording state changes
  $effect(() => {
    if (isTranslating) {
      // Start or continue timer
      if (!timerInterval) {
        timerInterval = setInterval(() => {
          recordingTime++;
        }, 1000);
      }

      // Start audio level simulation
      if (!audioLevelInterval) {
        audioLevelInterval = setInterval(() => {
          // Simulate audio levels with random values
          audioLevels = audioLevels.map(() => Math.random() * 100);
        }, 100);
      }
    } else {
      // Stop timer and reset
      if (timerInterval) {
        clearInterval(timerInterval);
        timerInterval = null;
      }
      recordingTime = 0;

      // Stop audio levels
      if (audioLevelInterval) {
        clearInterval(audioLevelInterval);
        audioLevelInterval = null;
      }
      audioLevels = [0, 0, 0, 0, 0];
    }

    return () => {
      if (timerInterval) {
        clearInterval(timerInterval);
      }
      if (audioLevelInterval) {
        clearInterval(audioLevelInterval);
      }
    };
  });

  onDestroy(() => {
    if (timerInterval) {
      clearInterval(timerInterval);
    }
  });

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

  function handleSettingsClick() {
    console.log('[ControlBar] Settings clicked');
    onOpenSettings();
  }
</script>

<div class="control-bar">
  <!-- Left: Gear + Status -->
  <div class="bar-section">
    {#if onShowShortcuts}
      <Tooltip content="Keyboard shortcuts (?)" position="bottom">
        <button class="bar-btn shortcuts-btn" onclick={onShowShortcuts}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="2" y="4" width="20" height="16" rx="2"/>
            <path d="M6 10h.01M10 10h.01M14 10h.01M18 10h.01M6 14h.01M10 14h.01M14 14h.01M18 14h.01"/>
          </svg>
        </button>
      </Tooltip>
    {/if}
    <button class="bar-btn gear-btn" onclick={() => { console.log('[ControlBar] Settings clicked'); onOpenSettings(); }} title="Settings (⌘,)">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="12" cy="12" r="3"/>
        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
      </svg>
      {#if updateAvailable}
        <span class="update-badge"></span>
      {/if}
    </button>
    <span class="status-dot" style="background: {statusColor()}"></span>
    <span class="status-text" data-tauri-drag-region>{statusText}</span>
    <Tooltip content={`${getLangLabel(sourceLanguage)} → ${getLangLabel(targetLanguage)} (Click to change)`}>
      <div class="lang-indicator" onclick={() => onShowLanguageSelector ? onShowLanguageSelector() : onOpenSettings()}>
      <span class="lang-flag">{getLangFlag(sourceLanguage)}</span>
      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="var(--text-dim)" stroke-width="2">
        <path d="M5 12h14M12 5l7 7-7 7"/>
      </svg>
      <span class="lang-flag">{getLangFlag(targetLanguage)}</span>
      </div>
    </Tooltip>
    <Tooltip content={mode === 'cloud' ? 'Cloud mode (Click to change)' : 'Offline mode (Click to change)'}>
      <div class="mode-indicator" class:cloud={mode === 'cloud'} class:offline={mode === 'offline'} onclick={() => onShowModeSelector ? onShowModeSelector() : onOpenSettings()}>
      {#if mode === 'cloud'}
        <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M18 10h-1.26A8 8 0 1 0 9 20h9a5 5 0 0 0 0-10z"/>
        </svg>
        <span class="mode-label">Cloud</span>
      {:else}
        <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <rect x="3" y="3" width="18" height="18" rx="2"/>
        </svg>
        <span class="mode-label">Offline</span>
      {/if}
      </div>
    </Tooltip>
    {#if isTranslating}
      <Tooltip content="Recording duration: {formatTime(recordingTime)}">
        <div class="recording-timer">
          <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10"/>
            <polyline points="12 6 12 12 12 18"/>
          </svg>
          <span>{formatTime(recordingTime)}</span>
        </div>
      </Tooltip>
    {/if}
  </div>

  <!-- Center: Main action buttons (without start button) -->
  <div class="bar-section center" data-tauri-drag-region>
    <div class="src-group">
      <Tooltip content="Microphone input" position="top">
        <button class="src-btn" class:active={audioSource === 'microphone'} onclick={() => onSetAudioSource('microphone')}>
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z"/>
          <path d="M19 10v2a7 7 0 0 1-14 0v-2"/>
          <line x1="12" y1="19" x2="12" y2="23"/>
          <line x1="8" y1="23" x2="16" y2="23"/>
        </svg>
        {#if activeAudioSources.includes('microphone')}
          <span class="audio-active-indicator"></span>
        {/if}
        </button>
      </Tooltip>
      {#if systemAudioAvailable}
        <Tooltip content="System audio" position="top">
          <button class="src-btn" class:active={audioSource === 'system'} onclick={() => onSetAudioSource('system')}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"/>
            <path d="M19.07 4.93a10 10 0 0 1 0 14.14"/>
            <path d="M15.54 8.46a5 5 0 0 1 0 7.07"/>
          </svg>
          {#if activeAudioSources.includes('system')}
            <span class="audio-active-indicator"></span>
          {/if}
          </button>
        </Tooltip>
        <Tooltip content="Microphone + System audio" position="top">
          <button class="src-btn" class:active={audioSource === 'both'} onclick={() => onSetAudioSource('both')}>
          <svg viewBox="0 0 28 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <path d="M9 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z"/>
            <path d="M16 10v2a7 7 0 0 1-14 0v-2"/>
            <line x1="9" y1="19" x2="9" y2="23"/>
            <line x1="5" y1="23" x2="13" y2="23"/>
            <path d="M19 9.5a3.5 3.5 0 0 1 0 5" opacity="0.55"/>
            <path d="M23 6.5a7 7 0 0 1 0 11" opacity="0.55"/>
          </svg>
          {#if activeAudioSources.includes('microphone') || activeAudioSources.includes('system')}
            <span class="audio-active-indicator"></span>
          {/if}
          </button>
        </Tooltip>
      {/if}
    </div>

  </div>

  <!-- Start button (absolutely centered on control bar) -->
  <div class="start-btn-wrapper">
    <Tooltip content={isTranslating ? 'Stop recording (Space)' : 'Start recording (Space)'} position="top">
      <button class="start-btn" class:recording={isTranslating} onclick={onToggleRecord}>
      {#if isTranslating}
        <svg viewBox="0 0 24 24" fill="white"><rect x="6" y="6" width="12" height="12" rx="2"/></svg>
      {:else}
        <svg viewBox="0 0 24 24" fill="white"><polygon points="9 5 21 12 9 19"/></svg>
      {/if}
      {#if isTranslating}
        <span class="recording-indicator"></span>
      {/if}
      </button>
    </Tooltip>
  </div>

  <!-- Right: Utility group + Window controls -->
  <div class="bar-section right">
    <div class="utility-group">
      <Tooltip content={ttsEnabled ? 'Disable text-to-speech (Click)' : 'Select TTS provider (Click)'} position="top">
        <button class="tts-btn" class:active={ttsEnabled} onclick={() => ttsEnabled ? onToggleTts() : (onShowTtsSelector ? onShowTtsSelector() : onToggleTts())}>
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"/>
          <path d="M15.54 8.46a5 5 0 0 1 0 7.07"/>
        </svg>
        <span class="tts-label">TTS</span>
        </button>
      </Tooltip>

      <Tooltip content="View saved transcripts" position="top">
        <button class="bar-btn" onclick={() => { console.log('[ControlBar] Saved transcripts clicked'); onOpenSaved(); }}>
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="10"/>
          <polyline points="12 6 12 12 16 14"/>
        </svg>
        </button>
      </Tooltip>

      <Tooltip content="Clear current transcript" position="top">
        <button class="bar-btn" onclick={onClear}>
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="3 6 5 6 21 6"/>
          <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>
        </svg>
        </button>
      </Tooltip>
    </div>

    {#if isTranslating}
      <Tooltip content="Audio input level" position="top">
        <div class="audio-levels">
        {#each audioLevels as level, i}
          <div class="audio-bar" style="height: {Math.max(4, level * 0.16)}px;"></div>
        {/each}
        </div>
      </Tooltip>
    {/if}
    <Tooltip content={isPinned ? 'Unpin from top' : 'Pin to top'} position="left">
      <button class="bar-btn" class:active={isPinned} onclick={onTogglePin}>
      <svg viewBox="0 0 24 24" fill={isPinned ? 'currentColor' : 'none'} stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M12 17v5"/>
        <path d="M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V17h14v-1.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V6h1a2 2 0 0 0 0-4H8a2 2 0 0 0 0 4h1v4.76z"/>
      </svg>
      </button>
    </Tooltip>
    <Tooltip content="Minimize window" position="left">
      <button class="bar-btn" onclick={handleMinimize}>
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="5" y1="12" x2="19" y2="12"/></svg>
      </button>
    </Tooltip>
    <Tooltip content="Close application" position="left">
      <button class="bar-btn bar-btn-close" onclick={handleClose}>
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
        <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
      </svg>
      </button>
    </Tooltip>
  </div>
</div>

<style>
  .control-bar {
    display: flex;
    align-items: center;
    height: 52px;
    padding: 0 12px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    user-select: none;
    -webkit-user-select: none;
    position: relative;
  }

  .bar-section {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
    pointer-events: auto;
  }

  .bar-section.center {
    flex: 1;
    justify-content: flex-end;
    align-items: center;
    gap: 20px;
  }

  /* Start button wrapper - absolutely centered on control bar */
  .start-btn-wrapper {
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    z-index: var(--z-header);
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
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
    box-shadow: 0 0 6px currentColor;
    animation: status-glow 2s ease-in-out infinite;
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

  /* Language indicator */
  .lang-indicator {
    display: flex;
    align-items: center;
    gap: 3px;
    padding: 3px 6px;
    margin-left: var(--space-xs);
    background: rgba(99, 140, 255, 0.08);
    border: 1px solid rgba(99, 140, 255, 0.15);
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .lang-indicator:hover {
    background: rgba(99, 140, 255, 0.15);
    border-color: rgba(99, 140, 255, 0.3);
  }

  .lang-flag {
    font-size: 10px;
  }

  /* Mode indicator */
  .mode-indicator {
    display: flex;
    align-items: center;
    gap: 3px;
    padding: 3px 6px;
    margin-left: var(--space-xs);
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .mode-indicator:hover {
    background: rgba(255, 255, 255, 0.08);
    border-color: rgba(255, 255, 255, 0.12);
  }

  .mode-label {
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.3px;
    text-transform: uppercase;
  }

  .mode-indicator.cloud {
    color: var(--text-dim);
  }

  .mode-indicator.cloud:hover {
    color: var(--text-secondary);
  }

  .mode-indicator.offline {
    color: var(--success);
  }

  .mode-indicator.offline:hover {
    color: var(--success);
    opacity: 0.8;
  }

  /* Recording timer */
  .recording-timer {
    display: flex;
    align-items: center;
    gap: 3px;
    padding: 3px 6px;
    margin-left: var(--space-xs);
    background: rgba(255, 77, 77, 0.1);
    border: 1px solid rgba(255, 77, 77, 0.2);
    border-radius: var(--radius-sm);
    color: var(--danger);
    font-size: 10px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    animation: pulse-border 2s ease-in-out infinite;
  }

  @keyframes pulse-border {
    0%, 100% {
      border-color: rgba(255, 77, 77, 0.2);
    }
    50% {
      border-color: rgba(255, 77, 77, 0.4);
    }
  }

  /* Audio level visualization */
  .audio-levels {
    display: flex;
    align-items: center;
    gap: 3px;
    padding: 0 8px;
    margin-left: var(--space-sm);
    height: 20px;
  }

  .audio-bar {
    width: 4px;
    background: var(--accent);
    border-radius: 2px;
    transition: height 0.1s ease;
  }

  /* Shared bar button (utility icon buttons) */
  .bar-btn {
    width: 30px;
    height: 30px;
    border-radius: 6px;
    border: none;
    background: transparent;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.05);
    color: var(--text-dim);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    flex-shrink: 0;
    padding: 0;
    position: relative;
    z-index: 1;
  }

  .bar-btn svg { width: 14px; height: 14px; }
  .bar-btn:hover {
    color: var(--text-primary);
    background: rgba(255, 255, 255, 0.06);
    transform: scale(1.05);
  }
  .bar-btn.active { color: var(--accent); }
  .bar-btn-close:hover { color: var(--danger); background: rgba(255, 77, 77, 0.1); }

  /* Update badge on gear icon */
  .gear-btn { position: relative; }
  .update-badge {
    position: absolute;
    top: 4px;
    right: 4px;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 4px rgba(99, 140, 255, 0.5);
    pointer-events: none;
  }

  /* Keyboard shortcuts button */
  .shortcuts-btn {
    opacity: 0.7;
  }
  .shortcuts-btn:hover {
    opacity: 1;
  }

  /* Audio source group */
  .src-group {
    display: flex;
    align-items: center;
    background: rgba(255, 255, 255, 0.06);
    backdrop-filter: blur(10px);
    border-radius: 10px;
    padding: 3px;
    gap: 3px;
    border: 1px solid rgba(255, 255, 255, 0.12);
    box-shadow:
      0 4px 12px rgba(0, 0, 0, 0.15),
      inset 0 1px 0 rgba(255, 255, 255, 0.1);
    flex-shrink: 0;
  }

  .src-btn {
    width: 36px;
    height: 36px;
    border-radius: 8px;
    border: none;
    background: transparent;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.05);
    color: var(--text-dim);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    flex-shrink: 0;
    padding: 0;
    position: relative;
  }

  .src-btn svg { width: 16px; height: 16px; }
  .src-btn:hover {
    color: var(--text-secondary);
    background: rgba(255, 255, 255, 0.04);
    transform: translateY(-1px);
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.15);
  }

  .src-btn.active {
    color: white;
    background: linear-gradient(135deg, var(--accent) 0%, #5a7fd4 100%);
    box-shadow:
      0 2px 8px rgba(99, 140, 255, 0.5),
      inset 0 1px 0 rgba(255, 255, 255, 0.2);
    transform: translateY(-1px);
  }

  .src-btn.active:hover {
    filter: brightness(1.1);
    box-shadow:
      0 3px 10px rgba(99, 140, 255, 0.6),
      inset 0 1px 0 rgba(255, 255, 255, 0.2);
  }

  /* Add subtle indicator for current selection */
  .src-btn.active::after {
    content: '';
    position: absolute;
    bottom: -2px;
    left: 50%;
    transform: translateX(-50%);
    width: 4px;
    height: 4px;
    background: white;
    border-radius: 50%;
    opacity: 0.8;
  }

  /* Start / Stop */
  .start-btn {
    width: 44px;
    height: 44px;
    border-radius: 12px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    background: linear-gradient(135deg, var(--accent) 0%, #5a7fd4 100%);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.2s, transform 0.15s, box-shadow 0.2s;
    box-shadow: 0 4px 16px rgba(99, 140, 255, 0.5);
    flex-shrink: 0;
    padding: 0;
    position: relative;
    overflow: hidden;
  }

  .start-btn::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 50%;
    background: linear-gradient(to bottom, rgba(255,255,255,0.15), transparent);
    border-radius: 12px 12px 0 0;
    opacity: 0;
    transition: opacity 0.2s;
  }

  .start-btn:hover::before {
    opacity: 1;
  }

  .start-btn svg { width: 17px; height: 17px; }
  .start-btn:hover { transform: scale(1.05); box-shadow: 0 5px 20px rgba(99, 140, 255, 0.6); }
  .start-btn:active { transform: scale(0.95); }
  .start-btn.recording { background: linear-gradient(135deg, var(--danger) 0%, #d64040 100%); box-shadow: 0 4px 16px rgba(255, 77, 77, 0.5); animation: pulse 2s ease-in-out infinite; }
  .start-btn.recording:hover { transform: scale(1.05); box-shadow: 0 5px 20px rgba(255, 77, 77, 0.6); }

  /* Recording indicator */
  .recording-indicator {
    position: absolute;
    top: -3px;
    right: -3px;
    width: 10px;
    height: 10px;
    background: var(--danger);
    border: 2px solid var(--bg-solid);
    border-radius: 50%;
    animation: recording-pulse 1.5s infinite;
  }

  /* Audio source active indicator */
  .audio-active-indicator {
    position: absolute;
    top: 4px;
    right: 4px;
    width: 7px;
    height: 7px;
    background: linear-gradient(135deg, #4caf50 0%, #66bb6a 100%);
    border: 1.5px solid rgba(0, 0, 0, 0.4);
    border-radius: 50%;
    animation: audio-indicator-pulse 1.5s infinite;
    z-index: var(--z-base);
    box-shadow: 0 0 6px rgba(76, 175, 80, 0.6);
  }

  /* Utility group: TTS + History + Clear */
  .utility-group {
    display: flex;
    align-items: center;
    background: rgba(255, 255, 255, 0.06);
    backdrop-filter: blur(10px);
    border-radius: 10px;
    padding: 3px;
    gap: 3px;
    border: 1px solid rgba(255, 255, 255, 0.12);
    box-shadow:
      0 4px 12px rgba(0, 0, 0, 0.15),
      inset 0 1px 0 rgba(255, 255, 255, 0.1);
    flex-shrink: 0;
  }

  /* TTS button */
  .tts-btn {
    height: 36px;
    padding: 0 12px;
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    background: transparent;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.05);
    color: var(--text-dim);
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 4px;
    font-family: var(--font-family);
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    flex-shrink: 0;
  }

  .tts-btn svg { width: 14px; height: 14px; }
  .tts-label { font-size: 9px; font-weight: 600; letter-spacing: 0.5px; }
  .tts-btn:hover {
    border-color: rgba(255, 255, 255, 0.15);
    color: var(--text-secondary);
    transform: translateY(-1px);
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.1);
  }
  .tts-btn.active {
    background: linear-gradient(135deg, rgba(99, 140, 255, 0.15) 0%, rgba(99, 140, 255, 0.08) 100%);
    border-color: rgba(99, 140, 255, 0.5);
    color: var(--accent);
    box-shadow: 0 2px 6px rgba(99, 140, 255, 0.2);
  }
  .tts-btn.active:hover {
    background: linear-gradient(135deg, rgba(99, 140, 255, 0.2) 0%, rgba(99, 140, 255, 0.12) 100%);
    box-shadow: 0 3px 8px rgba(99, 140, 255, 0.3);
  }


  /* Enhanced clear button */
  .bar-btn:last-child:hover {
    color: var(--danger);
    background: rgba(255, 77, 77, 0.1);
  }
</style>
