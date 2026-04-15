<script lang="ts">
  import { tick } from 'svelte';
  import { onDestroy } from 'svelte';
  import type { Segment } from '../types';
  import { getLangLabel, getLangFlag } from '../js/lang';
  import ConversationBubble from './ConversationBubble.svelte';

  let {
    sourceLanguage = 'en',
    targetLanguage = 'vi',
    translationType = 'one_way',
    mode = 'cloud',
    audioSource = 'microphone',
    segments = [],
    provisionalText = '',
    provisionalLang = '',
    fontSize = 14,
    onOpenSettings,
  }: {
    sourceLanguage?: string;
    targetLanguage?: string;
    translationType?: 'one_way' | 'two_way';
    mode?: 'cloud' | 'offline';
    audioSource?: 'microphone' | 'system' | 'both';
    segments?: Segment[];
    provisionalText?: string;
    provisionalLang?: string;
    fontSize?: number;
    onOpenSettings?: () => void;
  } = $props();

  let leftContainer: HTMLDivElement | undefined = $state();
  let rightContainer: HTMLDivElement | undefined = $state();
  let leftCopied = $state(false);
  let rightCopied = $state(false);
  let leftCopyTimer: ReturnType<typeof setTimeout> | null = null;
  let rightCopyTimer: ReturnType<typeof setTimeout> | null = null;

  function getOriginalText(): string {
    return segments.map(s => s.original).filter(Boolean).join('\n');
  }

  function getTranslatedText(): string {
    return segments.map(s => s.translated).filter(Boolean).join('\n');
  }

  async function copyLeft() {
    const text = getOriginalText();
    if (!text) return;
    await navigator.clipboard.writeText(text);
    leftCopied = true;
    if (leftCopyTimer) clearTimeout(leftCopyTimer);
    leftCopyTimer = setTimeout(() => { leftCopied = false; }, 1500);
  }

  async function copyRight() {
    const text = getTranslatedText();
    if (!text) return;
    await navigator.clipboard.writeText(text);
    rightCopied = true;
    if (rightCopyTimer) clearTimeout(rightCopyTimer);
    rightCopyTimer = setTimeout(() => { rightCopied = false; }, 1500);
  }

  async function scrollToBottom(el: HTMLDivElement | undefined) {
    if (!el) return;
    await tick();
    el.scrollTop = el.scrollHeight;
  }

  $effect(() => {
    // Explicitly read reactive values so Svelte tracks deep changes
    const len = segments.length;
    const last = len > 0 ? segments[len - 1] : null;
    if (last) { last.original; last.translated; last.status; }
    const prov = provisionalText;
    scrollToBottom(leftContainer);
    scrollToBottom(rightContainer);
  });

  function getSpeakerNumber(seg: Segment): number {
    if (seg.detectedLang === sourceLanguage) return 1;
    if (seg.detectedLang === targetLanguage) return 2;
    return 1;
  }

  function shouldShowSpeakerLabel(index: number): boolean {
    if (index === 0) return true;
    const prev = segments[index - 1];
    const curr = segments[index];
    return prev.detectedLang !== curr.detectedLang;
  }

  // Helper for two-way mode to get speaker info
  function getSpeakerInfo(seg: Segment): {
    number: number;
    confidence?: 'high' | 'medium' | 'low';
  } {
    if (seg.detectedLang === sourceLanguage) {
      return { number: 1, confidence: seg.confidence };
    }
    if (seg.detectedLang === targetLanguage) {
      return { number: 2, confidence: seg.confidence };
    }
    return { number: 1 };
  }

  // Helper to determine which speaker the provisional text belongs to
  const isEmpty = $derived(segments.length === 0 && !provisionalText);
  const isTwoWay = $derived(translationType === 'two_way');

  onDestroy(() => {
    if (leftCopyTimer) clearTimeout(leftCopyTimer);
    if (rightCopyTimer) clearTimeout(rightCopyTimer);
  });
</script>

<div class="transcript" style="--entry-font-size: {fontSize}px">
  {#if isTwoWay}
    <!-- Two-way mode: Conversation bubble layout -->
    <div class="conversation-view">
      {#if isEmpty}
        <div class="empty-state-enhanced">
          <div class="empty-icon">
            <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="var(--text-dim)" stroke-width="1.5">
              <path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z"/>
              <path d="M19 10v2a7 7 0 0 1-14 0v-2"/>
              <line x1="12" y1="19" x2="12" y2="23"/>
              <line x1="8" y1="23" x2="16" y2="23"/>
            </svg>
          </div>
          <div class="empty-title">Ready to translate</div>
          <div class="empty-config">
            <span class="config-badge">{getLangFlag(sourceLanguage)} Speaker 1</span>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--text-dim)" stroke-width="2">
              <path d="M5 12h14M12 5l7 7-7 7"/>
            </svg>
            <span class="config-badge">{getLangFlag(targetLanguage)} Speaker 2</span>
          </div>
          <div class="empty-mode">
            {#if mode === 'cloud'}
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M18 10h-1.26A8 8 0 1 0 9 20h9a5 5 0 0 0 0-10z"/>
              </svg>
              Cloud Mode
            {:else}
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <rect x="3" y="3" width="18" height="18" rx="2"/>
              </svg>
              Offline Mode
            {/if}
          </div>
          <div class="empty-hint">Click the microphone button to start</div>
          {#if onOpenSettings}
            <button class="empty-settings-btn" onclick={onOpenSettings}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="3"/>
                <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06-.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
              </svg>
              Configure Settings
            </button>
          {/if}
        </div>
      {:else}
        <!-- Conversation bubbles for two-way mode -->
        {#each segments as seg, i (seg.id)}
          {@const speakerInfo = getSpeakerInfo(seg)}
          <ConversationBubble
            segment={seg}
            speakerNumber={speakerInfo.number}
            {sourceLanguage}
            {targetLanguage}
            showTranslation={true}
            {fontSize}
          />
        {/each}

        <!-- Provisional text bubble -->
        {#if provisionalText}
          {@const provisionalSpeakerNumber = provisionalLang === sourceLanguage ? 1 : 2}
          <ConversationBubble
            provisionalText={provisionalText}
            provisionalLang={provisionalLang}
            speakerNumber={provisionalSpeakerNumber}
            {sourceLanguage}
            {targetLanguage}
            showTranslation={false}
            {fontSize}
          />
        {/if}
      {/if}
    </div>
  {:else}
    <!-- One-way mode: Existing two-column layout -->
    <!-- Column headers -->
    <div class="panel-headers">
    <div class="panel-header">
      <span class="header-flag">{getLangFlag(sourceLanguage)}</span>
      <span class="header-lang">{getLangLabel(sourceLanguage)}</span>
      <button class="btn-copy" onclick={copyLeft} title="Copy original" disabled={isEmpty}>
        {#if leftCopied}
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="var(--success)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="20 6 9 17 4 12"/>
          </svg>
        {:else}
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2"/>
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
          </svg>
        {/if}
      </button>
    </div>
    <div class="header-divider"></div>
    <div class="panel-header">
      <span class="header-flag">{getLangFlag(targetLanguage)}</span>
      <span class="header-lang">{getLangLabel(targetLanguage)}</span>
      <button class="btn-copy" onclick={copyRight} title="Copy translation" disabled={isEmpty}>
        {#if rightCopied}
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="var(--success)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="20 6 9 17 4 12"/>
          </svg>
        {:else}
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2"/>
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
          </svg>
        {/if}
      </button>
    </div>
  </div>

  <!-- Two columns: left = original, right = translated -->
  <div class="columns">
    <!-- LEFT: Original text -->
    <div class="column" bind:this={leftContainer}>
      {#if isEmpty}
        <div class="empty-state-enhanced">
          <div class="empty-icon">
            <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="var(--text-dim)" stroke-width="1.5">
              <path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z"/>
              <path d="M19 10v2a7 7 0 0 1-14 0v-2"/>
              <line x1="12" y1="19" x2="12" y2="23"/>
              <line x1="8" y1="23" x2="16" y2="23"/>
            </svg>
          </div>
          <div class="empty-title">Ready to translate</div>
          <div class="empty-config">
            <span class="config-badge">{getLangFlag(sourceLanguage)} {getLangLabel(sourceLanguage)}</span>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--text-dim)" stroke-width="2">
              <path d="M5 12h14M12 5l7 7-7 7"/>
            </svg>
            <span class="config-badge">{getLangFlag(targetLanguage)} {getLangLabel(targetLanguage)}</span>
          </div>
          <div class="empty-mode">
            {#if mode === 'cloud'}
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M18 10h-1.26A8 8 0 1 0 9 20h9a5 5 0 0 0 0-10z"/>
              </svg>
              Cloud Mode
            {:else}
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <rect x="3" y="3" width="18" height="18" rx="2"/>
              </svg>
              Offline Mode
            {/if}
          </div>
          <div class="empty-hint">Click the microphone button to start</div>
          {#if onOpenSettings}
            <button class="empty-settings-btn" onclick={onOpenSettings}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="3"/>
                <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
              </svg>
              Configure Settings
            </button>
          {/if}
        </div>
      {:else}
        {#each segments as seg, i (seg.id)}
          {#if isTwoWay && seg.detectedLang && shouldShowSpeakerLabel(i)}
            <div class="speaker-label">
              <span class="speaker-flag">{getLangFlag(seg.detectedLang)}</span>
              <span class="speaker-name">Speaker {getSpeakerNumber(seg)}</span>
              <span class="speaker-lang">{getLangLabel(seg.detectedLang)}</span>
            </div>
          {/if}
          <div class="entry-line" class:pending={seg.status === 'pending'} class:provisional={seg.status === 'provisional'}>
            {seg.original}
          </div>
        {/each}
        {#if provisionalText}
          <div class="entry-line provisional">
            {#if isTwoWay && provisionalLang}
              <span class="inline-lang">{getLangFlag(provisionalLang)} {getLangLabel(provisionalLang)}</span>
            {/if}
            {provisionalText}
            <span class="typing-indicator">
              <span></span><span></span><span></span>
            </span>
          </div>
        {/if}
      {/if}
    </div>

    <div class="column-divider"></div>

    <!-- RIGHT: Translated text -->
    <div class="column" bind:this={rightContainer}>
      {#if isEmpty}
        <div class="empty-state-enhanced empty-state-secondary">
          <div class="empty-icon">
            <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="var(--accent)" stroke-width="1.5">
              <path d="M5 8l6 6M5 14l6-6"/>
              <path d="M12.5 3L17 8l-4.5 5M17 8H7"/>
            </svg>
          </div>
          <div class="empty-title">Translation</div>
          <div class="empty-desc">Translated text will appear here</div>
        </div>
      {:else}
        {#each segments as seg, i (seg.id)}
          {#if seg.translated}
            <div class="entry-line translated">{seg.translated}</div>
          {:else if seg.status === 'pending' || seg.status === 'provisional'}
            <div class="entry-line waiting">
              <span class="dot-pulse"></span>
            </div>
          {/if}
        {/each}
        {#if provisionalText}
          <div class="entry-line waiting">
            <span class="dot-pulse"></span>
          </div>
        {/if}
      {/if}
    </div>
  </div>
  {/if}
</div>

<style>
  .transcript {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  /* --- Headers --- */
  .panel-headers {
    display: grid;
    grid-template-columns: 1fr 1px 1fr;
    flex-shrink: 0;
    border-bottom: 1px solid var(--border);
  }

  .panel-header {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    padding: var(--space-xs) var(--space-md);
  }

  .btn-copy {
    margin-left: auto;
    width: 22px;
    height: 22px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    border-radius: 4px;
    color: var(--text-dim);
    cursor: pointer;
    transition: all var(--transition-fast);
    flex-shrink: 0;
  }

  .btn-copy:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .btn-copy:disabled {
    opacity: 0.3;
    cursor: default;
  }

  .header-divider {
    background: var(--border);
  }

  .header-flag {
    font-size: 12px;
  }

  .header-lang {
    font-size: var(--font-size-xs);
    font-weight: 600;
    color: var(--text-secondary);
    letter-spacing: 0.5px;
  }

  /* --- Columns --- */
  .columns {
    flex: 1;
    display: grid;
    grid-template-columns: 1fr 1px 1fr;
    overflow: hidden;
  }

  .column {
    overflow-y: auto;
    padding: var(--space-sm) var(--space-md);
    display: flex;
    flex-direction: column;
    gap: 2px;
    user-select: text;
    -webkit-user-select: text;
  }

  .column-divider {
    background: var(--border);
  }


  /* --- Enhanced empty state --- */
  .empty-state-enhanced {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    padding: var(--space-lg);
    text-align: center;
    gap: var(--space-md);
  }

  .empty-state-secondary .empty-icon {
    opacity: 0.6;
  }

  .empty-state-secondary .empty-title {
    color: var(--text-dim);
  }

  .empty-icon {
    opacity: 0.4;
    margin-bottom: var(--space-xs);
  }

  .empty-title {
    font-size: var(--font-size-md);
    font-weight: 600;
    color: var(--text-primary);
  }

  .empty-config {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    flex-wrap: wrap;
    justify-content: center;
  }

  .config-badge {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 10px;
    background: rgba(99, 140, 255, 0.1);
    border: 1px solid rgba(99, 140, 255, 0.2);
    border-radius: var(--radius-md);
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-primary);
  }

  .empty-mode {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    background: rgba(255, 255, 255, 0.04);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-xs);
    color: var(--text-dim);
  }

  .empty-hint {
    font-size: var(--font-size-sm);
    color: var(--text-dim);
  }

  .empty-desc {
    font-size: var(--font-size-sm);
    color: var(--text-dim);
  }

  .empty-settings-btn {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    padding: 8px 14px;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    font-size: var(--font-size-sm);
    font-family: var(--font-family);
    cursor: pointer;
    transition: all 0.2s ease;
    position: relative;
    z-index: 1;
    pointer-events: auto;
  }

  .empty-settings-btn:hover {
    background: rgba(99, 140, 255, 0.1);
    border-color: var(--accent);
    color: var(--text-primary);
  }

  /* --- Speaker label (two-way mode) --- */
  .speaker-label {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: var(--space-sm) 0 2px 0;
    margin-top: var(--space-xs);
  }

  .speaker-label:first-child {
    margin-top: 0;
  }

  .speaker-flag {
    font-size: 13px;
  }

  .speaker-name {
    font-size: var(--font-size-xs);
    font-weight: 600;
    color: var(--text-primary);
    letter-spacing: 0.3px;
  }

  .speaker-lang {
    font-size: var(--font-size-xs);
    color: var(--text-dim);
  }

  /* --- Entry line (flat, no box) --- */
  .entry-line {
    font-size: var(--entry-font-size, var(--font-size-base));
    line-height: 1.5;
    color: var(--text-primary);
    word-break: break-word;
    padding: 3px 0;
    animation: fadeIn 0.15s ease;
  }

  .entry-line.pending {
    opacity: 0.75;
  }

  .entry-line.provisional {
    opacity: 0.85;
  }

  .entry-line.translated {
    color: var(--accent);
  }

  .entry-line.waiting {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    color: var(--text-dim);
    min-height: 20px;
  }

  .inline-lang {
    font-size: var(--font-size-xs);
    color: var(--text-dim);
    margin-right: 4px;
  }

  /* --- Typing indicator --- */
  .typing-indicator {
    display: inline-flex;
    gap: 2px;
    align-items: center;
    margin-left: 4px;
  }

  .typing-indicator span {
    width: 3px;
    height: 3px;
    border-radius: 50%;
    background: var(--accent);
    animation: pulse 1s ease-in-out infinite;
  }

  .typing-indicator span:nth-child(2) { animation-delay: 0.15s; }
  .typing-indicator span:nth-child(3) { animation-delay: 0.3s; }

  /* --- Dot pulse --- */
  .dot-pulse {
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: var(--text-dim);
    animation: pulse 1.2s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 0.3; }
    50% { opacity: 1; }
  }

  @keyframes fadeIn {
    from { opacity: 0; transform: translateY(2px); }
    to { opacity: 1; transform: translateY(0); }
  }

  /* --- Conversation View (Two-Way Mode) --- */
  .conversation-view {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: var(--space-md) var(--space-sm);
    overflow-y: auto;
    gap: var(--space-md);
  }
</style>
