<script lang="ts">
  import Tooltip from './Tooltip.svelte';
  import { getLangFlag, getLangLabel } from '../js/lang';
  import type { DetectionState } from '../types';

  /**
   * AutoDetectIndicator - Visual component for language auto-detection state
   *
   * Displays current detection state with appropriate visual feedback:
   * - Idle: Subtle globe icon
   * - Detecting: Animated pulse with ripple effect
   * - Detected: Success state with detected language flag
   * - Uncertain: Warning state with wobble animation
   */

  let {
    detectionState = {
      status: 'idle' as const
    },
    targetLanguage = 'vi',
    showLabel = true,
    compact = false,
    onClick = null as (() => void) | null
  }: {
    detectionState: DetectionState;
    targetLanguage?: string;
    showLabel?: boolean;
    compact?: boolean;
    onClick?: (() => void) | null;
  } = $props();

  // Computed values for visual state - use props directly with $derived
  let iconClass = $derived(
    detectionState.status === 'idle' ? 'idle' :
    detectionState.status === 'detecting' ? 'detecting' :
    detectionState.status === 'detected' ? 'detected' :
    detectionState.status === 'uncertain' ? 'uncertain' : ''
  );

  let displayIcon = $derived(
    detectionState.status === 'detected' && detectionState.detectedLanguage
      ? getLangFlag(detectionState.detectedLanguage) ?? '🌐'
      : '🌐'
  );

  let statusText = $derived(
    detectionState.status === 'idle' ? 'Ready' :
    detectionState.status === 'detecting' ? 'Detecting...' :
    detectionState.status === 'detected' && detectionState.detectedLanguage
      ? getLangLabel(detectionState.detectedLanguage) ?? 'Unknown'
    : detectionState.status === 'uncertain' ? 'Uncertain' : 'Error'
  );

  // Aria labels for accessibility
  let ariaLabel = $derived(
    detectionState.status === 'idle' ? 'Auto-detect ready' :
    detectionState.status === 'detecting' ? 'Detecting language' :
    detectionState.status === 'detected' && detectionState.detectedLanguage
      ? `Detected ${getLangLabel(detectionState.detectedLanguage) ?? 'Unknown language'}`
    : detectionState.status === 'uncertain' ? 'Language detection uncertain' : 'Detection error'
  );
</script>

<div
  class="auto-detect-indicator"
  class:compact={compact}
  class:detecting={detectionState.status === 'detecting'}
  class:detected={detectionState.status === 'detected'}
  class:uncertain={detectionState.status === 'uncertain'}
  role="status"
  aria-label={ariaLabel}
  aria-live="polite"
  onclick={onClick}
>
  <!-- Detection overlay for shimmer effect -->
  {#if detectionState.status === 'detecting'}
    <div class="detection-overlay"></div>
  {/if}

  <!-- Icon/Flag display -->
  <div class="icon-container">
    <span class="auto-detect-icon {iconClass}">
      {displayIcon}
    </span>
  </div>

  <!-- Arrow separator -->
  {#if showLabel && !compact}
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="var(--text-dim)" stroke-width="2">
      <path d="M5 12h14M12 5l7 7-7 7"/>
    </svg>
  {/if}

  <!-- Target language flag -->
  <span class="lang-flag">
    {getLangFlag(targetLanguage) ?? '🌐'}
  </span>

  <!-- Status badge -->
  {#if showLabel && !compact}
    <span class="detection-status {detectionState.status}">
      {statusText}
    </span>
  {/if}

  <!-- Screen reader only content -->
  <span class="sr-only">
    {ariaLabel}
  </span>
</div>

<style>
  .auto-detect-indicator {
    position: relative;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    background: rgba(99, 140, 255, 0.08);
    border: 1px solid rgba(99, 140, 255, 0.15);
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.3s ease;
    overflow: hidden;
  }

  .auto-detect-indicator:hover {
    background: rgba(99, 140, 255, 0.15);
    border-color: rgba(99, 140, 255, 0.3);
    transform: translateY(-1px);
  }

  .auto-detect-indicator.compact {
    padding: 3px 6px;
    gap: 4px;
  }

  .auto-detect-indicator.detecting {
    background: rgba(99, 140, 255, 0.12);
    border-color: rgba(99, 140, 255, 0.25);
  }

  .auto-detect-indicator.detected {
    background: rgba(76, 175, 80, 0.1);
    border-color: rgba(76, 175, 80, 0.2);
  }

  .auto-detect-indicator.uncertain {
    background: rgba(251, 191, 36, 0.1);
    border-color: rgba(251, 191, 36, 0.2);
  }

  .detection-overlay {
    position: absolute;
    inset: 0;
    background: linear-gradient(
      90deg,
      transparent,
      rgba(99, 140, 255, 0.1),
      transparent
    );
    border-radius: inherit;
    animation: detection-shimmer 2s ease-in-out infinite;
    pointer-events: none;
  }

  .icon-container {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
  }

  .auto-detect-icon {
    font-size: 12px;
    transition: all 0.3s ease;
  }

  .auto-detect-icon.idle {
    opacity: 0.7;
    filter: grayscale(0.3);
  }

  .auto-detect-icon.detecting {
    opacity: 1;
    animation: auto-detect-pulse 2s ease-in-out infinite;
  }

  .auto-detect-icon.detecting::before {
    content: '';
    position: absolute;
    inset: -4px;
    border-radius: 50%;
    background: radial-gradient(circle, rgba(99, 140, 255, 0.9) 0%, transparent 70%);
    animation: auto-detect-ripple 2s ease-in-out infinite;
    z-index: -1;
  }

  .auto-detect-icon.detected {
    animation: flag-pop 0.4s cubic-bezier(0.68, -0.55, 0.265, 1.55);
  }

  .auto-detect-icon.uncertain {
    opacity: 0.8;
    animation: auto-detect-wobble 0.5s ease-in-out;
  }

  .lang-flag {
    font-size: 10px;
    transition: all 0.3s ease;
  }

  .detection-status {
    padding: 2px 6px;
    border-radius: 10px;
    font-size: 9px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    transition: all 0.3s ease;
  }

  .detection-status.idle {
    background: rgba(255, 255, 255, 0.06);
    color: rgba(255, 255, 255, 0.3);
  }

  .detection-status.detecting {
    background: rgba(99, 140, 255, 0.15);
    color: var(--accent);
    animation: status-pulse 1.5s ease-in-out infinite;
  }

  .detection-status.detected {
    background: rgba(76, 175, 80, 0.15);
    color: #4caf50;
  }

  .detection-status.uncertain {
    background: rgba(251, 191, 36, 0.15);
    color: #ffb74d;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border-width: 0;
  }

  /* Animations */
  @keyframes auto-detect-pulse {
    0%, 100% {
      transform: scale(1);
      opacity: 0.9;
    }
    50% {
      transform: scale(1.05);
      opacity: 1;
    }
  }

  @keyframes auto-detect-ripple {
    0% {
      transform: scale(1);
      opacity: 0.6;
    }
    100% {
      transform: scale(1.8);
      opacity: 0;
    }
  }

  @keyframes flag-pop {
    0% {
      transform: scale(0.8);
      opacity: 0;
    }
    50% {
      transform: scale(1.2);
    }
    100% {
      transform: scale(1);
      opacity: 1;
    }
  }

  @keyframes auto-detect-wobble {
    0%, 100% {
      transform: rotate(0deg);
    }
    25% {
      transform: rotate(-3deg);
    }
    75% {
      transform: rotate(3deg);
    }
  }

  @keyframes status-pulse {
    0%, 100% {
      opacity: 1;
    }
    50% {
      opacity: 0.7;
    }
  }

  @keyframes detection-shimmer {
    0% {
      transform: translateX(-100%);
    }
    100% {
      transform: translateX(100%);
    }
  }

  /* Reduced motion support */
  @media (prefers-reduced-motion: reduce) {
    .auto-detect-icon.detecting,
    .detection-status.detecting,
    .detection-overlay {
      animation: none;
    }

    .auto-detect-indicator,
    .auto-detect-icon,
    .detection-status {
      transition: none;
    }
  }

  /* Focus styles for accessibility */
  .auto-detect-indicator:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
</style>
