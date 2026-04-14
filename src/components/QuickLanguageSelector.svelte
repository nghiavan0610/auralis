<script lang="ts">
  import { getLangLabel, getLangFlag, SUPPORTED_LANGUAGES } from '../js/lang';
  import Modal from './Modal.svelte';

  let {
    show = false,
    sourceLanguage = 'en',
    targetLanguage = 'vi',
    onSelect,
    onClose
  }: {
    show: boolean;
    sourceLanguage?: string;
    targetLanguage?: string;
    onSelect: (source: string, target: string) => void;
    onClose?: () => void;
  } = $props();

  let localSource = $state(sourceLanguage);
  let localTarget = $state(targetLanguage);
  let searchQuery = $state('');

  // Common language pairs
  const commonPairs = [
    { source: 'en', target: 'vi', label: 'English → Vietnamese' },
    { source: 'vi', target: 'en', label: 'Vietnamese → English' },
    { source: 'en', target: 'es', label: 'English → Spanish' },
    { source: 'en', target: 'zh', label: 'English → Chinese' },
    { source: 'en', target: 'ja', label: 'English → Japanese' },
    { source: 'en', target: 'ko', label: 'English → Korean' },
    { source: 'en', target: 'fr', label: 'English → French' },
    { source: 'en', target: 'de', label: 'English → German' },
  ];

  // Get filtered languages based on search
  let filteredLanguages = $derived(() => {
    if (!searchQuery.trim()) return SUPPORTED_LANGUAGES;
    const query = searchQuery.toLowerCase();
    return SUPPORTED_LANGUAGES.filter(lang => {
      const code = lang.code.toLowerCase();
      const name = lang.name.toLowerCase();
      return code.includes(query) || name.includes(query);
    });
  });

  function handleSelect() {
    onSelect(localSource, localTarget);
    close();
  }

  function handleQuickSelect(source: string, target: string) {
    localSource = source;
    localTarget = target;
    onSelect(source, target);
    close();
  }

  function close() {
    searchQuery = '';
    // Reset to original values if cancelled
    localSource = sourceLanguage;
    localTarget = targetLanguage;
    onClose?.();
  }
</script>

<Modal show={show} title="Select Languages" onClose={close}>
  <!-- Common pairs -->
  <div class="section">
    <div class="section-title">Common Pairs</div>
    <div class="pairs-grid">
      {#each commonPairs as pair}
        <button
          class="pair-btn"
          class:active={localSource === pair.source && localTarget === pair.target}
          onclick={() => handleQuickSelect(pair.source, pair.target)}
        >
          <span class="pair-flags">
            <span class="pair-flag">{SUPPORTED_LANGUAGES.find(l => l.code === pair.source)?.flag ?? '🌐'}</span>
            <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="var(--text-dim)" stroke-width="2">
              <path d="M5 12h14M12 5l7 7-7 7"/>
            </svg>
            <span class="pair-flag">{SUPPORTED_LANGUAGES.find(l => l.code === pair.target)?.flag ?? '🌐'}</span>
          </span>
          <span class="pair-label">{pair.label}</span>
        </button>
      {/each}
    </div>
  </div>

  <!-- Custom selection -->
  <div class="section">
    <div class="section-title">Custom Selection ({filteredLanguages().length} languages)</div>
    <div class="custom-selection">
      <div class="lang-column">
        <label class="lang-label">From</label>
        <div class="lang-list">
          {#each filteredLanguages() as lang}
            <button
              class="lang-option"
              class:selected={localSource === lang.code}
              onclick={() => localSource = lang.code}
            >
              <span class="lang-option-flag">{lang.flag}</span>
              <span class="lang-option-name">{lang.name}</span>
              {#if localSource === lang.code}
                <svg class="check-icon" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="var(--accent)" stroke-width="2.5">
                  <polyline points="20 6 9 17 4 12"/>
                </svg>
              {/if}
            </button>
          {/each}
        </div>
      </div>

      <div class="lang-divider">
        <button class="swap-btn" onclick={() => {
          const temp = localSource;
          localSource = localTarget;
          localTarget = temp;
        }} title="Swap languages">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M7 16V4M7 4L3 8M7 4L11 8M17 8V20M17 20L21 16M17 20L13 16"/>
          </svg>
        </button>
      </div>

      <div class="lang-column">
        <label class="lang-label">To</label>
        <div class="lang-list">
          {#each filteredLanguages() as lang}
            <button
              class="lang-option"
              class:selected={localTarget === lang.code}
              onclick={() => localTarget = lang.code}
            >
              <span class="lang-option-flag">{lang.flag}</span>
              <span class="lang-option-name">{lang.name}</span>
              {#if localTarget === lang.code}
                <svg class="check-icon" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="var(--accent)" stroke-width="2.5">
                  <polyline points="20 6 9 17 4 12"/>
                </svg>
              {/if}
            </button>
          {/each}
        </div>
      </div>
    </div>
  </div>

  <div class="selector-footer">
    <button class="btn-apply" onclick={handleSelect}>
      Apply Selection
    </button>
  </div>
</Modal>

<style>
  .section {
    padding: var(--space-md) var(--space-xl);
  }

  .section:not(:last-child) {
    border-bottom: 1px solid var(--border);
  }

  .section-title {
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: var(--space-sm);
  }

  .pairs-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: var(--space-sm);
  }

  .pair-btn {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-xs);
    padding: var(--space-sm);
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .pair-btn:hover {
    background: var(--bg-hover);
    border-color: var(--border-hover);
  }

  .pair-btn.active {
    background: rgba(99, 140, 255, 0.1);
    border-color: var(--accent);
  }

  .pair-flags {
    display: flex;
    align-items: center;
    gap: var(--space-xs);
  }

  .pair-flag {
    font-size: 14px;
  }

  .pair-label {
    font-size: var(--font-size-sm);
    color: var(--text-primary);
  }

  .custom-selection {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    grid-template-rows: 1fr;
    gap: var(--space-md);
    height: 280px;
    overflow: hidden;
  }

  .lang-column {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .lang-label {
    font-size: var(--font-size-xs);
    font-weight: 600;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: var(--space-sm);
    flex-shrink: 0;
  }

  .lang-list {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding-right: var(--space-xs);
    min-height: 0;
  }

  .lang-list::-webkit-scrollbar {
    width: 6px;
  }

  .lang-list::-webkit-scrollbar-track {
    background: transparent;
  }

  .lang-list::-webkit-scrollbar-thumb {
    background: var(--border);
    border-radius: 3px;
  }

  .lang-option {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-sm);
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all 0.2s ease;
    text-align: left;
    position: relative;
  }

  .lang-option:hover {
    background: var(--bg-hover);
  }

  .lang-option.selected {
    background: rgba(99, 140, 255, 0.1);
  }

  .lang-option-flag {
    font-size: 12px;
  }

  .lang-option-name {
    flex: 1;
    font-size: var(--font-size-sm);
    color: var(--text-primary);
  }

  .check-icon {
    margin-left: auto;
  }

  .lang-divider {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .swap-btn {
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text-dim);
    cursor: pointer;
    transition: all 0.2s ease;
    flex-shrink: 0;
  }

  .swap-btn:hover {
    background: var(--bg-hover);
    border-color: var(--border-hover);
    color: var(--text-secondary);
  }

  .selector-footer {
    padding: var(--space-md) var(--space-xl);
    border-top: 1px solid var(--border);
  }

  .btn-apply {
    width: 100%;
    padding: var(--space-sm);
    background: var(--accent);
    border: none;
    border-radius: var(--radius-md);
    color: white;
    font-size: var(--font-size-sm);
    font-weight: 600;
    font-family: var(--font-family);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .btn-apply:hover {
    background: var(--accent-hover);
  }
</style>
