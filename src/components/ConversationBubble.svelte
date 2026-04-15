<script lang="ts">
  import type { Segment } from '../types';
  import { getLangFlag, getLangLabel } from '../js/lang';

  interface Props {
    segment?: Segment;  // Optional for provisional bubbles
    speakerNumber: 1 | 2;
    sourceLanguage: string;
    targetLanguage: string;
    showTranslation: boolean;
    fontSize: number;
    // Provisional props (for real-time detection feedback)
    provisionalText?: string;
    provisionalLang?: string;
  }

  let {
    segment,
    speakerNumber,
    sourceLanguage,
    targetLanguage,
    showTranslation,
    fontSize,
    provisionalText,
    provisionalLang
  }: Props = $props();

  const isSpeaker1 = $derived(speakerNumber === 1);
  const isTranslated = $derived(segment?.status === 'translated');
  const isProvisional = $derived(segment?.status === 'provisional' || !!provisionalText);
  const isSourceLang = $derived(segment?.detectedLang === sourceLanguage);

  // Use provisional values if segment doesn't exist, otherwise use segment data
  const displayText = $derived(provisionalText || segment?.original || '');
  const displayLang = $derived(provisionalLang || segment?.detectedLang || sourceLanguage);

  // Get language label for display
  const langLabel = $derived(getLangLabel(displayLang));
  const langFlag = $derived(getLangFlag(displayLang));

  // For confidence badges, only show on non-provisional segments
  const confidenceLevel = $derived(segment?.confidence);
</script>

<div class="conversation-bubble" class:speaker-1={isSpeaker1} class:speaker-2={!isSpeaker1} class:provisional={isProvisional}>
  <!-- Speaker Label -->
  <div class="bubble-header">
    <span class="speaker-flag">{langFlag}</span>
    {#if provisionalText}
      <span class="speaker-name">Detecting...</span>
    {:else}
      <span class="speaker-name">Speaker {speakerNumber}</span>
    {/if}
    <span class="speaker-lang">{langLabel}</span>
    {#if confidenceLevel && !provisionalText}
      <span class="confidence-badge {confidenceLevel}">{confidenceLevel}</span>
    {/if}
  </div>

  <!-- Content (original or provisional text) -->
  {#if displayText}
    <div class="bubble-content original" style="font-size: {fontSize}px">
      {displayText}
    </div>
  {/if}

  <!-- Translated Text (only for non-provisional segments) -->
  {#if showTranslation && isTranslated && segment}
    <div class="bubble-content translated" style="font-size: {fontSize}px">
      {segment.translated}
    </div>
  {:else if showTranslation && !isTranslated && !provisionalText}
    <div class="bubble-content waiting">
      <span class="dot-pulse"></span>
    </div>
  {/if}

  <!-- Provisional Typing Indicator -->
  {#if isProvisional}
    <span class="typing-indicator">
      <span></span><span></span><span></span>
    </span>
  {/if}
</div>

<style>
  .conversation-bubble {
    max-width: 70%;
    padding: var(--space-sm);
    margin-bottom: var(--space-md);
    border-radius: var(--radius-md);
    backdrop-filter: blur(10px);
    animation: slideIn 0.3s ease;
    position: relative;
  }

  .conversation-bubble.speaker-1 {
    align-self: flex-start;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-left: 3px solid var(--accent);
  }

  .conversation-bubble.speaker-2 {
    align-self: flex-end;
    background: rgba(99, 140, 255, 0.1);
    border: 1px solid rgba(99, 140, 255, 0.2);
    border-right: 3px solid var(--accent);
  }

  .conversation-bubble.provisional {
    opacity: 0.85;
  }

  .bubble-header {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
    margin-bottom: var(--space-xs);
    font-size: var(--font-size-xs);
    color: var(--text-secondary);
  }

  .speaker-flag {
    font-size: 13px;
  }

  .speaker-name {
    font-weight: 600;
    color: var(--text-primary);
    letter-spacing: 0.3px;
  }

  .speaker-lang {
    color: var(--text-dim);
    font-size: 10px;
  }

  .bubble-content {
    line-height: 1.5;
    word-break: break-word;
  }

  .bubble-content.original {
    color: var(--text-primary);
  }

  .bubble-content.translated {
    color: var(--accent);
    margin-top: var(--space-xs);
  }

  .bubble-content.waiting {
    display: flex;
    align-items: center;
    color: var(--text-dim);
    min-height: 20px;
  }

  .confidence-badge {
    padding: 2px 6px;
    border-radius: var(--radius-sm);
    font-size: 9px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .confidence-badge.high {
    background: rgba(76, 175, 80, 0.2);
    color: var(--success);
  }

  .confidence-badge.medium {
    background: rgba(255, 152, 0, 0.2);
    color: var(--warning);
  }

  .confidence-badge.low {
    background: rgba(255, 77, 77, 0.2);
    color: var(--danger);
  }

  .typing-indicator {
    display: flex;
    gap: 4px;
    padding: 4px 0;
  }

  .typing-indicator span {
    width: 3px;
    height: 3px;
    border-radius: 50%;
    background: var(--accent);
    animation: pulse 1s ease-in-out infinite;
  }

  .typing-indicator span:nth-child(2) {
    animation-delay: 0.15s;
  }

  .typing-indicator span:nth-child(3) {
    animation-delay: 0.3s;
  }

  @keyframes slideIn {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  @keyframes pulse {
    0%, 100% {
      opacity: 0.3;
    }
    50% {
      opacity: 1;
    }
  }

  /* Reduced motion support */
  @media (prefers-reduced-motion: reduce) {
    .conversation-bubble {
      animation: none;
    }

    .typing-indicator span {
      animation: none;
    }
  }
</style>
