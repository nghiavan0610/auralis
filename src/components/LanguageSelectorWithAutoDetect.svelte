<script lang="ts">
  import { getLangFlag, getLangLabel, SUPPORTED_LANGUAGES } from '../js/lang';

  /**
   * LanguageSelectorWithAutoDetect - Enhanced language selector with auto-detect option
   *
   * Features:
   * - Auto-detect option with visual prominence
   * - One-way mode: Show auto-detect + target language
   * - Two-way mode: Show source + target language selectors
   * - Visual differentiation between modes
   * - Help text and descriptions
   */

  interface LanguageSelectorProps {
    translationType: 'one_way' | 'two_way';
    sourceLanguage: string;
    targetLanguage: string;
    disabled?: boolean;
    onChange: (source: string, target: string) => void;
  }

  let {
    translationType,
    sourceLanguage: initialSource,
    targetLanguage: initialTarget,
    disabled = false,
    onChange
  }: LanguageSelectorProps = $props();

  let localSource = $state(initialSource);
  let localTarget = $state(initialTarget);

  // Sync with props
  $effect(() => {
    localSource = initialSource;
    localTarget = initialTarget;
  });

  function handleSourceChange(code: string) {
    localSource = code;
    onChange(localSource, localTarget);
  }

  function handleTargetChange(code: string) {
    localTarget = code;
    onChange(localSource, localTarget);
  }

  let isAutoDetect = $derived(translationType === 'one_way');
</script>

<div class="language-selector-with-auto-detect">
  <!-- Section Label -->
  <div class="section-label">
    Languages
  </div>
  <p class="section-desc">
    {#if isAutoDetect}
      One-way mode auto-detects any spoken language and translates to target.
    {:else}
      Two-way mode requires selecting both source and target languages.
    {/if}
  </p>

  <!-- Language Selectors -->
  <div class="language-row" class:one-way-mode={isAutoDetect}>
    {#if !isAutoDetect}
      <!-- Source Language Selector -->
      <div class="field">
        <label for="source-lang">Source</label>
        <select
          id="source-lang"
          bind:value={localSource}
          onchange={(e) => handleSourceChange(e.target.value)}
          disabled={disabled}
        >
          {#each SUPPORTED_LANGUAGES as lang}
            <option value={lang.code}>{lang.flag} {lang.name}</option>
          {/each}
        </select>
      </div>

      <!-- Arrow Indicator -->
      <div class="lang-arrow">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M5 12h14M12 5l7 7-7 7"/>
        </svg>
      </div>
    {:else}
      <!-- Auto-Detect Indicator -->
      <div class="auto-detect-field">
        <div class="field-label">Source</div>
        <div class="auto-detect-option" role="status" aria-live="polite">
          <span class="auto-detect-icon">🌐</span>
          <div class="auto-detect-content">
            <span class="auto-detect-label">Auto-detect</span>
            <span class="auto-detect-desc">Any language</span>
          </div>
          <span class="auto-detect-badge">AUTO</span>
        </div>
      </div>

      <!-- Arrow Indicator -->
      <div class="lang-arrow">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M5 12h14M12 5l7 7-7 7"/>
        </svg>
      </div>
    {/if}

    <!-- Target Language Selector -->
    <div class="field">
      <label for="target-lang">Target</label>
      <select
        id="target-lang"
        bind:value={localTarget}
        onchange={(e) => handleTargetChange(e.target.value)}
        disabled={disabled}
      >
        {#each SUPPORTED_LANGUAGES as lang}
          <option value={lang.code}>{lang.flag} {lang.name}</option>
        {/each}
      </select>
    </div>
  </div>

  <!-- Mode Description -->
  <div class="mode-description">
    {#if isAutoDetect}
      <div class="mode-info">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--accent)" stroke-width="2">
          <circle cx="12" cy="12" r="10"/>
          <path d="M12 16v-4M12 8h.01"/>
        </svg>
        <span>
          Automatically detects any spoken language and translates to <strong>{getLangLabel(targetLanguage)}</strong>
        </span>
      </div>
    {:else}
      <div class="mode-info">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--accent)" stroke-width="2">
          <path d="M7 16l-4-4 4-4"/>
          <path d="M17 8l4 4-4 4"/>
          <line x1="3" y1="12" x2="21" y2="12"/>
        </svg>
        <span>
          Auto-detects between <strong>{getLangLabel(sourceLanguage)}</strong> and <strong>{getLangLabel(targetLanguage)}</strong>
        </span>
      </div>
    {/if}
  </div>
</div>

<style>
  .language-selector-with-auto-detect {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .section-label {
    font-size: 0.75rem;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.55);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .section-desc {
    font-size: 0.75rem;
    color: rgba(255, 255, 255, 0.3);
    margin: 0;
    margin-top: -6px;
    line-height: 1.5;
  }

  .language-row {
    display: flex;
    align-items: flex-end;
    gap: 12px;
  }

  .language-row.one-way-mode {
    /* Hide source language selector in one-way mode */
  }

  .language-row.one-way-mode .field:first-child {
    display: none;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 8px;
    flex: 1;
  }

  .field label {
    font-size: 0.875rem;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.55);
  }

  .field select {
    font-family: var(--font-family);
    font-size: 1rem;
    color: var(--text-primary);
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 6px;
    padding: 8px 12px;
    outline: none;
    transition: border-color 0.15s ease;
    cursor: pointer;
    -webkit-appearance: none;
    appearance: none;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' fill='rgba(255,255,255,0.5)' viewBox='0 0 16 16'%3E%3Cpath d='M4 6l4 4 4-4'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 10px center;
    padding-right: 28px;
  }

  .field select:focus {
    border-color: var(--accent);
  }

  .field select:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .lang-arrow {
    color: rgba(255, 255, 255, 0.3);
    padding-bottom: 8px;
    flex-shrink: 0;
  }

  /* Auto-detect field styling */
  .auto-detect-field {
    display: flex;
    flex-direction: column;
    gap: 8px;
    flex: 1;
  }

  .auto-detect-field label {
    font-size: 0.875rem;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.55);
  }

  .auto-detect-option {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px;
    background: linear-gradient(
      135deg,
      rgba(99, 140, 255, 0.1) 0%,
      rgba(99, 140, 255, 0.05) 100%
    );
    border: 1px solid rgba(99, 140, 255, 0.2);
    border-radius: 10px;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .auto-detect-option:hover {
    background: linear-gradient(
      135deg,
      rgba(99, 140, 255, 0.15) 0%,
      rgba(99, 140, 255, 0.08) 100%
    );
    border-color: rgba(99, 140, 255, 0.3);
    transform: translateX(2px);
  }

  .auto-detect-icon {
    font-size: 18px;
    flex-shrink: 0;
  }

  .auto-detect-content {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
  }

  .auto-detect-label {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--accent);
  }

  .auto-detect-desc {
    font-size: 0.75rem;
    color: rgba(255, 255, 255, 0.3);
  }

  .auto-detect-badge {
    display: inline-flex;
    align-items: center;
    padding: 2px 8px;
    background: rgba(99, 140, 255, 0.2);
    color: var(--accent);
    font-size: 9px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    border-radius: 10px;
    border: 1px solid rgba(99, 140, 255, 0.3);
    flex-shrink: 0;
  }

  /* Mode description */
  .mode-description {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .mode-info {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    background: rgba(99, 140, 255, 0.05);
    border: 1px solid rgba(99, 140, 255, 0.1);
    border-radius: 8px;
    font-size: 0.875rem;
    color: rgba(255, 255, 255, 0.55);
  }

  .mode-info strong {
    color: var(--accent);
    font-weight: 600;
  }

  .mode-info svg {
    flex-shrink: 0;
  }

  /* Responsive adjustments */
  @media (max-width: 640px) {
    .language-row {
      flex-direction: column;
      align-items: stretch;
    }

    .lang-arrow {
      transform: rotate(90deg);
      align-self: center;
      padding: 4px 0;
    }

    .auto-detect-option {
      padding: 10px;
    }

    .auto-detect-icon {
      font-size: 16px;
    }
  }

  /* Accessibility */
  @media (prefers-reduced-motion: reduce) {
    .auto-detect-option,
    .auto-detect-option:hover {
      transition: none;
      transform: none;
    }
  }

  .auto-detect-option:focus-visible,
  .field select:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
</style>
