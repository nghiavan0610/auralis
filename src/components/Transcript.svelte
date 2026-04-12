<script lang="ts">
  import { tick } from 'svelte';
  import { onDestroy } from 'svelte';
  import type { Segment } from '../types';
  import { getLangLabel, getLangFlag } from '../js/lang';

  let {
    sourceLanguage = 'en',
    targetLanguage = 'vi',
    translationType = 'one_way',
    segments = [],
    provisionalText = '',
    provisionalLang = '',
    fontSize = 14,
  }: {
    sourceLanguage?: string;
    targetLanguage?: string;
    translationType?: 'one_way' | 'two_way';
    segments?: Segment[];
    provisionalText?: string;
    provisionalLang?: string;
    fontSize?: number;
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

  const isEmpty = $derived(segments.length === 0 && !provisionalText);
  const isTwoWay = $derived(translationType === 'two_way');

  onDestroy(() => {
    if (leftCopyTimer) clearTimeout(leftCopyTimer);
    if (rightCopyTimer) clearTimeout(rightCopyTimer);
  });
</script>

<div class="transcript" style="--entry-font-size: {fontSize}px">
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
        <div class="empty-state">Listening...</div>
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
        <div class="empty-state">Translation...</div>
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

  /* --- Empty state --- */
  .empty-state {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-dim);
    font-size: var(--font-size-sm);
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
</style>
